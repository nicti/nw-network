//! Multi-peer DTLS listener.
//!
//! [`MultiPeerListener`] owns one bound UDP socket and runs a
//! demux loop that routes inbound datagrams by source address into
//! per-peer SSL sessions. Each new peer's first ClientHello spawns a
//! per-peer task that drives `SSL_accept` to completion and then
//! reads decrypted plaintext until the connection ends.
//!
//! Compared to [`super::secure_connection::SecureSocketListener`] (which
//! consumes the listener after one accept and is only safe with a
//! single peer), this listener handles many concurrent peers:
//!
//! - Inbound datagrams: a top-level `recv_from` loop matches
//!   `peer_addr` against an `Arc<Mutex<HashMap<...>>>` of known
//!   peers and forwards into that peer's `Sender<Bytes>` queue.
//! - Outbound: each peer's task uses the same shared
//!   `Arc<Async<UdpSocket>>` for `send_to(plaintext)` — UDP allows
//!   concurrent sends from one socket.
//! - Lifecycle: established / data / disconnect events fan into a
//!   single application-facing [`MultiPeerEvent`] channel.
//! - Re-handshakes: a ClientHello from an established peer's address
//!   starts a `Candidate` session beside it, which replaces the
//!   established one only when its handshake completes.
//!
//! The application drives the listener via [`MultiPeerListener::next_event`]
//! and [`MultiPeerListener::send_to`]. Bevy integration (Resource +
//! drain system) lives in the embedding application.

#![cfg(feature = "server")]

use crate::driver::DriverError;
use crate::driver::secure_connection::{
    MemBio, bio_drain, bio_feed, bio_new_mem_pair, build_server_ssl_ctx,
};
use async_channel::{Receiver, Sender, TrySendError, bounded};
use async_io::Async;
use bytes::Bytes;
use dashmap::DashMap;
use foreign_types::ForeignType;
use openssl::ssl::{Ssl, SslContext};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{debug, trace, warn};

const SSL_ERROR_WANT_READ: i32 = 2;
const SSL_ERROR_WANT_WRITE: i32 = 3;

/// Lifecycle events fanned out by the DTLS listener for every peer.
///
/// Internal to gridmate — application code consumes the higher-level
/// [`crate::session_service::Event`] via
/// [`crate::ServerListenerHandle`], which translates this enum and
/// adds carrier-level state on top.
#[derive(Debug, Clone)]
pub(crate) enum MultiPeerEvent {
    /// A peer's `SSL_accept` returned 1 — DTLS handshake complete.
    Established { peer_addr: SocketAddr },
    /// Plaintext bytes from a peer (one DTLS application record per
    /// event, modulo SSL coalescing).
    Data {
        peer_addr: SocketAddr,
        plaintext: Bytes,
    },
    /// Peer's task ended (clean close, SSL error, or task abort).
    Disconnected {
        peer_addr: SocketAddr,
        reason: String,
    },
    /// Listener-level error (e.g. socket recv failure).
    Error { description: String },
}

/// Shared per-peer state — the demux task pushes inbound bytes into
/// `inbound_tx`, and `send_to` pushes outbound plaintext into
/// `outbound_tx`. The peer task selects on both.
struct PeerHandle {
    inbound_tx: Sender<Bytes>,
    outbound_tx: Sender<Bytes>,
    /// Set once `SSL_accept` completes. Also the entry's identity, so
    /// a peer task only removes its own entry, never its successor's.
    established: Arc<AtomicBool>,
}

/// A DTLS record carrying a ClientHello: handshake content type (22),
/// epoch 0, handshake message type 1 (RFC 6347 §4.1, §4.2.2).
fn is_client_hello(datagram: &[u8]) -> bool {
    datagram.len() > 13 && datagram[0] == 22 && datagram[3..5] == [0, 0] && datagram[13] == 1
}

/// The ClientHello's random, after the 13-byte record header, the
/// 12-byte handshake header and the 2-byte client_version. A client
/// keeps it for its retransmits and its cookie-bearing second hello.
fn client_random(datagram: &[u8]) -> Option<&[u8]> {
    datagram.get(27..59)
}

/// Handshake (22) or ChangeCipherSpec (20) record. After its own
/// handshake an established session only sees application data (23)
/// and alerts (21), so these belong to a candidate session. Any epoch:
/// the candidate client's Finished is epoch 1.
fn is_handshake(datagram: &[u8]) -> bool {
    matches!(datagram.first(), Some(20 | 22))
}

/// A handshake that has not finished by then is dropped: a new peer's
/// entry goes away, a candidate is abandoned and the session it would
/// have replaced is untouched.
const HANDSHAKE_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(10)
};

/// A second DTLS session from an established peer's address, handshaking
/// beside it. It replaces the established entry only once its
/// `SSL_accept` completes, so a forged ClientHello cannot end a session.
struct Candidate {
    handle: PeerHandle,
    client_random: Vec<u8>,
}

/// The listener state the demux and the peer tasks share.
#[derive(Clone)]
struct Shared {
    peers: PeerMap,
    candidates: Arc<DashMap<SocketAddr, Candidate>>,
    /// Held while a peer entry leaves the map and its Disconnected (or a
    /// replacement's Disconnected + Established) is sent, so the bridge
    /// never sees a replacement's Established before the old peer's end.
    lifecycle: Arc<async_lock::Mutex<()>>,
}

/// Lock-free peer table. Inbound demux looks up the handle on every
/// datagram (high-frequency hot path) and `send_to` looks it up on
/// every outbound write; at MMO scale a `Mutex<HashMap>` would
/// serialise both. `DashMap` shards per-bucket so demux + per-peer
/// SSL_write scale with cores.
type PeerMap = Arc<DashMap<SocketAddr, PeerHandle>>;

/// DTLS listener that supports many concurrent peers over one UDP
/// socket via software demux.
///
/// Constructed via [`MultiPeerListener::bind`]; consumed by the
/// caller via [`MultiPeerListener::next_event`] (lifecycle events)
/// and [`MultiPeerListener::send_to`] (outbound plaintext).
///
/// The demux task is detached on the embedder-registered spawner
/// (see [`crate::spawn::set_spawner`]). Lifecycle is gated by
/// `_shutdown_tx`: when the listener drops, the channel closes and
/// the demux loop observes the close on its select branch and
/// exits.
pub struct MultiPeerListener {
    socket: Arc<Async<UdpSocket>>,
    peers: PeerMap,
    event_rx: Receiver<MultiPeerEvent>,
    /// Held only for its Drop side-effect: closing the channel is the
    /// demux task's exit signal. See [`crate::spawn::ShutdownSignal`].
    _shutdown: crate::spawn::ShutdownSignal,
}

impl MultiPeerListener {
    /// Bind the UDP socket, build the server SSL context, and start
    /// the demux task. Returns once the listener is ready to accept
    /// peers; the actual accept work happens in spawned tasks.
    pub async fn bind(addr: &str, cert_pem: &str, key_pem: &str) -> Result<Self, DriverError> {
        let bind_addr: SocketAddr = addr
            .parse()
            .map_err(|e| DriverError::Address(format!("Invalid address: {e}")))?;
        let std_socket = UdpSocket::bind(bind_addr)?;
        // Windows-only: disable `WSAECONNRESET` propagation. Without
        // this, the OS reflects every ICMP "port unreachable" we
        // receive (because we sent a keepalive to a peer that has
        // closed its socket) into the *next* `recv_from`, which then
        // returns `ConnectionReset`. async-io surfaces that as a
        // hard error and the demux loop exits — but because the
        // error is silently consumed by the polling layer in some
        // codepaths, the symptom users see is "the listener stops
        // accepting new peers after the first one disconnects".
        disable_udp_connreset(&std_socket);
        let socket = Arc::new(Async::new(std_socket)?);
        let ssl_ctx = Arc::new(build_server_ssl_ctx(cert_pem, key_pem)?);

        let peers: PeerMap = Arc::new(DashMap::new());
        // DTLS-level lifecycle events; aggregates across all peers so
        // size for burst tolerance, not per-peer scale.
        const LISTENER_EVENT_CAPACITY: usize = 4096;
        let (event_tx, event_rx) = bounded::<MultiPeerEvent>(LISTENER_EVENT_CAPACITY);
        let (shutdown, shutdown_rx) = crate::spawn::ShutdownSignal::new();

        crate::spawn::spawn_detached(demux_loop(
            socket.clone(),
            ssl_ctx,
            Shared {
                peers: peers.clone(),
                candidates: Arc::new(DashMap::new()),
                lifecycle: Arc::new(async_lock::Mutex::new(())),
            },
            event_tx,
            shutdown_rx,
        ));

        Ok(Self {
            socket,
            peers,
            event_rx,
            _shutdown: shutdown,
        })
    }

    /// Local address the demux socket is bound to. Useful when
    /// binding to port 0 to discover the OS-chosen port.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.get_ref().local_addr()
    }

    /// Next lifecycle event. Returns `None` when the demux task has
    /// stopped (listener dropped). Cancel-safe. Internal —
    /// The server event bridge is the only consumer.
    pub(crate) async fn next_event(&self) -> Option<MultiPeerEvent> {
        self.event_rx.recv().await.ok()
    }

    /// Send plaintext to an established peer. The bytes go into the
    /// peer's task, which `SSL_write`s and flushes to the shared
    /// socket. Returns `Err` if the peer is unknown or its task
    /// has ended.
    ///
    /// Lock-free peer lookup; the outbound channel push may still
    /// `.await` if the per-peer SSL_write queue is full (backpressure).
    pub async fn send_to(
        &self,
        peer_addr: SocketAddr,
        plaintext: Bytes,
    ) -> Result<(), DriverError> {
        // Clone the sender out of the dashmap entry so we don't hold
        // the shard guard across the `.await`.
        let outbound_tx = {
            let Some(handle) = self.peers.get(&peer_addr) else {
                return Err(DriverError::Ssl(format!("unknown peer {peer_addr}")));
            };
            handle.outbound_tx.clone()
        };
        outbound_tx
            .send(plaintext)
            .await
            .map_err(|e| DriverError::Ssl(format!("peer {peer_addr} closed: {e}")))
    }

    /// Sync, non-blocking variant. Returns `Err` if the peer is
    /// unknown or its outbound queue is full. Used by the carrier
    /// driver task to push without spawning a forwarder.
    pub fn try_send_to(&self, peer_addr: SocketAddr, plaintext: Bytes) -> Result<(), DriverError> {
        let Some(handle) = self.peers.get(&peer_addr) else {
            return Err(DriverError::Ssl(format!("unknown peer {peer_addr}")));
        };
        handle.outbound_tx.try_send(plaintext).map_err(|e| match e {
            TrySendError::Full(_) => {
                DriverError::Ssl(format!("peer {peer_addr} outbound queue full"))
            }
            TrySendError::Closed(_) => {
                DriverError::Ssl(format!("peer {peer_addr} outbound queue closed"))
            }
        })
    }
}

/// Top-level demux: read every incoming datagram, route by
/// `peer_addr`. New peers get a fresh per-peer task spawned on the
/// embedder-selected executor.
///
/// `shutdown_rx` is the listener's exit signal — when the listener
/// drops, its `_shutdown_tx` closes the channel and the select
/// branch wakes, returning the demux from its blocking
/// `recv_from`.
/// Windows: clear `SIO_UDP_CONNRESET` on the bound UDP socket so that
/// an outbound datagram to a closed peer port doesn't poison the
/// next `recv_from` with `WSAECONNRESET`. The default-on behaviour
/// is a Windows-only convenience for clients (so they learn the
/// server is gone); for a *server* socket it's a footgun — a peer's
/// disappearance silently breaks reception for every other peer.
///
/// Best-effort: if the ioctl fails we log and continue. Real-world
/// failure modes are exotic (the kernel always supports this ioctl
/// on UDP sockets since Windows 2000), but the listener still
/// functions if the call returns an error — it just becomes
/// vulnerable to the original bug.
fn disable_udp_connreset(_socket: &UdpSocket) {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawSocket;
        use windows_sys::Win32::Networking::WinSock::{SIO_UDP_CONNRESET, SOCKET, WSAIoctl};

        let raw = _socket.as_raw_socket() as SOCKET;
        let mut disable: u32 = 0;
        let mut returned: u32 = 0;
        let rc = unsafe {
            WSAIoctl(
                raw,
                SIO_UDP_CONNRESET,
                &mut disable as *mut _ as *mut _,
                std::mem::size_of_val(&disable) as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
                None,
            )
        };
        if rc != 0 {
            warn!(
                "disable_udp_connreset: WSAIoctl(SIO_UDP_CONNRESET, FALSE) returned {rc}; \
                 demux may silently stall after first peer disconnect"
            );
        } else {
            debug!("disable_udp_connreset: SIO_UDP_CONNRESET cleared on listener socket");
        }
    }
}

async fn demux_loop(
    socket: Arc<Async<UdpSocket>>,
    ssl_ctx: Arc<SslContext>,
    shared: Shared,
    event_tx: Sender<MultiPeerEvent>,
    shutdown_rx: Receiver<()>,
) {
    // Refcounted recv ring: `recv_from` writes directly into the
    // ring's uninit tail and `commit` splits the just-received
    // datagram off as an `Arc`-shared `Bytes` — no per-datagram
    // allocation, no `copy_from_slice` on the inbound hot path.
    // 4 MB ring × 64 KB max datagram ≈ 64 datagrams between
    // amortised rollover allocations.
    const RECV_BUF_SIZE: usize = 65536;
    const RING_CHUNK_SIZE: usize = 64 * RECV_BUF_SIZE;
    let mut ring = super::recv_ring::RecvRing::new(RING_CHUNK_SIZE, RECV_BUF_SIZE);
    loop {
        enum DemuxStep {
            Datagram(std::io::Result<(usize, SocketAddr)>),
            Shutdown,
        }
        let (len, peer_addr) = {
            let slot = ring.recv_slot();
            let step = futures_lite::future::or(
                async { DemuxStep::Datagram(socket.recv_from(slot).await) },
                async {
                    let _ = shutdown_rx.recv().await;
                    DemuxStep::Shutdown
                },
            )
            .await;
            match step {
                DemuxStep::Datagram(Ok(v)) => v,
                DemuxStep::Datagram(Err(err)) => {
                    let _ = event_tx
                        .send(MultiPeerEvent::Error {
                            description: format!("recv_from: {err}"),
                        })
                        .await;
                    return;
                }
                DemuxStep::Shutdown => {
                    debug!("MultiPeerListener: demux shutdown signal observed; exiting");
                    return;
                }
            }
        };
        let datagram = ring.commit(len);

        // Lock-free clones of the inbound senders. Drop the dashmap
        // guards before `await` so we never hold a shard lock across
        // suspension.
        let hello = is_client_hello(&datagram);
        let random = client_random(&datagram).unwrap_or_default().to_vec();
        let peer = shared
            .peers
            .get(&peer_addr)
            .map(|h| (h.inbound_tx.clone(), h.established.load(Ordering::Acquire)));
        let candidate = shared
            .candidates
            .get(&peer_addr)
            .map(|c| (c.handle.inbound_tx.clone(), c.client_random == random));

        let established = matches!(peer, Some((_, true)));
        if established && hello && !matches!(candidate, Some((_, true))) {
            // A ClientHello on a finished handshake: a restarted client
            // on the same address:port, or a forged datagram. The old
            // SSL session would drop it, so handshake it in a candidate
            // session beside the established one, which it replaces
            // only if the handshake completes. A hello with another
            // random replaces an unfinished candidate.
            debug!(
                ?peer_addr,
                replacing_candidate = candidate.is_some(),
                "demux: ClientHello on an established peer; starting a candidate DTLS session"
            );
            spawn_peer(
                peer_addr,
                datagram,
                &ssl_ctx,
                &socket,
                &event_tx,
                &shared,
                Some(random),
            );
            continue;
        }
        let inbound_tx = match (peer, candidate) {
            // Handshake records go to the candidate, everything else to
            // the established session.
            (Some((tx, established)), Some((candidate_tx, _))) => {
                if established && is_handshake(&datagram) {
                    candidate_tx
                } else {
                    tx
                }
            }
            (Some((tx, _)), None) => tx,
            // The established session ended while its candidate handshakes.
            (None, Some((candidate_tx, _))) => candidate_tx,
            (None, None) => {
                debug!(?peer_addr, "demux: new peer — spawning peer_task");
                spawn_peer(
                    peer_addr, datagram, &ssl_ctx, &socket, &event_tx, &shared, None,
                );
                continue;
            }
        };
        // A failed send means that task has ended; it removes its own entry.
        trace!(?peer_addr, len, "demux: datagram to existing peer");
        if inbound_tx.send(datagram).await.is_err() {
            trace!(?peer_addr, "demux: peer_task gone");
        }
    }
}

/// Registers a peer task for `peer_addr`, seeded with its ClientHello:
/// as the address's peer, or as its candidate when `candidate_random`
/// is set. A candidate replaces any unfinished one, whose task then
/// sees its channels close.
fn spawn_peer(
    peer_addr: SocketAddr,
    client_hello: Bytes,
    ssl_ctx: &Arc<SslContext>,
    socket: &Arc<Async<UdpSocket>>,
    event_tx: &Sender<MultiPeerEvent>,
    shared: &Shared,
    candidate_random: Option<Vec<u8>>,
) {
    // Bounded — slow consumer triggers `.send().await` backpressure
    // rather than unbounded memory growth.
    const PER_PEER_DTLS_QUEUE: usize = 256;
    let (inbound_tx, inbound_rx) = bounded::<Bytes>(PER_PEER_DTLS_QUEUE);
    let (outbound_tx, outbound_rx) = bounded::<Bytes>(PER_PEER_DTLS_QUEUE);
    // Cannot fail: the queue is new and empty.
    let _ = inbound_tx.try_send(client_hello);
    let established = Arc::new(AtomicBool::new(false));
    let handle = PeerHandle {
        inbound_tx,
        outbound_tx,
        established: established.clone(),
    };
    let is_candidate = candidate_random.is_some();
    match candidate_random {
        Some(client_random) => {
            shared.candidates.insert(
                peer_addr,
                Candidate {
                    handle,
                    client_random,
                },
            );
        }
        None => {
            shared.peers.insert(peer_addr, handle);
        }
    }
    crate::spawn::spawn_detached(peer_task(
        peer_addr,
        ssl_ctx.clone(),
        socket.clone(),
        inbound_rx,
        outbound_rx,
        event_tx.clone(),
        shared.clone(),
        established,
        is_candidate,
    ));
}

/// Per-peer SSL session driver. Handles SSL_accept, then loops on
/// (inbound encrypted bytes → SSL_read → plaintext event) and
/// (outbound plaintext request → SSL_write → encrypted on the wire).
#[allow(clippy::too_many_arguments)]
async fn peer_task(
    peer_addr: SocketAddr,
    ssl_ctx: Arc<SslContext>,
    socket: Arc<Async<UdpSocket>>,
    inbound_rx: Receiver<Bytes>,
    outbound_rx: Receiver<Bytes>,
    event_tx: Sender<MultiPeerEvent>,
    shared: Shared,
    established: Arc<AtomicBool>,
    is_candidate: bool,
) {
    let result = run_peer(
        peer_addr,
        &ssl_ctx,
        &socket,
        &inbound_rx,
        &outbound_rx,
        &event_tx,
        &shared,
        &established,
        is_candidate,
    )
    .await;
    debug!(
        ?peer_addr,
        is_candidate,
        ?result,
        "MultiPeerListener: peer task ended"
    );

    // Entries are ours only while they hold our `established` flag.
    let own = |h: &PeerHandle| Arc::ptr_eq(&h.established, &established);
    // A candidate that never completed ends silently: the session it
    // would have replaced is untouched.
    if shared
        .candidates
        .remove_if(&peer_addr, |_, c| own(&c.handle))
        .is_some()
    {
        return;
    }
    // Clean up the peer entry so a reconnecting client gets a fresh SSL
    // session -- but only our own: if a candidate replaced it, that
    // candidate has reported our end and the entry is its.
    let _lifecycle = shared.lifecycle.lock().await;
    if shared.peers.remove_if(&peer_addr, |_, h| own(h)).is_none() {
        return;
    }

    match result {
        Ok(()) => {
            let _ = event_tx
                .send(MultiPeerEvent::Disconnected {
                    peer_addr,
                    reason: "peer task ended".into(),
                })
                .await;
        }
        Err(reason) => {
            let _ = event_tx
                .send(MultiPeerEvent::Disconnected { peer_addr, reason })
                .await;
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_peer(
    peer_addr: SocketAddr,
    ssl_ctx: &SslContext,
    socket: &Async<UdpSocket>,
    inbound_rx: &Receiver<Bytes>,
    outbound_rx: &Receiver<Bytes>,
    event_tx: &Sender<MultiPeerEvent>,
    shared: &Shared,
    established: &Arc<AtomicBool>,
    is_candidate: bool,
) -> Result<(), String> {
    let mut ssl = Ssl::new(ssl_ctx).map_err(|e| format!("Ssl::new: {e}"))?;
    ssl.set_mtu(1200).ok();
    let (read_bio, write_bio) = bio_new_mem_pair().map_err(|e| format!("bio_new: {e}"))?;
    unsafe {
        openssl_sys::SSL_set_bio(ssl.as_ptr(), read_bio.as_ptr(), write_bio.as_ptr());
    }
    ssl.set_accept_state();

    // Drive SSL_accept until it returns 1, eating from inbound_rx
    // and flushing write_bio after each call.
    let final_flight = futures_lite::future::or(
        accept_handshake(&ssl, read_bio, write_bio, peer_addr, socket, inbound_rx),
        async {
            futures_timer::Delay::new(HANDSHAKE_TIMEOUT).await;
            Err(format!(
                "DTLS handshake not complete after {HANDSHAKE_TIMEOUT:?}"
            ))
        },
    )
    .await?;
    debug!("MultiPeerListener: SSL_accept complete for {peer_addr}");

    // Take the address before our last flight goes out, so the client's
    // first application data, which follows it, reaches this session.
    if is_candidate {
        let _lifecycle = shared.lifecycle.lock().await;
        let own = |c: &Candidate| Arc::ptr_eq(&c.handle.established, established);
        let Some((_, candidate)) = shared.candidates.remove_if(&peer_addr, |_, c| own(c)) else {
            // A newer ClientHello superseded this candidate.
            return Ok(());
        };
        established.store(true, Ordering::Release);
        if shared.peers.insert(peer_addr, candidate.handle).is_some() {
            debug!(
                ?peer_addr,
                "MultiPeerListener: candidate DTLS session replaces the established one"
            );
            let _ = event_tx
                .send(MultiPeerEvent::Disconnected {
                    peer_addr,
                    reason: "replaced by a new DTLS session from the same address".into(),
                })
                .await;
        }
        let _ = event_tx
            .send(MultiPeerEvent::Established { peer_addr })
            .await;
    } else {
        established.store(true, Ordering::Release);
        let _ = event_tx
            .send(MultiPeerEvent::Established { peer_addr })
            .await;
    }
    if !final_flight.is_empty() {
        socket
            .send_to(final_flight.as_ref(), peer_addr)
            .await
            .map_err(|e| format!("send_to: {e}"))?;
    }

    // Steady-state loop: select on inbound encrypted bytes and
    // outbound plaintext requests. `or` resolves whichever future
    // wakes first; both branches normalise to the same `Event`
    // enum so the combinator's `Output` type matches.
    //
    // `recv_ring` is the per-peer SSL_read sink: `SSL_read` writes
    // decrypted plaintext directly into the ring's tail and we hand
    // out an `Arc`-shared `Bytes` to the bridge via
    // `MultiPeerEvent::Data` — no `copy_from_slice` per record.
    //
    // Sized small (4 × max_record = 256 KB) so a 5–10 k-peer server
    // fits the total recv-ring footprint in 1–2.5 GB. The ring also
    // lazy-allocates on first recv, so idle peers cost zero.
    const SSL_RECV_BUF: usize = 65536;
    const SSL_RING_CHUNK: usize = 4 * SSL_RECV_BUF;
    let mut recv_ring = super::recv_ring::RecvRing::new(SSL_RING_CHUNK, SSL_RECV_BUF);
    loop {
        let event =
            futures_lite::future::or(async { Event::Inbound(inbound_rx.recv().await) }, async {
                Event::Outbound(outbound_rx.recv().await)
            })
            .await;

        match event {
            Event::Inbound(Ok(datagram)) => {
                bio_feed(read_bio, &datagram).map_err(|e| format!("bio_feed: {e}"))?;
                drain_decrypted(&ssl, &mut recv_ring, peer_addr, event_tx).await?;
            }
            Event::Inbound(Err(_)) => return Ok(()),
            Event::Outbound(Ok(plaintext)) => {
                let n = unsafe {
                    openssl_sys::SSL_write(
                        ssl.as_ptr(),
                        plaintext.as_ptr() as *const _,
                        plaintext.len() as i32,
                    )
                };
                if n <= 0 {
                    let err = unsafe { openssl_sys::SSL_get_error(ssl.as_ptr(), n) };
                    return Err(format!("SSL_write {err}"));
                }
                let outgoing = bio_drain(write_bio);
                if !outgoing.is_empty() {
                    socket
                        .send_to(outgoing.as_ref(), peer_addr)
                        .await
                        .map_err(|e| format!("send_to: {e}"))?;
                }
            }
            Event::Outbound(Err(_)) => return Ok(()),
        }
    }
}

enum Event {
    Inbound(Result<Bytes, async_channel::RecvError>),
    Outbound(Result<Bytes, async_channel::RecvError>),
}

async fn accept_handshake(
    ssl: &Ssl,
    read_bio: MemBio,
    write_bio: MemBio,
    peer_addr: SocketAddr,
    socket: &Async<UdpSocket>,
    inbound_rx: &Receiver<Bytes>,
) -> Result<Bytes, String> {
    loop {
        let ret = unsafe { openssl_sys::SSL_accept(ssl.as_ptr()) };

        let outgoing = bio_drain(write_bio);
        if ret == 1 {
            // The caller sends the last flight once the session is registered.
            return Ok(outgoing);
        }
        if !outgoing.is_empty() {
            socket
                .send_to(outgoing.as_ref(), peer_addr)
                .await
                .map_err(|e| format!("send_to: {e}"))?;
        }

        let err = unsafe { openssl_sys::SSL_get_error(ssl.as_ptr(), ret) };
        if err != SSL_ERROR_WANT_READ && err != SSL_ERROR_WANT_WRITE {
            // Drain the entire ERR queue (each SSL_accept failure can
            // queue several reason codes — the *first* one is usually
            // the actual handshake alert, the rest are propagation
            // wrappers from OpenSSL's call stack). The numeric pair
            // alone (`error 1 (0xa00041b)`) was opaque; passing the
            // queue through the safe `openssl::error::ErrorStack`
            // wrapper yields strings like `error:0A00041B:SSL routines:
            // :tlsv1 alert decrypt error` so we can tell a launcher
            // cert-pin rejection from a cipher-mismatch from a
            // record-layer version mismatch.
            let stack = openssl::error::ErrorStack::get();
            let joined = if stack.errors().is_empty() {
                "<empty ERR queue>".to_string()
            } else {
                stack
                    .errors()
                    .iter()
                    .map(|e| format!("{e}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            return Err(format!("SSL_accept error {err}: {joined}"));
        }

        let datagram = match inbound_rx.recv().await {
            Ok(d) => d,
            Err(_) => return Err("inbound channel closed during handshake".into()),
        };
        bio_feed(read_bio, &datagram).map_err(|e| format!("bio_feed: {e}"))?;
    }
}

async fn drain_decrypted(
    ssl: &Ssl,
    recv_ring: &mut super::recv_ring::RecvRing,
    peer_addr: SocketAddr,
    event_tx: &Sender<MultiPeerEvent>,
) -> Result<(), String> {
    loop {
        let n;
        let plaintext = {
            let slot = recv_ring.recv_slot();
            n = unsafe {
                openssl_sys::SSL_read(ssl.as_ptr(), slot.as_mut_ptr() as *mut _, slot.len() as i32)
            };
            if n <= 0 {
                // SSL_read returned 0 or an error code; do not
                // commit. Fall through to the error-classification
                // block below.
                Bytes::new()
            } else {
                recv_ring.commit(n as usize)
            }
        };
        if n > 0 {
            trace!(?peer_addr, len = n, "MultiPeerListener: decrypted record");
            if event_tx
                .send(MultiPeerEvent::Data {
                    peer_addr,
                    plaintext,
                })
                .await
                .is_err()
            {
                return Ok(()); // listener dropped
            }
            continue;
        }
        let err = unsafe { openssl_sys::SSL_get_error(ssl.as_ptr(), n) };
        if err == SSL_ERROR_WANT_READ || err == SSL_ERROR_WANT_WRITE {
            return Ok(());
        }
        let code = unsafe { openssl_sys::ERR_get_error() };
        warn!(?peer_addr, err, code, "MultiPeerListener: SSL_read fatal");
        return Err(format!("SSL_read {err} (0x{code:x})"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ssl::{SslConnector, SslMethod, SslStream, SslVerifyMode};
    use std::io::{Read, Write};

    struct ThreadSpawner;
    impl crate::spawn::Spawner for ThreadSpawner {
        fn spawn(&self, future: crate::spawn::BoxedFuture) {
            std::thread::spawn(move || async_io::block_on(future));
        }
    }

    /// A connected UDP socket as a `Read + Write` stream: one datagram
    /// per call, which is what DTLS needs.
    #[derive(Debug)]
    struct Udp(UdpSocket);
    impl Read for Udp {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.recv(buf)
        }
    }
    impl Write for Udp {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.send(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A DTLS client handshake over `socket`, already bound, to `server`.
    fn connect(socket: UdpSocket, server: SocketAddr) -> Result<SslStream<Udp>, String> {
        socket
            .connect(server)
            .map_err(|e| format!("connect: {e}"))?;
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let mut builder = SslConnector::builder(SslMethod::dtls()).map_err(|e| e.to_string())?;
        builder.set_verify(SslVerifyMode::NONE);
        builder
            .set_cipher_list("ECDHE-RSA-AES256-GCM-SHA384")
            .map_err(|e| e.to_string())?;
        builder
            .build()
            .configure()
            .map_err(|e| e.to_string())?
            .connect("localhost", Udp(socket))
            .map_err(|e| format!("handshake: {e}"))
    }

    /// Handshakes from `local` to `server` and sends `hello`. The client
    /// socket is then dropped without a close_notify, as a client that
    /// exits or crashes leaves it.
    fn client_session(local: SocketAddr, server: SocketAddr, hello: &[u8]) -> Result<(), String> {
        let socket = UdpSocket::bind(local).map_err(|e| format!("bind: {e}"))?;
        let mut stream = connect(socket, server)?;
        stream.write_all(hello).map_err(|e| e.to_string())?;
        // Close the socket without the SSL layer sending close_notify.
        let Udp(socket) = std::mem::replace(
            stream.get_mut(),
            Udp(UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?),
        );
        drop(socket);
        std::mem::forget(stream);
        Ok(())
    }

    /// A real client's first ClientHello datagram, taken from a
    /// handshake to a socket that never answers.
    fn client_hello() -> Vec<u8> {
        let sink = UdpSocket::bind("127.0.0.1:0").unwrap();
        let to = sink.local_addr().unwrap();
        std::thread::spawn(move || {
            let _ = connect(UdpSocket::bind("127.0.0.1:0").unwrap(), to);
        });
        let mut buf = [0u8; 2048];
        let n = sink.recv(&mut buf).unwrap();
        buf[..n].to_vec()
    }

    fn listen() -> (MultiPeerListener, SocketAddr) {
        let _ = crate::spawn::set_spawner(Arc::new(ThreadSpawner));
        let (cert, key) = crate::driver::test_cert::generate_self_signed_cert("localhost").unwrap();
        let listener =
            async_io::block_on(MultiPeerListener::bind("127.0.0.1:0", &cert, &key)).unwrap();
        let server = listener.local_addr().unwrap();
        (listener, server)
    }

    /// Every listener event within `window`, as text.
    fn events_for(listener: &MultiPeerListener, window: Duration) -> Vec<String> {
        async_io::block_on(async {
            let deadline = std::time::Instant::now() + window;
            let mut out = Vec::new();
            loop {
                let event = futures_lite::future::or(listener.next_event(), async {
                    async_io::Timer::at(deadline).await;
                    None
                })
                .await;
                out.push(match event {
                    None => return out,
                    Some(MultiPeerEvent::Established { .. }) => "established".to_string(),
                    Some(MultiPeerEvent::Data { plaintext, .. }) => {
                        format!("data {}", String::from_utf8_lossy(&plaintext))
                    }
                    Some(MultiPeerEvent::Disconnected { reason, .. }) => {
                        format!("disconnected: {reason}")
                    }
                    Some(MultiPeerEvent::Error { description }) => format!("error: {description}"),
                });
            }
        })
    }

    const SETTLE: Duration = Duration::from_millis(300);

    async fn next_data(listener: &MultiPeerListener) -> (SocketAddr, Bytes) {
        loop {
            match listener.next_event().await {
                Some(MultiPeerEvent::Data {
                    peer_addr,
                    plaintext,
                }) => return (peer_addr, plaintext),
                Some(_) => continue,
                None => panic!("listener ended"),
            }
        }
    }

    #[test]
    fn a_restarted_client_on_the_same_port_handshakes_again() {
        let (listener, server) = listen();
        // One client port for both sessions, as a restarted client
        // tries the same first port again.
        let local = UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();

        client_session(local, server, b"first").expect("first session");
        let (from, data) = async_io::block_on(next_data(&listener));
        assert_eq!((from, data.as_ref()), (local, &b"first"[..]));

        // The first client is gone and nothing above the listener ended
        // its peer: nothing in the embedding server does.
        client_session(local, server, b"second").expect("second session from the same port");
        let (from, data) = async_io::block_on(next_data(&listener));
        assert_eq!((from, data.as_ref()), (local, &b"second"[..]));
    }

    #[test]
    fn a_client_hello_that_never_completes_leaves_the_established_session_alone() {
        let (listener, server) = listen();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let local = socket.local_addr().unwrap();
        let raw = socket.try_clone().unwrap();
        let mut client = connect(socket, server).expect("handshake");
        client.write_all(b"before").unwrap();
        assert_eq!(
            events_for(&listener, SETTLE),
            ["established", "data before"]
        );

        // From the established peer's own address:port, as a forged
        // datagram would be, and no handshake follows it.
        let hello = client_hello();
        assert!(is_client_hello(&hello));
        raw.send(&hello).unwrap();
        // Past the candidate's handshake timeout (1 s in tests).
        let during = events_for(&listener, Duration::from_millis(1500));

        client.write_all(b"after").unwrap();
        assert_eq!(during, Vec::<String>::new());
        assert_eq!(events_for(&listener, SETTLE), ["data after"]);
        async_io::block_on(listener.send_to(local, Bytes::from_static(b"reply"))).unwrap();
        let mut buf = [0u8; 64];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"reply");
    }

    #[test]
    fn a_completed_candidate_replaces_the_established_session_once() {
        let (listener, server) = listen();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let same_port = socket.try_clone().unwrap();
        let mut old = connect(socket, server).expect("first handshake");
        old.write_all(b"old").unwrap();
        assert_eq!(events_for(&listener, SETTLE), ["established", "data old"]);

        // A second client on the same address:port while the first
        // session is still open.
        let mut new = connect(same_port, server).expect("second handshake");
        new.write_all(b"new").unwrap();
        assert_eq!(
            events_for(&listener, SETTLE),
            [
                "disconnected: replaced by a new DTLS session from the same address",
                "established",
                "data new",
            ]
        );
    }
}
