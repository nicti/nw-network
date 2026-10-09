//! Transport abstraction for [`super::connection_state::ConnectionState`].
//!
//! Lumberyard's GridMate `Carrier` uses one `Driver` for all
//! connections — the driver is socket-shaped and the carrier just
//! `read`s and `write`s bytes. Our port collapses driver +
//! connection into [`SecureConnection`] for the initiator
//! (one DTLS socket per outbound peer) and routes through a shared
//! [`crate::driver::MultiPeerListener`] for the responder (multi-peer demux on one
//! shared socket).
//!
//! [`CarrierTransport`] is the seam. Both impls live alongside this
//! module; [`ConnectionState`] is generic over `T: CarrierTransport`
//! so it never matches on the transport flavor — each call to
//! `read`/`write`/`try_recv` is monomorphised against the concrete
//! transport, no `dyn` dispatch and no per-call match arms.

use crate::driver::error::DriverError;
use crate::driver::{Established, SecureConnection};
#[cfg(feature = "server")]
use async_channel::{Receiver, Sender, TryRecvError};
use bytes::Bytes;
use std::future::Future;
use std::net::SocketAddr;

/// Per-peer byte transport feeding one [`super::connection_state::ConnectionState`].
///
/// Two impls ship with gridmate: [`DtlsTransport`] for the initiator
/// (dedicated DTLS connection) and [`ChannelTransport`] for the
/// responder (per-peer queue + shared
/// [`crate::driver::MultiPeerListener`]). Embedders building on
/// custom transports implement this trait directly.
///
/// The async methods return `Send + '_` futures so the carrier
/// driver task (which is spawned on the embedder's `Spawner`)
/// satisfies `Send` end-to-end — without this bound the trait's
/// auto-trait inference can leave the driver future `!Send` and
/// `spawn_detached` rejects it.
pub trait CarrierTransport: Send + 'static {
    /// Address of the peer this transport is bound to. Used for
    /// diagnostics.
    fn peer_addr(&self) -> SocketAddr;

    /// Async read of the next plaintext datagram. Mirrors
    /// [`SecureConnection::read`].
    fn read(&mut self) -> impl Future<Output = Result<Bytes, DriverError>> + Send + '_;

    /// Non-blocking variant — `Ok(None)` if no data is queued. Used
    /// by the carrier's batch-drain loop after an async read fires,
    /// to pick up additional packets the OS already buffered.
    fn try_recv(&mut self) -> Result<Option<Bytes>, DriverError>;

    /// Push one carrier datagram out. Takes `Bytes` so callers that
    /// already have one (the carrier driver's `prepare_outgoing_datagram`
    /// returns `Bytes`) avoid a `copy_from_slice` round-trip — the
    /// inner `Arc` is incremented and the channel/socket gets the
    /// same backing buffer.
    fn write(
        &mut self,
        data: Bytes,
    ) -> impl Future<Output = Result<usize, DriverError>> + Send + '_;
}

/// Initiator-side transport: a dedicated DTLS connection on its own
/// UDP socket used by client connections.
pub struct DtlsTransport(pub SecureConnection<Established>);

impl CarrierTransport for DtlsTransport {
    fn peer_addr(&self) -> SocketAddr {
        self.0.peer_addr()
    }

    async fn read(&mut self) -> Result<Bytes, DriverError> {
        self.0.read().await
    }

    fn try_recv(&mut self) -> Result<Option<Bytes>, DriverError> {
        self.0.try_recv_decrypt()
    }

    async fn write(&mut self, data: Bytes) -> Result<usize, DriverError> {
        // `SecureConnection::write` takes `&[u8]` (it copies into the
        // OpenSSL `SSL_write` buffer anyway). The `Bytes` `Deref`s to
        // `&[u8]` without allocating.
        self.0.write(&data).await
    }
}

/// Responder-side transport: plaintext arrives via `inbound` (fed by
/// [`crate::driver::MultiPeerListener`]'s demux loop) and outbound carrier datagrams
/// go straight into the per-peer SSL_write queue of the one DTLS
/// session this carrier was made for.
///
/// That queue, not the peer's address: a client restarted on the same
/// address:port replaces the DTLS session, and an address lookup would
/// then hand this carrier's last writes (its resends, the SM_DISCONNECT
/// its drop sends) to the new client, which fails its connect (#439).
/// Once its session is replaced or closed, a write still goes out under
/// the OLD session's keys until that session's task drops the queue (the
/// new client discards those as bad MAC); after that, writes fail.
///
/// A closed `inbound` means the session is gone (the server registry
/// held its only sender), so `read` reports a non-retryable error: the
/// driver stops instead of spinning, and a carrier still in `ready()`
/// gets its `Error`.
#[cfg(feature = "server")]
pub struct ChannelTransport {
    pub peer_addr: SocketAddr,
    pub inbound: Receiver<Bytes>,
    pub outbound: Sender<Bytes>,
}

#[cfg(feature = "server")]
impl CarrierTransport for ChannelTransport {
    fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    async fn read(&mut self) -> Result<Bytes, DriverError> {
        self.inbound.recv().await.map_err(|_| self.session_closed())
    }

    fn try_recv(&mut self) -> Result<Option<Bytes>, DriverError> {
        match self.inbound.try_recv() {
            Ok(bytes) => Ok(Some(bytes)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Closed) => Err(self.session_closed()),
        }
    }

    async fn write(&mut self, data: Bytes) -> Result<usize, DriverError> {
        let len = data.len();
        self.outbound
            .send(data)
            .await
            .map_err(|_| self.session_closed())?;
        Ok(len)
    }
}

#[cfg(feature = "server")]
impl ChannelTransport {
    /// Not `ConnectionClosed`: that one is retryable.
    fn session_closed(&self) -> DriverError {
        DriverError::Ssl(format!("peer {} DTLS session closed", self.peer_addr))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    /// A closed inbound must stop the driver, not read as retryable:
    /// a retryable error would leave it spinning.
    #[test]
    fn a_closed_inbound_is_not_retryable() {
        let (inbound_tx, inbound) = async_channel::bounded::<Bytes>(1);
        let (outbound, _outbound_rx) = async_channel::bounded::<Bytes>(1);
        let mut io = ChannelTransport {
            peer_addr: "127.0.0.1:1".parse().unwrap(),
            inbound,
            outbound,
        };
        drop(inbound_tx);
        assert!(!io.try_recv().unwrap_err().is_retryable());
        assert!(!async_io::block_on(io.read()).unwrap_err().is_retryable());
    }
}
