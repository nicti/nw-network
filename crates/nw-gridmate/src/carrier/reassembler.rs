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

    /// Stage one frame on its channel. Frames on channels
    /// `>= MAX_CHANNELS` are dropped (the production receive path does
    /// the same — out-of-range channels are malformed).
    ///
    /// Staging is kept in reliable-id order, not arrival order, since
    /// the gate only ever looks at the head: a reliable frame that
    /// arrives after its successors (a lost frame's resend, or plain
    /// UDP reordering) must go in front of them, or the channel waits
    /// on it for ever. A reliable frame whose id is already staged or
    /// already delivered is a repeat and is dropped. An unreliable frame
    /// goes right after the reliable frame it names as its predecessor,
    /// behind any unreliable frame already there, which is the order it
    /// was sent in.
    pub fn accept(&mut self, frame: MessageData) {
        let ch = frame.channel as usize;
        if ch >= MAX_CHANNELS {
            return;
        }
        let reliable = frame.reliability == DataReliability::Reliable;
        let repeat = reliable
            && (self.order_key(ch, &frame).is_none()
                || self.staging[ch].iter().any(|m| {
                    m.reliability == DataReliability::Reliable
                        && m.send_reliable_seq_num == frame.send_reliable_seq_num
                }));
        if repeat {
            debug!(
                "[REASSEMBLE] channel={} discarded repeated reliable id {} ({} bytes)",
                ch,
                frame.send_reliable_seq_num.get(),
                frame.data.len()
            );
            return;
        }
        let key = self.order_key(ch, &frame);
        // ponytail: linear insert; staging holds a handful of frames.
        let at = self.staging[ch]
            .iter()
            .position(|m| self.order_key(ch, m) > key)
            .unwrap_or(self.staging[ch].len());
        self.staging[ch].insert(at, frame);
    }

    /// Where a frame sorts in its channel's staging queue: how far its
    /// reliable id (its own, or the predecessor an unreliable frame
    /// names) is ahead of the last delivered one, then reliable before
    /// unreliable. An unreliable frame free to go now sorts first.
    /// `None` for a reliable frame that has already been delivered.
    fn order_key(&self, ch: usize, m: &MessageData) -> Option<(u16, bool)> {
        let last_rel = self.received_reliable_seq_num[ch];
        let delivered_any = self.reliable_delivered[ch];
        let id = m.send_reliable_seq_num;
        let ahead = !delivered_any || seq_less_than(last_rel, id);
        // From the sentinel on a fresh channel, so that keys stay
        // consistent once the first reliable frame is delivered.
        let dist = sequence_number_sequential_distance(last_rel, id);
        match (m.reliability == DataReliability::Reliable, ahead) {
            (true, true) => Some((dist, false)),
            (true, false) => None,
            (false, true) => Some((dist, true)),
            (false, false) => Some((0, true)),
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

    /// Deliver reliable ids 0 to `last` on channel 1, one at a time, as
    /// a live link would: staging more than half the id space at once
    /// has no order the wrapping comparison can give it.
    fn deliver_up_to(r: &mut Reassembler, last: u16) {
        for id in 0..=last {
            r.accept(frame(1, true, 1, id, b"x"));
            assert_eq!(delivered(r).len(), 1, "id {id}");
        }
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
    /// arrived waits, as it always did, and is not discarded: when the
    /// predecessor arrives *after* it, both deliver, in id order.
    #[test]
    fn a_reliable_gap_still_waits_for_what_is_missing() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"a"));
        r.accept(frame(1, true, 3, 2, b"c"));
        assert_eq!(delivered(&mut r), vec![b"a".to_vec()], "c waits for b");
        assert!(delivered(&mut r).is_empty(), "and keeps waiting");

        r.accept(frame(1, true, 2, 1, b"b"));
        assert_eq!(delivered(&mut r), vec![b"b".to_vec(), b"c".to_vec()]);
    }

    /// A lost reliable frame is resent by the peer
    /// after later frames have already arrived. Arrival-ordered staging
    /// queued the resend behind the frames waiting for it and the
    /// channel never moved again. An unreliable frame sent between r5
    /// and r6 delivers between them, wherever it arrived.
    #[test]
    fn a_lost_reliable_frame_resent_behind_later_ones_delivers_in_order() {
        let mut r = Reassembler::new();
        for id in 0..=4 {
            r.accept(frame(1, true, id, id, b"x"));
        }
        assert_eq!(delivered(&mut r).len(), 5);

        r.accept(frame(1, true, 6, 6, b"r6"));
        r.accept(frame(1, false, 7, 5, b"after r5"));
        r.accept(frame(1, true, 8, 7, b"r7"));
        assert!(delivered(&mut r).is_empty(), "all wait for r5");

        r.accept(frame(1, true, 9, 5, b"r5"));
        assert_eq!(
            delivered(&mut r),
            vec![
                b"r5".to_vec(),
                b"after r5".to_vec(),
                b"r6".to_vec(),
                b"r7".to_vec()
            ]
        );
    }

    /// A chunk run whose middle chunk is resent: once staged twice
    /// (a copy arriving while the original is still staged), and once
    /// lost and resent behind the chunk after it. Either way
    /// `chunks_ready` used to see a broken run for good.
    #[test]
    fn a_resent_chunk_inside_a_run_does_not_break_the_run() {
        let mut r = Reassembler::new();
        r.accept(chunk(1, 1, 0, 3, b"he"));
        r.accept(chunk(1, 2, 1, 1, b"l"));
        r.accept(chunk(1, 3, 1, 1, b"l"));
        r.accept(chunk(1, 4, 2, 1, b"lo"));
        assert_eq!(delivered(&mut r), vec![b"hello".to_vec()]);

        r.accept(chunk(1, 5, 3, 3, b"wo"));
        r.accept(chunk(1, 7, 5, 1, b"ld"));
        r.accept(chunk(1, 8, 4, 1, b"r"));
        assert_eq!(delivered(&mut r), vec![b"world".to_vec()]);
    }

    /// Ordering reads the wrap the way the ids wrap: 0xFFFE, 0xFFFF,
    /// 0, 1 arriving backwards still deliver forwards.
    #[test]
    fn a_reorder_across_the_wrap_delivers_in_id_order() {
        let mut r = Reassembler::new();
        deliver_up_to(&mut r, 0xfffd);

        for (id, body) in [(1, b"1"), (0, b"0"), (0xffff, b"f"), (0xfffe, b"e")] {
            r.accept(frame(1, true, 2, id, body));
            if id != 0xfffe {
                assert!(delivered(&mut r).is_empty());
            }
        }
        assert_eq!(
            delivered(&mut r),
            vec![b"e".to_vec(), b"f".to_vec(), b"0".to_vec(), b"1".to_vec()]
        );
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
    /// everything". It is released once the predecessor arrives, even
    /// though that arrives after it.
    #[test]
    fn an_unreliable_frame_waits_for_a_reliable_predecessor_that_is_missing() {
        let mut r = Reassembler::new();
        r.accept(frame(1, true, 1, 0, b"r0"));
        r.accept(frame(1, false, 2, 1, b"after r1"));
        assert_eq!(delivered(&mut r), vec![b"r0".to_vec()]);
        assert!(delivered(&mut r).is_empty(), "and it keeps waiting");

        r.accept(frame(1, true, 3, 1, b"r1"));
        assert_eq!(
            delivered(&mut r),
            vec![b"r1".to_vec(), b"after r1".to_vec()]
        );
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
        deliver_up_to(&mut r, u16::MAX);

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
        deliver_up_to(&mut r, u16::MAX);

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
