//! Per-channel chunk reassembly + reliable-message ordering.
//!
//! Lives downstream of [`super::decoder::decode_datagram`] and upstream
//! of the application-level envelope parse (`SessionService::dispatch_messages`).
//! The contract is GridMate's `Carrier.cpp::ProcessReceivedMessages`
//! ordering — reliable messages must deliver in sequence on their
//! channel; unreliable messages on the same channel wait until the
//! latest reliable ID has been delivered.
//!
//! Used by both:
//! - the production peer state machine
//!   ([`super::connection_state::ConnectionState`]), which holds one
//!   `Reassembler` privately and layers its dedup + ACK + retransmit
//!   logic around it;
//! - external decode consumers ([`super::receiver::Receiver`], `cap`,
//!   capture-extractor tools), which want the same correctness
//!   guarantees without standing up a full peer connection.

use bytes::Bytes;
use std::collections::VecDeque;
use tracing::debug;

use super::datagram_history::{seq_less_than, sequence_number_sequential_distance};
#[cfg(debug_assertions)]
use super::message::MessageWireSpans;
use super::message::{DataReliability, MessageData};
use super::types::{MAX_CHANNELS, SEQUENCE_NUMBER_MAX, SequenceNumber};

/// Per-channel chunk reassembly + reliable-ordering for inbound
/// carrier frames. Push frames in arrival order via [`Self::accept`];
/// drain reassembled messages in delivery order via [`Self::drain`].
///
/// The struct owns these parallel arrays indexed by channel:
///
/// - `staging` — frames not yet delivered (waiting for a reliable
///   predecessor, or for chunks to complete a multi-chunk message).
/// - `received_reliable_seq_num` — the latest reliable sequence id
///   delivered on this channel. Initialised to
///   [`SEQUENCE_NUMBER_MAX`], so the first reliable id is 0.
/// - `reliable_delivered` — whether that id is real yet. Until it
///   is, unreliable frames flow regardless of their predecessor id
///   and no reliable frame is read as a repeat.
/// - `received_seq_num` — the latest sequence id (reliable or
///   unreliable) delivered on this channel. Stats-only today; the
///   ordering gate uses the reliable counter.
pub struct Reassembler {
    /// Per-channel staging queue: frames received but not yet ready
    /// to deliver to the application.
    staging: [VecDeque<MessageData>; MAX_CHANNELS],
    /// Last delivered reliable message sequence number, per channel.
    /// Drives the ordering gate in [`Self::drain_channel`].
    received_reliable_seq_num: [SequenceNumber; MAX_CHANNELS],
    /// Last delivered sequence number, per channel (reliable or not).
    /// Not used by the ordering gate; tracked to match GridMate's
    /// `m_receivedSeqNum` for stats / diagnostics parity.
    received_seq_num: [SequenceNumber; MAX_CHANNELS],
    /// Whether this channel has delivered any reliable message. The
    /// "nothing delivered yet" state lives here and not in
    /// `received_reliable_seq_num == SEQUENCE_NUMBER_MAX`, because
    /// 0xFFFF is also a real id: after delivering it, a resend of it
    /// would otherwise read as the first frame of a fresh channel.
    reliable_delivered: [bool; MAX_CHANNELS],
}

impl Default for Reassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl Reassembler {
    /// Fresh reassembler. All channels start with
    /// `received_reliable_seq_num = SequenceNumberMax` to match
    /// GridMate's "nothing delivered yet" sentinel.
    pub fn new() -> Self {
        Self {
            staging: Default::default(),
            received_reliable_seq_num: [SEQUENCE_NUMBER_MAX; MAX_CHANNELS],
            received_seq_num: [SEQUENCE_NUMBER_MAX; MAX_CHANNELS],
            reliable_delivered: [false; MAX_CHANNELS],
        }
    }

    /// Push one frame into the staging queue for its channel. Frames
    /// on channels `>= MAX_CHANNELS` are dropped (the production
    /// receive path does the same — out-of-range channels are
    /// malformed).
    pub fn accept(&mut self, frame: MessageData) {
        let ch = frame.channel as usize;
        if ch < MAX_CHANNELS {
            self.staging[ch].push_back(frame);
        }
    }

    /// Drain every channel's ready messages in delivery order.
    /// Yields `(channel, message)` tuples; the iterator borrows
    /// `&mut self` so callers must collect or step through it before
    /// dropping the borrow.
    pub fn drain(&mut self) -> Drain<'_> {
        Drain {
            reassembler: self,
            channel: 0,
        }
    }

    /// Snapshot of the latest reliable sequence id delivered on
    /// `channel`. Mirrors `ConnectionState::received_reliable_seq_num`
    /// for telemetry / test harnesses that want to assert delivery
    /// order without poking private fields.
    pub fn received_reliable_seq_num(&self, channel: u8) -> SequenceNumber {
        self.received_reliable_seq_num[channel as usize]
    }

    /// Pop the next ready message on `channel`, applying the
    /// ordering gate and multi-chunk reassembly. Returns `None` when
    /// the channel has no deliverable message right now (either
    /// empty, blocked on a reliable predecessor, or waiting for more
    /// chunks).
    fn next_ready(&mut self, channel: usize) -> Option<MessageData> {
        loop {
            // Peek the head and decide whether it's deliverable.
            let head = self.staging[channel].front()?;
            let head_reliable = head.reliability == DataReliability::Reliable;
            let head_send_rel = head.send_reliable_seq_num;
            let head_num_chunks = head.num_chunks.get() as usize;

            let last_rel = self.received_reliable_seq_num[channel];
            let nothing_delivered_yet = !self.reliable_delivered[channel];
            // Whether the head's reliable id (its own, or the predecessor
            // an unreliable frame names) is still ahead of what this
            // channel has delivered. Wrapping-aware, because both ids wrap
            // at 0xFFFF: a plain `distance > 0` reads an id that is *behind*
            // as one 65,000 ahead.
            let ahead = nothing_delivered_yet || seq_less_than(last_rel, head_send_rel);

            if head_reliable {
                // A reliable id this channel has already delivered: a peer
                // retransmission. The datagram-level dedup upstream cannot
                // catch it, because a retransmission travels in a *new*
                // datagram with its own sequence number. Discard it.
                //
                // Returning `None` here -- which is what this did before --
                // parks every later message on the channel behind a frame
                // that can never be delivered, for the rest of the session.
                // The corpus shows real clients retransmitting.
                if !ahead {
                    let dropped = self.staging[channel].pop_front()?;
                    debug!(
                        "[REASSEMBLE] channel={} discarded already-delivered reliable id {} \
                         ({} bytes)",
                        channel,
                        dropped.send_reliable_seq_num.get(),
                        dropped.data.len()
                    );
                    continue;
                }
                // Reliable: must be the next sequential reliable id, and
                // for multi-chunk messages every chunk must already be
                // queued with sequential reliable ids behind it.
                let dist = sequence_number_sequential_distance(last_rel, head_send_rel);
                if dist != 1 {
                    return None;
                }
                if head_num_chunks > 1 {
                    if self.staging[channel].len() < head_num_chunks {
                        return None;
                    }
                    if !self.chunks_ready(channel, head_num_chunks) {
                        return None;
                    }
                    return self.assemble_chunks(channel, head_num_chunks);
                }
            } else {
                // Unreliable: wait until the latest reliable predecessor
                // has been delivered. The special-case `nothing_delivered_yet`
                // unblocks the first message on a freshly-opened channel
                // (handles SM_CONNECT_ACK and friends). A predecessor that
                // is *older* than the last delivered id has been delivered
                // long since, so the frame goes out rather than waiting.
                if ahead && !nothing_delivered_yet {
                    return None;
                }
            }

            // Single-frame delivery: pop and update the trailing seq ids.
            let msg = self.staging[channel].pop_front()?;
            if msg.reliability == DataReliability::Reliable {
                self.received_reliable_seq_num[channel] = msg.send_reliable_seq_num;
                self.reliable_delivered[channel] = true;
            }
            self.received_seq_num[channel] = msg.sequence_number;
            return Some(msg);
        }
    }

    /// Pre-flight check: the next `num_chunks` frames in `staging` are
    /// all reliable and carry sequential reliable ids. Mirrors
    /// `ConnectionState::verify_chunks_ready`.
    fn chunks_ready(&self, channel: usize, num_chunks: usize) -> bool {
        if self.staging[channel].len() < num_chunks {
            return false;
        }
        let mut iter = self.staging[channel].iter();
        let Some(first) = iter.next() else {
            return false;
        };
        if first.reliability != DataReliability::Reliable {
            return false;
        }
        let mut prev_rel_seq = first.send_reliable_seq_num;
        for chunk in iter.take(num_chunks - 1) {
            if chunk.reliability != DataReliability::Reliable {
                return false;
            }
            let dist =
                sequence_number_sequential_distance(prev_rel_seq, chunk.send_reliable_seq_num);
            if dist != 1 {
                return false;
            }
            prev_rel_seq = chunk.send_reliable_seq_num;
        }
        true
    }

    /// Pop `num_chunks` frames from `staging[channel]` and concatenate
    /// their `data` into one reassembled [`MessageData`]. Updates the
    /// channel's trailing seq counters from the *last* chunk so the
    /// gate moves forward.
    fn assemble_chunks(&mut self, channel: usize, num_chunks: usize) -> Option<MessageData> {
        let total_size: usize = self.staging[channel]
            .iter()
            .take(num_chunks)
            .map(|m| m.data.len())
            .sum();
        debug!(
            "[REASSEMBLE] channel={} chunks={} total={}",
            channel, num_chunks, total_size
        );
        let mut data = Vec::with_capacity(total_size);
        #[cfg(debug_assertions)]
        let mut wire_spans = MessageWireSpans::new();
        let mut last_seq = SequenceNumber::ZERO;
        let mut last_rel = SequenceNumber::ZERO;
        for _ in 0..num_chunks {
            let chunk = self.staging[channel].pop_front()?;
            data.extend_from_slice(&chunk.data);
            #[cfg(debug_assertions)]
            wire_spans.extend(chunk.wire_spans);
            last_seq = chunk.sequence_number;
            last_rel = chunk.send_reliable_seq_num;
        }
        self.received_reliable_seq_num[channel] = last_rel;
        self.reliable_delivered[channel] = true;
        self.received_seq_num[channel] = last_seq;

        let mut msg = MessageData::new();
        msg.channel = channel as u8;
        msg.reliability = DataReliability::Reliable;
        msg.num_chunks = SequenceNumber::from(1);
        msg.sequence_number = last_seq;
        msg.send_reliable_seq_num = last_rel;
        msg.data = Bytes::from(data);
        #[cfg(debug_assertions)]
        {
            msg.wire_spans = wire_spans;
        }
        Some(msg)
    }
}

/// Iterator returned by [`Reassembler::drain`]. Walks channels in
/// numerical order and yields each channel's ready messages until
/// none can be delivered, then advances to the next channel.
pub struct Drain<'a> {
    reassembler: &'a mut Reassembler,
    channel: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame. `rel` is the frame's own reliable id when it is
    /// reliable, and the id of its latest reliable predecessor when it
    /// is not -- which is how the wire carries it.
    fn frame(channel: u8, reliable: bool, seq: u16, rel: u16, body: &[u8]) -> MessageData {
        let mut msg = MessageData::new();
        msg.channel = channel;
        msg.reliability = match reliable {
            true => DataReliability::Reliable,
            false => DataReliability::Unreliable,
        };
        msg.num_chunks = SequenceNumber::from(1);
        msg.sequence_number = SequenceNumber::from(seq);
        msg.send_reliable_seq_num = SequenceNumber::from(rel);
        msg.data = Bytes::copy_from_slice(body);
        msg
    }

    fn chunk(channel: u8, seq: u16, rel: u16, chunks: u16, body: &[u8]) -> MessageData {
        let mut msg = frame(channel, true, seq, rel, body);
        msg.num_chunks = SequenceNumber::from(chunks);
        msg
    }

    fn delivered(r: &mut Reassembler) -> Vec<Vec<u8>> {
        r.drain().map(|(_, m)| m.data.to_vec()).collect()
    }

    /// The channel-death bug: a peer that retransmits a reliable
    /// frame does it in a *new* datagram, so the datagram-level dedup
    /// upstream passes it through. Before the fix the repeat parked the
    /// channel for the rest of the session.
    #[test]
    fn a_retransmitted_reliable_frame_is_dropped_and_the_channel_keeps_delivering() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"a"));
        assert_eq!(delivered(&mut r), vec![b"a".to_vec()]);

        r.accept(frame(1, true, 2, 0, b"a"));
        r.accept(frame(1, true, 3, 1, b"b"));
        assert_eq!(
            delivered(&mut r),
            vec![b"b".to_vec()],
            "the repeat is discarded and what follows it still arrives"
        );
    }

    /// The control for the test above: the fix must not turn a *gap*
    /// into a discard. A reliable frame whose predecessor has not
    /// arrived waits, as it always did, and is not counted as a repeat.
    ///
    /// It stops there deliberately. Staging is arrival-ordered and only
    /// its head is examined, so a predecessor that arrives *later* is
    /// queued behind the frame waiting for it and neither ever moves.
    /// That is a second way this channel dies, beyond the retransmission
    /// this patch fixes; a later commit fixes it.
    #[test]
    fn a_reliable_gap_still_waits_for_what_is_missing() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"a"));
        r.accept(frame(1, true, 3, 2, b"c"));
        assert_eq!(delivered(&mut r), vec![b"a".to_vec()], "c waits for b");
        assert!(delivered(&mut r).is_empty(), "and keeps waiting");
        assert_eq!(r.staging[1].len(), 1, "a gap is not a repeat: c is kept");
    }

    /// An unreliable frame names the reliable id it was sent behind. A
    /// reordered one can name an *older* id than the channel has since
    /// delivered; the wrapping distance from the newer id to the older
    /// one is huge, which the old `distance > 0` test read as "still
    /// ahead" and waited on for ever.
    #[test]
    fn an_unreliable_frame_behind_the_last_reliable_id_still_delivers() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"r0"));
        r.accept(frame(1, true, 2, 1, b"r1"));
        assert_eq!(delivered(&mut r), vec![b"r0".to_vec(), b"r1".to_vec()]);

        r.accept(frame(1, false, 3, 0, b"late input"));
        r.accept(frame(1, false, 4, 1, b"input"));
        assert_eq!(
            delivered(&mut r),
            vec![b"late input".to_vec(), b"input".to_vec()]
        );
    }

    /// An unreliable frame whose reliable predecessor has *not* been
    /// delivered still waits: the control for the test above, so that
    /// "deliver what is behind us" cannot quietly become "deliver
    /// everything".
    ///
    /// It only asserts the waiting. Whether the frame is ever released
    /// once its predecessor arrives is a separate question this gate
    /// does not answer: `Drain` looks at the head of the channel's
    /// queue alone, so a predecessor that arrives *after* it is queued
    /// behind it. A later commit fixes that.
    #[test]
    fn an_unreliable_frame_waits_for_a_reliable_predecessor_that_is_missing() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"r0"));
        r.accept(frame(1, false, 2, 1, b"after r1"));
        assert_eq!(delivered(&mut r), vec![b"r0".to_vec()]);
        assert!(delivered(&mut r).is_empty(), "and it keeps waiting");
        assert_eq!(r.staging[1].len(), 1, "and it is kept");
    }

    /// A retransmitted first chunk of a multi-chunk message blocks the
    /// same way, and is discarded the same way.
    #[test]
    fn a_retransmitted_chunk_run_does_not_block_the_channel() {
        let mut r = Reassembler::new();
        r.accept(chunk(1, 1, 0, 2, b"he"));
        r.accept(chunk(1, 2, 1, 1, b"llo"));
        assert_eq!(delivered(&mut r), vec![b"hello".to_vec()]);

        r.accept(chunk(1, 3, 0, 2, b"he"));
        r.accept(chunk(1, 4, 1, 1, b"llo"));
        r.accept(frame(1, true, 5, 2, b"next"));
        assert_eq!(delivered(&mut r), vec![b"next".to_vec()]);
    }

    /// A fresh channel has delivered nothing, and its
    /// `received_reliable_seq_num` starts at 0xFFFF only so that id 0 is
    /// "the next one". The first reliable frame, id 0, delivers, and so
    /// do the ids after it -- none is read as a repeat of the sentinel.
    #[test]
    fn a_fresh_channel_delivers_from_id_zero() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"first"));
        r.accept(frame(1, true, 2, 1, b"second"));
        assert_eq!(
            delivered(&mut r),
            vec![b"first".to_vec(), b"second".to_vec()]
        );
    }

    /// 0xFFFF is the sentinel's value and also a real id. Once a channel
    /// has delivered 0xFFFF, a resend of it must still read as a repeat,
    /// not as the first frame of a fresh channel, which would park it at
    /// the head waiting for id 0 behind it.
    #[test]
    fn a_resend_of_id_ffff_after_delivering_it_is_a_repeat() {
        let mut r = Reassembler::new();
        for id in 0..=u16::MAX {
            r.accept(frame(1, true, 1, id, b"x"));
        }
        assert_eq!(delivered(&mut r).len(), 65_536);

        r.accept(frame(1, true, 2, 0xffff, b"resent"));
        r.accept(frame(1, true, 3, 0, b"past the wrap"));
        assert_eq!(delivered(&mut r), vec![b"past the wrap".to_vec()]);
    }

    /// Ids wrap at 0xFFFF. Past the wrap a new id still delivers, and a
    /// repeat of one just below the wrap is still a repeat rather than
    /// a 65,000-frame gap -- which is what a non-wrapping comparison
    /// would make of it.
    #[test]
    fn the_gate_reads_the_sequence_wrap_the_way_the_ids_wrap() {
        let mut r = Reassembler::new();
        // The gate's sentinel means a channel can only start at id 0,
        // so the wrap has to be walked up to.
        for id in 0..=u16::MAX {
            r.accept(frame(1, true, 1, id, b"x"));
        }
        assert_eq!(delivered(&mut r).len(), 65_536);

        r.accept(frame(1, true, 2, 0, b"past the wrap"));
        assert_eq!(delivered(&mut r), vec![b"past the wrap".to_vec()]);

        r.accept(frame(1, true, 3, 0xffff, b"a repeat from before it"));
        r.accept(frame(1, true, 4, 1, b"next"));
        assert_eq!(delivered(&mut r), vec![b"next".to_vec()]);
    }
}

impl<'a> Iterator for Drain<'a> {
    type Item = (u8, MessageData);

    fn next(&mut self) -> Option<Self::Item> {
        while self.channel < MAX_CHANNELS {
            if let Some(msg) = self.reassembler.next_ready(self.channel) {
                return Some((self.channel as u8, msg));
            }
            self.channel += 1;
        }
        None
    }
}
