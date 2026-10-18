//! The client's input message (`REPClient` type index 1589), both ways.
//!
//! [`ClientInput::decode`] reads a body and keeps every field, including the
//! ones a simulation never looks at, so that [`ClientInput::encode`] writes
//! the same bytes back. The layout follows the client's reader: an
//! `ActorRequestId`, a per-message header, then a count and that many
//! entries. Each entry is a delta against the one before it: a 3-byte
//! little-endian mask says which optional fields follow, and two values
//! (`flags` and `tail_count`) are carried from entry to entry because they
//! decide which later fields exist and which mask bit each one uses.
//!
//! Fields whose meaning is not known have positional names (`f00`, `w3`),
//! or the client's structure offsets (the header). The reader refuses what
//! no known client has been seen to send (a non-empty first or third tail
//! list, the flags bit-14 field, a contact value below its lower bound, a
//! bool that is neither 0 nor 1); such a body is an error rather than a
//! guess.
//!
//! Encoding derives each entry's mask from which `Option` fields are `Some`.
//! Mask bits no field uses are kept in [`Entry::unread_mask`] so a decoded
//! body re-encodes byte for byte.

use crate::serialize::{
    CARRIER_ENDIAN, Marshal, MarshalerError, ReadBuffer, Unmarshal, VlqU32Marshaler,
    VlqU64Marshaler, WriteBuffer,
};

/// The message's type index.
pub const TYPE_INDEX: u32 = 1589;

/// Most pairs the header's list may hold.
pub const MAX_HEADER_PAIRS: u32 = 16;

/// Longest string, in bytes, the client's string reader accepts.
pub const MAX_STRING: u32 = 0x2ffff;

/// Why a body did not decode.
#[derive(Debug, thiserror::Error)]
pub enum ClientInputError {
    /// The bytes do not parse.
    #[error("{0}")]
    Marshal(#[from] MarshalerError),
    /// A bool that is neither 0 nor 1.
    #[error("a bool of {0}")]
    Bool(u8),
    /// A list or string longer than the client allows.
    #[error("{what} of {len}, above {max}")]
    TooLong {
        what: &'static str,
        len: u32,
        max: u32,
    },
    /// A field no known client sends, refused rather than guessed.
    #[error("{0}")]
    Unseen(String),
    /// Bytes after the last entry.
    #[error("{0} bytes left over")]
    Trailing(usize),
}

/// A world position: `f32` x, `f32` y, and z as a `u16` over [-100, 1000],
/// kept raw so it re-encodes exactly.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z_raw: u16,
}

impl Position {
    const Z_MIN: f32 = -100.0;
    const Z_MAX: f32 = 1000.0;

    /// The height the raw `u16` stands for.
    #[must_use]
    pub fn z(&self) -> f32 {
        let v = Self::Z_MIN
            + (f32::from(self.z_raw) / f32::from(u16::MAX)) * (Self::Z_MAX - Self::Z_MIN);
        v.clamp(Self::Z_MIN, Self::Z_MAX)
    }

    /// The raw `u16` nearest to a height (clamped to the range).
    #[must_use]
    pub fn z_raw_from(z: f32) -> u16 {
        let n = ((z - Self::Z_MIN) / (Self::Z_MAX - Self::Z_MIN)).clamp(0.0, 1.0);
        (n * f32::from(u16::MAX)).round() as u16
    }

    /// A position from a height in metres.
    #[must_use]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z_raw: Self::z_raw_from(z),
        }
    }
}

/// The 16-byte `ActorRequestId`: two big-endian `u64`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActorRequestId(pub u64, pub u64);

/// The per-message header (the client's structure offsets name the fields).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub b9: u8,
    pub b10: u8,
    pub b8: u8,
    pub b11: u8,
    pub v10: u64,
    pub v1c8: u32,
    pub v18: u32,
    pub b1d: u8,
    /// A bool, then a VLQ64 when it is set.
    pub opt1d0: Option<u64>,
    pub u64_1d8: u64,
    pub b1c: u8,
    pub v20: u32,
    pub v22: u32,
    pub v24: u32,
    pub b1e: u8,
    /// `list28`: at most 16 pairs of VLQ64.
    pub list28: Vec<(u64, u64)>,
    pub v1b0: u64,
    pub v1b8: u64,
    pub v1c0: u32,
    pub v1c4: u32,
}

/// The target block: kind 3 stands for "none" on the wire and is `None` in
/// the entry; any other kind carries these fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Target {
    /// Never 3.
    pub kind: u8,
    pub b: u8,
    pub position: Position,
    pub shorts: [u16; 3],
}

/// The interaction block behind mask bit 8 (`FUN_1435ebc30`), positional
/// names in wire order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Interaction {
    pub id16: [u8; 16],
    pub q0: u64,
    pub q1: u64,
    pub b0: u8,
    pub b1: u8,
    pub q2: u64,
    pub d0: u32,
    pub s0: Vec<u8>,
    pub d1: u32,
    pub d2: u32,
    pub d3: u32,
    pub flag0: bool,
    pub s1: Vec<u8>,
    pub s2: Vec<u8>,
    pub s3: Vec<u8>,
    pub s4: Vec<u8>,
    pub d4: u32,
    /// A bool, then six `u32`s when it is set.
    pub six: Option<[u32; 6]>,
    pub flag1: bool,
    pub d5: u32,
    pub b2: u8,
    pub d6: [u32; 5],
    pub v0: u64,
    pub d7: u32,
    pub v1: u32,
    pub s5: Vec<u8>,
    pub d8: u32,
    pub b3: u8,
    pub flag2: bool,
    pub d9: u32,
    pub s6: Vec<u8>,
    pub d10: u32,
    pub q3: u64,
    pub d11: u32,
    pub flag3: bool,
}

/// One entry of the second tail list (`FUN_142a1a590`), 192 bytes' worth of
/// positional fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Contact {
    /// Four 3-float vectors.
    pub vectors: [[f32; 3]; 4],
    pub id16: [u8; 16],
    pub d0: u32,
    /// Three entity references: 16 bytes and a `u64` each.
    pub refs: [([u8; 16], u64); 3],
    /// The second reference's extra `u64`.
    pub ref1_extra: u64,
    pub q0: u64,
    pub q1: u64,
    /// Lower bounds when read: -1, 0, 0.
    pub values: [f32; 3],
    pub d1: u32,
    pub d2: u32,
    pub d3: u32,
    pub flag0: bool,
    pub flag1: bool,
    pub b0: u8,
    pub flag2: bool,
}

/// The conditional tail of an entry. Which fields exist depends on the
/// carried `flags` and `tail_count`; the mask bit of each is assigned in
/// order, so a field absent by flags frees its bit for the next one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tail {
    /// Three VLQ32 when flags bit 0 is set.
    pub flag0_values: Option<[u32; 3]>,
    /// Flags bit 11, a mask bit.
    pub position_flag11: Option<Position>,
    /// Flags bit 8, a mask bit.
    pub position_flag8: Option<Position>,
    /// Flags bit 8, the next mask bit: 16 bytes and a `u64`.
    pub ref_flag8: Option<([u8; 16], u64)>,
    /// Sets the carried `tail_count`.
    pub tail_count: Option<u8>,
    /// With a non-zero `tail_count`.
    pub position_tail: Option<Position>,
    /// With a non-zero `tail_count`: 16 bytes and two `u64`s.
    pub ref_tail: Option<([u8; 16], u64, u64)>,
    /// The second list. The first and third must be empty on the wire.
    pub contacts: Vec<Contact>,
    /// Flags bit 15.
    pub position_flag15: Option<Position>,
    /// Three raw half floats.
    pub halves: Option<[u16; 3]>,
    /// Flags bit 16.
    pub word_flag16: Option<u64>,
    /// Flags bit 18.
    pub byte_flag18: Option<u8>,
    pub byte_a: Option<u8>,
    pub word_b: Option<u32>,
    /// A raw half float, with flags bit 0.
    pub half_c: Option<u16>,
}

/// One input.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    pub counter: u16,
    /// Mask bit 0. Carried to later entries.
    pub flags: Option<u64>,
    /// Mask bit 1.
    pub b1: Option<u8>,
    /// Mask bit 2.
    pub bool2: Option<bool>,
    /// Mask bit 3.
    pub b3: Option<u8>,
    /// Mask bit 4: a raw `u16` over [0, 10].
    pub q4: Option<u16>,
    /// The action bitset, 32 bits a word.
    pub actions: Vec<u32>,
    /// Four axes as the raw bytes: `-1 + b / 128`.
    pub axes: [u8; 4],
    /// Mask bit 5.
    pub dt: Option<f32>,
    /// Mask bit 6: two floats.
    pub angles: Option<(f32, f32)>,
    /// Mask bit 7. Not carried: absent means none.
    pub camera: Option<Position>,
    /// `None` is kind 3.
    pub target: Option<Target>,
    /// Mask bit 8 then a bool: outer `Some(None)` is the bit with the bool
    /// false.
    pub interaction: Option<Option<Box<Interaction>>>,
    pub tail: Tail,
    /// Mask bits no field uses, kept so a body re-encodes exactly.
    pub unread_mask: u32,
}

impl Entry {
    /// An axis as a float in [-1, 1).
    #[must_use]
    pub fn axis(&self, i: usize) -> f32 {
        -1.0 + f32::from(self.axes[i]) / 128.0
    }
}

/// A whole 1589 body.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClientInput {
    pub request_id: ActorRequestId,
    pub header: Header,
    /// In wire order (each is a delta against the one before it).
    pub entries: Vec<Entry>,
}

/// What entry-to-entry carries.
#[derive(Clone, Copy, Default)]
struct Carry {
    flags: u64,
    tail_count: u8,
}

struct Mask {
    value: u32,
    consumed: u32,
}

impl Mask {
    fn take(&mut self, n: u32) -> bool {
        self.consumed |= 1 << n;
        self.value >> n & 1 == 1
    }
}

type R<T> = Result<T, ClientInputError>;

fn u8(rb: &mut ReadBuffer) -> R<u8> {
    Ok(rb.read_u8()?)
}
fn u16(rb: &mut ReadBuffer) -> R<u16> {
    Ok(rb.read_u16()?)
}
fn u32(rb: &mut ReadBuffer) -> R<u32> {
    Ok(rb.read_u32()?)
}
fn u64(rb: &mut ReadBuffer) -> R<u64> {
    Ok(u64::unmarshal(rb)?)
}
fn f32(rb: &mut ReadBuffer) -> R<f32> {
    Ok(f32::unmarshal(rb)?)
}
fn vlq32(rb: &mut ReadBuffer) -> R<u32> {
    Ok(VlqU32Marshaler.unmarshal(rb)?)
}
fn vlq64(rb: &mut ReadBuffer) -> R<u64> {
    Ok(VlqU64Marshaler.unmarshal(rb)?)
}
fn boolean(rb: &mut ReadBuffer) -> R<bool> {
    match u8(rb)? {
        0 => Ok(false),
        1 => Ok(true),
        v => Err(ClientInputError::Bool(v)),
    }
}
fn arr16(rb: &mut ReadBuffer) -> R<[u8; 16]> {
    Ok(*rb.read_array::<16>()?)
}
fn string(rb: &mut ReadBuffer) -> R<Vec<u8>> {
    let n = vlq32(rb)?;
    if n > MAX_STRING {
        return Err(ClientInputError::TooLong {
            what: "a string",
            len: n,
            max: MAX_STRING,
        });
    }
    Ok(rb.read_bytes(n as usize)?.to_vec())
}
fn position(rb: &mut ReadBuffer) -> R<Position> {
    Ok(Position {
        x: f32(rb)?,
        y: f32(rb)?,
        z_raw: u16(rb)?,
    })
}

fn w_vlq32(wb: &mut WriteBuffer, v: u32) {
    VlqU32Marshaler.marshal(wb, v);
}
fn w_vlq64(wb: &mut WriteBuffer, v: u64) {
    VlqU64Marshaler.marshal(wb, v);
}
fn w_bool(wb: &mut WriteBuffer, v: bool) {
    wb.write_u8(u8::from(v));
}
fn w_string(wb: &mut WriteBuffer, s: &[u8]) {
    w_vlq32(wb, s.len() as u32);
    wb.write_bytes(s);
}
fn w_position(wb: &mut WriteBuffer, p: &Position) {
    p.x.marshal(wb);
    p.y.marshal(wb);
    wb.write_u16(p.z_raw);
}

impl ClientInput {
    /// Reads a body.
    ///
    /// # Errors
    ///
    /// On a truncated body, a refused field (see the module docs) or bytes
    /// left over.
    pub fn decode(body: &[u8]) -> R<Self> {
        let mut r = ReadBuffer::new(CARRIER_ENDIAN, body);
        let rb = &mut r;
        let request_id = ActorRequestId(u64(rb)?, u64(rb)?);
        let header = decode_header(rb)?;
        let count = u8(rb)?;
        let mut entries = Vec::with_capacity(count.into());
        let mut carry = Carry::default();
        for _ in 0..count {
            let entry = decode_entry(rb, &mut carry)?;
            entries.push(entry);
        }
        match rb.remaining().len() {
            0 => Ok(Self {
                request_id,
                header,
                entries,
            }),
            n => Err(ClientInputError::Trailing(n)),
        }
    }

    /// Writes the body.
    ///
    /// # Panics
    ///
    /// If more than 255 entries, a header list over 16, or action words
    /// over 255: none fits the wire.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut wb = WriteBuffer::new(CARRIER_ENDIAN);
        self.marshal(&mut wb);
        wb.into_vec()
    }
}

impl Marshal for ClientInput {
    fn marshal(&self, wb: &mut WriteBuffer) {
        self.request_id.0.marshal(wb);
        self.request_id.1.marshal(wb);
        encode_header(wb, &self.header);
        wb.write_u8(u8::try_from(self.entries.len()).expect("at most 255 entries"));
        let mut carry = Carry::default();
        for entry in &self.entries {
            encode_entry(wb, entry, &mut carry);
        }
    }
}

fn decode_header(rb: &mut ReadBuffer) -> R<Header> {
    let b = *rb.read_array::<4>()?;
    let mut h = Header {
        b9: b[0],
        b10: b[1],
        b8: b[2],
        b11: b[3],
        v10: vlq64(rb)?,
        v1c8: vlq32(rb)?,
        v18: vlq32(rb)?,
        b1d: u8(rb)?,
        ..Header::default()
    };
    h.opt1d0 = if boolean(rb)? { Some(vlq64(rb)?) } else { None };
    h.u64_1d8 = u64(rb)?;
    h.b1c = u8(rb)?;
    h.v20 = vlq32(rb)?;
    h.v22 = vlq32(rb)?;
    h.v24 = vlq32(rb)?;
    h.b1e = u8(rb)?;
    let pairs = vlq32(rb)?;
    if pairs > MAX_HEADER_PAIRS {
        return Err(ClientInputError::TooLong {
            what: "a header list",
            len: pairs,
            max: MAX_HEADER_PAIRS,
        });
    }
    for _ in 0..pairs {
        h.list28.push((vlq64(rb)?, vlq64(rb)?));
    }
    h.v1b0 = vlq64(rb)?;
    h.v1b8 = vlq64(rb)?;
    h.v1c0 = vlq32(rb)?;
    h.v1c4 = vlq32(rb)?;
    Ok(h)
}

fn encode_header(wb: &mut WriteBuffer, h: &Header) {
    wb.write_bytes(&[h.b9, h.b10, h.b8, h.b11]);
    w_vlq64(wb, h.v10);
    w_vlq32(wb, h.v1c8);
    w_vlq32(wb, h.v18);
    wb.write_u8(h.b1d);
    w_bool(wb, h.opt1d0.is_some());
    if let Some(v) = h.opt1d0 {
        w_vlq64(wb, v);
    }
    h.u64_1d8.marshal(wb);
    wb.write_u8(h.b1c);
    w_vlq32(wb, h.v20);
    w_vlq32(wb, h.v22);
    w_vlq32(wb, h.v24);
    wb.write_u8(h.b1e);
    assert!(
        h.list28.len() <= MAX_HEADER_PAIRS as usize,
        "header list over 16"
    );
    w_vlq32(wb, h.list28.len() as u32);
    for &(a, b) in &h.list28 {
        w_vlq64(wb, a);
        w_vlq64(wb, b);
    }
    w_vlq64(wb, h.v1b0);
    w_vlq64(wb, h.v1b8);
    w_vlq32(wb, h.v1c0);
    w_vlq32(wb, h.v1c4);
}

fn decode_entry(rb: &mut ReadBuffer, carry: &mut Carry) -> R<Entry> {
    let m = *rb.read_array::<3>()?;
    let mut mask = Mask {
        value: u32::from_le_bytes([m[0], m[1], m[2], 0]),
        consumed: 0,
    };
    let mut e = Entry {
        counter: u16(rb)?,
        ..Entry::default()
    };
    if mask.take(0) {
        let f = vlq64(rb)?;
        e.flags = Some(f);
        carry.flags = f;
    }
    if mask.take(1) {
        e.b1 = Some(u8(rb)?);
    }
    if mask.take(2) {
        e.bool2 = Some(boolean(rb)?);
    }
    if mask.take(3) {
        e.b3 = Some(u8(rb)?);
    }
    if mask.take(4) {
        e.q4 = Some(u16(rb)?);
    }
    let words = u8(rb)?;
    while e.actions.len() < usize::from(words) {
        if usize::from(words) - e.actions.len() < 2 {
            e.actions.push(vlq32(rb)?);
        } else {
            let pair = vlq64(rb)?;
            e.actions.push(pair as u32);
            e.actions.push((pair >> 32) as u32);
        }
    }
    for a in &mut e.axes {
        *a = u8(rb)?;
    }
    if mask.take(5) {
        e.dt = Some(f32(rb)?);
    }
    if mask.take(6) {
        e.angles = Some((f32(rb)?, f32(rb)?));
    }
    if mask.take(7) {
        e.camera = Some(position(rb)?);
    }
    let kind = u8(rb)?;
    if kind != 3 {
        e.target = Some(Target {
            kind,
            b: u8(rb)?,
            position: position(rb)?,
            shorts: [u16(rb)?, u16(rb)?, u16(rb)?],
        });
    }
    if mask.take(8) {
        e.interaction = Some(if boolean(rb)? {
            Some(Box::new(decode_interaction(rb)?))
        } else {
            None
        });
    }
    decode_tail(rb, &mut e, &mut mask, carry)?;
    e.unread_mask = mask.value & !mask.consumed;
    Ok(e)
}

fn decode_interaction(rb: &mut ReadBuffer) -> R<Interaction> {
    let mut i = Interaction {
        id16: arr16(rb)?,
        q0: u64(rb)?,
        q1: u64(rb)?,
        b0: u8(rb)?,
        b1: u8(rb)?,
        q2: u64(rb)?,
        d0: u32(rb)?,
        s0: string(rb)?,
        d1: u32(rb)?,
        d2: u32(rb)?,
        d3: u32(rb)?,
        flag0: boolean(rb)?,
        s1: string(rb)?,
        s2: string(rb)?,
        s3: string(rb)?,
        s4: string(rb)?,
        d4: u32(rb)?,
        ..Interaction::default()
    };
    if boolean(rb)? {
        i.six = Some([u32(rb)?, u32(rb)?, u32(rb)?, u32(rb)?, u32(rb)?, u32(rb)?]);
    }
    i.flag1 = boolean(rb)?;
    i.d5 = u32(rb)?;
    i.b2 = u8(rb)?;
    for d in &mut i.d6 {
        *d = u32(rb)?;
    }
    i.v0 = vlq64(rb)?;
    i.d7 = u32(rb)?;
    i.v1 = vlq32(rb)?;
    i.s5 = string(rb)?;
    i.d8 = u32(rb)?;
    i.b3 = u8(rb)?;
    i.flag2 = boolean(rb)?;
    i.d9 = u32(rb)?;
    i.s6 = string(rb)?;
    i.d10 = u32(rb)?;
    i.q3 = u64(rb)?;
    i.d11 = u32(rb)?;
    i.flag3 = boolean(rb)?;
    Ok(i)
}

fn encode_interaction(wb: &mut WriteBuffer, i: &Interaction) {
    wb.write_bytes(&i.id16);
    i.q0.marshal(wb);
    i.q1.marshal(wb);
    wb.write_u8(i.b0);
    wb.write_u8(i.b1);
    i.q2.marshal(wb);
    wb.write_u32(i.d0);
    w_string(wb, &i.s0);
    wb.write_u32(i.d1);
    wb.write_u32(i.d2);
    wb.write_u32(i.d3);
    w_bool(wb, i.flag0);
    for s in [&i.s1, &i.s2, &i.s3, &i.s4] {
        w_string(wb, s);
    }
    wb.write_u32(i.d4);
    w_bool(wb, i.six.is_some());
    for d in i.six.iter().flatten() {
        wb.write_u32(*d);
    }
    w_bool(wb, i.flag1);
    wb.write_u32(i.d5);
    wb.write_u8(i.b2);
    for d in i.d6 {
        wb.write_u32(d);
    }
    w_vlq64(wb, i.v0);
    wb.write_u32(i.d7);
    w_vlq32(wb, i.v1);
    w_string(wb, &i.s5);
    wb.write_u32(i.d8);
    wb.write_u8(i.b3);
    w_bool(wb, i.flag2);
    wb.write_u32(i.d9);
    w_string(wb, &i.s6);
    wb.write_u32(i.d10);
    i.q3.marshal(wb);
    wb.write_u32(i.d11);
    w_bool(wb, i.flag3);
}

fn decode_contact(rb: &mut ReadBuffer) -> R<Contact> {
    let mut c = Contact::default();
    for v in &mut c.vectors {
        for f in v {
            *f = f32(rb)?;
        }
    }
    c.id16 = arr16(rb)?;
    c.d0 = u32(rb)?;
    for (i, r) in c.refs.iter_mut().enumerate() {
        *r = (arr16(rb)?, u64(rb)?);
        if i == 1 {
            c.ref1_extra = u64(rb)?;
        }
    }
    c.q0 = u64(rb)?;
    c.q1 = u64(rb)?;
    // The client refuses a value below its lower bound, with a relative
    // slack of one f32 epsilon.
    for (slot, low) in c.values.iter_mut().zip([-1.0_f32, 0.0, 0.0]) {
        let v = f32(rb)?;
        if v < low && (v - low).abs() > v.abs().max(1.0) * f32::EPSILON {
            return Err(ClientInputError::Unseen(format!(
                "a contact value of {v}, below {low}"
            )));
        }
        *slot = v;
    }
    c.d1 = u32(rb)?;
    c.d2 = u32(rb)?;
    c.d3 = u32(rb)?;
    c.flag0 = boolean(rb)?;
    c.flag1 = boolean(rb)?;
    c.b0 = u8(rb)?;
    c.flag2 = boolean(rb)?;
    Ok(c)
}

fn encode_contact(wb: &mut WriteBuffer, c: &Contact) {
    for v in &c.vectors {
        for f in v {
            f.marshal(wb);
        }
    }
    wb.write_bytes(&c.id16);
    wb.write_u32(c.d0);
    for (i, (id, q)) in c.refs.iter().enumerate() {
        wb.write_bytes(id);
        q.marshal(wb);
        if i == 1 {
            c.ref1_extra.marshal(wb);
        }
    }
    c.q0.marshal(wb);
    c.q1.marshal(wb);
    for f in c.values {
        f.marshal(wb);
    }
    wb.write_u32(c.d1);
    wb.write_u32(c.d2);
    wb.write_u32(c.d3);
    w_bool(wb, c.flag0);
    w_bool(wb, c.flag1);
    wb.write_u8(c.b0);
    w_bool(wb, c.flag2);
}

fn decode_tail(rb: &mut ReadBuffer, e: &mut Entry, mask: &mut Mask, carry: &mut Carry) -> R<()> {
    let flags = carry.flags;
    let t = &mut e.tail;
    let mut bit = 9;
    if flags & 1 != 0 {
        t.flag0_values = Some([vlq32(rb)?, vlq32(rb)?, vlq32(rb)?]);
    }
    if flags >> 11 & 1 != 0 {
        if mask.take(9) {
            t.position_flag11 = Some(position(rb)?);
        }
        bit = 10;
    }
    if flags >> 8 & 1 != 0 {
        if mask.take(bit) {
            t.position_flag8 = Some(position(rb)?);
        }
        if mask.take(bit + 1) {
            t.ref_flag8 = Some((arr16(rb)?, u64(rb)?));
        }
        bit += 2;
    }
    if mask.take(bit) {
        let n = u8(rb)?;
        t.tail_count = Some(n);
        carry.tail_count = n;
    }
    bit += 1;
    if carry.tail_count != 0 {
        if mask.take(bit) {
            t.position_tail = Some(position(rb)?);
        }
        if mask.take(bit + 1) {
            t.ref_tail = Some((arr16(rb)?, u64(rb)?, u64(rb)?));
        }
        bit += 2;
    }
    if flags >> 14 & 1 != 0 {
        if mask.take(bit) {
            return Err(ClientInputError::Unseen(
                "the flags bit-14 field".to_string(),
            ));
        }
        bit += 1;
    }
    for list in ["first", "second", "third"] {
        let n = vlq32(rb)?;
        if list == "second" {
            for _ in 0..n {
                t.contacts.push(decode_contact(rb)?);
            }
        } else if n != 0 {
            return Err(ClientInputError::Unseen(format!(
                "the {list} tail list holds {n}"
            )));
        }
    }
    if flags >> 15 & 1 != 0 {
        if mask.take(bit) {
            t.position_flag15 = Some(position(rb)?);
        }
        bit += 1;
    }
    if mask.take(bit) {
        t.halves = Some([u16(rb)?, u16(rb)?, u16(rb)?]);
    }
    if flags >> 16 & 1 != 0 {
        t.word_flag16 = Some(u64(rb)?);
    }
    if flags >> 18 & 1 != 0 {
        t.byte_flag18 = Some(u8(rb)?);
    }
    if mask.take(bit + 1) {
        t.byte_a = Some(u8(rb)?);
    }
    if mask.take(bit + 2) {
        t.word_b = Some(u32(rb)?);
    }
    if flags & 1 != 0 && mask.take(bit + 3) {
        t.half_c = Some(u16(rb)?);
    }
    Ok(())
}

/// Writes the entry body after the mask; returns the mask it implies.
fn encode_entry(wb: &mut WriteBuffer, e: &Entry, carry: &mut Carry) {
    let mut body = WriteBuffer::new(CARRIER_ENDIAN);
    let b = &mut body;
    let mut mask = e.unread_mask;
    let mut set = |n: u32, present: bool| {
        if present {
            mask |= 1 << n;
        }
        present
    };
    b.write_u16(e.counter);
    if set(0, e.flags.is_some()) {
        let f = e.flags.unwrap_or_default();
        w_vlq64(b, f);
        carry.flags = f;
    }
    if set(1, e.b1.is_some()) {
        b.write_u8(e.b1.unwrap_or_default());
    }
    if set(2, e.bool2.is_some()) {
        w_bool(b, e.bool2.unwrap_or_default());
    }
    if set(3, e.b3.is_some()) {
        b.write_u8(e.b3.unwrap_or_default());
    }
    if set(4, e.q4.is_some()) {
        b.write_u16(e.q4.unwrap_or_default());
    }
    let words = u8::try_from(e.actions.len()).expect("at most 255 action words");
    b.write_u8(words);
    for chunk in e.actions.chunks(2) {
        match chunk {
            [lo, hi] => w_vlq64(b, u64::from(*lo) | u64::from(*hi) << 32),
            [last] => w_vlq32(b, *last),
            _ => unreachable!(),
        }
    }
    b.write_bytes(&e.axes);
    if set(5, e.dt.is_some()) {
        e.dt.unwrap_or_default().marshal(b);
    }
    if set(6, e.angles.is_some()) {
        let (a, p) = e.angles.unwrap_or_default();
        a.marshal(b);
        p.marshal(b);
    }
    if set(7, e.camera.is_some()) {
        w_position(b, &e.camera.unwrap_or_default());
    }
    match &e.target {
        None => b.write_u8(3),
        Some(t) => {
            assert_ne!(t.kind, 3, "target kind 3 is the absent marker");
            b.write_u8(t.kind);
            b.write_u8(t.b);
            w_position(b, &t.position);
            for s in t.shorts {
                b.write_u16(s);
            }
        }
    }
    if set(8, e.interaction.is_some()) {
        match e.interaction.as_ref().and_then(|i| i.as_deref()) {
            None => w_bool(b, false),
            Some(i) => {
                w_bool(b, true);
                encode_interaction(b, i);
            }
        }
    }
    encode_tail(b, e, &mut mask, carry);
    let m = mask.to_le_bytes();
    wb.write_bytes(&m[..3]);
    wb.write_bytes(body.as_slice());
}

fn encode_tail(b: &mut WriteBuffer, e: &Entry, mask: &mut u32, carry: &mut Carry) {
    let flags = carry.flags;
    let t = &e.tail;
    let mut set = |n: u32, present: bool| {
        if present {
            *mask |= 1 << n;
        }
        present
    };
    let mut bit = 9;
    if flags & 1 != 0 {
        for v in t.flag0_values.unwrap_or_default() {
            w_vlq32(b, v);
        }
    }
    if flags >> 11 & 1 != 0 {
        if set(9, t.position_flag11.is_some()) {
            w_position(b, &t.position_flag11.unwrap_or_default());
        }
        bit = 10;
    }
    if flags >> 8 & 1 != 0 {
        if set(bit, t.position_flag8.is_some()) {
            w_position(b, &t.position_flag8.unwrap_or_default());
        }
        if set(bit + 1, t.ref_flag8.is_some()) {
            let (id, q) = t.ref_flag8.unwrap_or_default();
            b.write_bytes(&id);
            q.marshal(b);
        }
        bit += 2;
    }
    if set(bit, t.tail_count.is_some()) {
        let n = t.tail_count.unwrap_or_default();
        b.write_u8(n);
        carry.tail_count = n;
    }
    bit += 1;
    if carry.tail_count != 0 {
        if set(bit, t.position_tail.is_some()) {
            w_position(b, &t.position_tail.unwrap_or_default());
        }
        if set(bit + 1, t.ref_tail.is_some()) {
            let (id, q0, q1) = t.ref_tail.unwrap_or_default();
            b.write_bytes(&id);
            q0.marshal(b);
            q1.marshal(b);
        }
        bit += 2;
    }
    if flags >> 14 & 1 != 0 {
        bit += 1;
    }
    w_vlq32(b, 0);
    w_vlq32(b, t.contacts.len() as u32);
    for c in &t.contacts {
        encode_contact(b, c);
    }
    w_vlq32(b, 0);
    if flags >> 15 & 1 != 0 {
        if set(bit, t.position_flag15.is_some()) {
            w_position(b, &t.position_flag15.unwrap_or_default());
        }
        bit += 1;
    }
    if set(bit, t.halves.is_some()) {
        for h in t.halves.unwrap_or_default() {
            b.write_u16(h);
        }
    }
    if flags >> 16 & 1 != 0 {
        t.word_flag16.unwrap_or_default().marshal(b);
    }
    if flags >> 18 & 1 != 0 {
        b.write_u8(t.byte_flag18.unwrap_or_default());
    }
    if set(bit + 1, t.byte_a.is_some()) {
        b.write_u8(t.byte_a.unwrap_or_default());
    }
    if set(bit + 2, t.word_b.is_some()) {
        b.write_u32(t.word_b.unwrap_or_default());
    }
    if flags & 1 != 0 && set(bit + 3, t.half_c.is_some()) {
        b.write_u16(t.half_c.unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The request id and header for a one-entry body, with made-up values:
    /// every VLQ is one byte.
    fn head(entries: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&1u64.to_be_bytes());
        v.extend_from_slice(&2u64.to_be_bytes());
        v.extend_from_slice(&[9, 8, 7, 6]); // b9 b10 b8 b11
        v.push(5); // v10
        v.extend_from_slice(&[6, 7]); // v1c8, v18
        v.push(4); // b1d
        v.push(0); // no opt1d0
        v.extend_from_slice(&0x0102_0304_0506_0708u64.to_be_bytes());
        v.push(3); // b1c
        v.extend_from_slice(&[10, 11, 12]); // v20 v22 v24
        v.push(13); // b1e
        v.push(0); // list28
        v.extend_from_slice(&[14, 15]); // v1b0 v1b8
        v.extend_from_slice(&[16, 17]); // v1c0 v1c4
        v.push(entries);
        v
    }

    /// An entry with only `dt` (mask bit 5), no flags, no tail.
    fn plain_entry(counter: u16, dt: f32) -> Vec<u8> {
        let mut v = vec![0x20, 0, 0];
        v.extend_from_slice(&counter.to_be_bytes());
        v.push(0); // no action words
        v.extend_from_slice(&[128, 128, 128, 128]);
        v.extend_from_slice(&dt.to_be_bytes());
        v.push(3); // no target
        v.extend_from_slice(&[0, 0, 0]); // three empty tail lists
        v
    }

    #[test]
    fn a_hand_built_body_decodes_field_by_field_and_re_encodes() {
        let mut body = head(1);
        body.extend(plain_entry(7, 0.05));
        let m = ClientInput::decode(&body).unwrap();
        assert_eq!(m.request_id, ActorRequestId(1, 2));
        assert_eq!(m.header.b9, 9);
        assert_eq!(m.header.b11, 6);
        assert_eq!(m.header.v10, 5);
        assert_eq!(m.header.u64_1d8, 0x0102_0304_0506_0708);
        assert_eq!(m.header.opt1d0, None);
        assert_eq!(m.header.v1c4, 17);
        assert_eq!(m.entries.len(), 1);
        let e = &m.entries[0];
        assert_eq!(e.counter, 7);
        assert_eq!(e.dt, Some(0.05));
        assert_eq!(e.axis(0), 0.0);
        assert_eq!(e.target, None);
        assert_eq!(e.unread_mask, 0);
        assert_eq!(m.encode(), body);
    }

    #[test]
    fn the_request_id_is_two_big_endian_u64s() {
        let m = ClientInput {
            request_id: ActorRequestId(0x0102_0304_0506_0708, 0x090a_0b0c_0d0e_0f10),
            ..ClientInput::default()
        };
        assert_eq!(&m.encode()[..16], &(1..=16).collect::<Vec<u8>>()[..]);
    }

    #[test]
    fn action_words_pack_two_to_a_vlq64_and_a_lone_last_as_vlq32() {
        for words in [
            vec![],
            vec![1],
            vec![1, 2],
            vec![0xffff_ffff, 3, 0x8000_0000],
        ] {
            let m = ClientInput {
                entries: vec![Entry {
                    actions: words.clone(),
                    ..Entry::default()
                }],
                ..ClientInput::default()
            };
            let back = ClientInput::decode(&m.encode()).unwrap();
            assert_eq!(back.entries[0].actions, words);
        }
    }

    fn busy_entry(flags: Option<u64>) -> Entry {
        let p = Position::new(10.5, -3.25, 42.0);
        Entry {
            counter: 513,
            flags,
            b1: Some(1),
            bool2: Some(true),
            b3: Some(3),
            q4: Some(62),
            actions: (0..19).map(|i| 0x0101_0101 * (i % 3)).collect(),
            axes: [128, 255, 0, 64],
            dt: Some(0.0333),
            angles: Some((1.5, -0.25)),
            camera: Some(p),
            target: Some(Target {
                kind: 1,
                b: 2,
                position: p,
                shorts: [1, 2, 3],
            }),
            interaction: Some(Some(Box::new(Interaction {
                id16: [7; 16],
                q0: 1,
                s0: b"abc".to_vec(),
                six: Some([1, 2, 3, 4, 5, 6]),
                s6: vec![0x55; 300],
                flag3: true,
                ..Interaction::default()
            }))),
            tail: Tail {
                flag0_values: Some([1, 200, 70000]),
                position_flag11: Some(p),
                position_flag8: Some(p),
                ref_flag8: Some(([3; 16], 99)),
                tail_count: Some(2),
                position_tail: Some(p),
                ref_tail: Some(([4; 16], 5, 6)),
                contacts: vec![
                    Contact {
                        vectors: [[1.0, 2.0, 3.0]; 4],
                        refs: [([1; 16], 1), ([2; 16], 2), ([3; 16], 3)],
                        ref1_extra: 77,
                        values: [-1.0, 0.5, 1.0],
                        flag1: true,
                        ..Contact::default()
                    },
                    Contact::default(),
                ],
                position_flag15: Some(p),
                halves: Some([0x3c00, 0xbc00, 0]),
                word_flag16: Some(u64::MAX),
                byte_flag18: Some(8),
                byte_a: Some(9),
                word_b: Some(10),
                half_c: Some(0x4000),
            },
            unread_mask: 0,
        }
    }

    /// Flags 0, 8, 11, 15, 16 and 18: every flag-gated field exists.
    const FLAGS: u64 = 1 | 1 << 8 | 1 << 11 | 1 << 15 | 1 << 16 | 1 << 18;

    #[test]
    fn every_field_survives_a_round_trip() {
        let m = ClientInput {
            request_id: ActorRequestId(u64::MAX, 1),
            header: Header {
                opt1d0: Some(1 << 40),
                list28: vec![(1, 2), (u64::MAX, 0)],
                ..Header::default()
            },
            entries: vec![busy_entry(Some(FLAGS)), busy_entry(None)],
        };
        let bytes = m.encode();
        let back = ClientInput::decode(&bytes).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.encode(), bytes);
    }

    #[test]
    fn flags_and_tail_count_carry_to_the_next_entry() {
        // The second entry writes neither flags nor tail_count, yet its
        // flag-gated fields are still on the wire, so it does not decode
        // without the carry.
        let first = Entry {
            flags: Some(1),
            tail: Tail {
                flag0_values: Some([1, 2, 3]),
                tail_count: Some(1),
                ..Tail::default()
            },
            ..Entry::default()
        };
        let second = Entry {
            tail: Tail {
                flag0_values: Some([4, 5, 6]),
                position_tail: Some(Position::new(1.0, 2.0, 3.0)),
                ..Tail::default()
            },
            ..Entry::default()
        };
        let m = ClientInput {
            entries: vec![first, second],
            ..ClientInput::default()
        };
        let back = ClientInput::decode(&m.encode()).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.entries[1].flags, None);
    }

    #[test]
    fn mask_bits_no_field_uses_are_kept() {
        let mut body = head(1);
        body.extend(plain_entry(1, 0.05));
        let at = body.len() - plain_entry(1, 0.05).len();
        body[at + 2] = 0x80; // mask bit 23, which nothing reads
        let m = ClientInput::decode(&body).unwrap();
        assert_eq!(m.entries[0].unread_mask, 1 << 23);
        assert_eq!(m.encode(), body);
    }

    #[test]
    fn refused_bodies_are_errors() {
        let mut ok = head(1);
        ok.extend(plain_entry(1, 0.05));
        assert!(ClientInput::decode(&ok).is_ok());

        // Bytes left over.
        let mut long = ok.clone();
        long.push(0);
        assert!(matches!(
            ClientInput::decode(&long),
            Err(ClientInputError::Trailing(1))
        ));

        // A truncated body.
        assert!(ClientInput::decode(&ok[..ok.len() - 1]).is_err());

        // A non-empty first tail list.
        let mut first = ok.clone();
        let n = first.len();
        first[n - 3] = 1;
        assert!(matches!(
            ClientInput::decode(&first),
            Err(ClientInputError::Unseen(_))
        ));

        // A bool of 2: mask bit 2 with the byte 2.
        let mut b = head(1);
        b.extend_from_slice(&[0x04, 0, 0, 0, 1, 2]);
        assert!(matches!(
            ClientInput::decode(&b),
            Err(ClientInputError::Bool(2))
        ));

        // A header list over 16.
        let mut h = head(0);
        let list_at = 16 + 4 + 1 + 2 + 1 + 1 + 8 + 1 + 3 + 1;
        h[list_at] = 17;
        assert!(matches!(
            ClientInput::decode(&h),
            Err(ClientInputError::TooLong { .. })
        ));
    }

    #[test]
    fn the_flags_bit_14_field_is_refused() {
        // flags = bit 14 (VLQ64 of 0x4000 is 2 bytes), mask bits 0 and 10
        // (bit 14 puts its field at mask bit 10: bit 9 is the tail_count)
        let mut v = head(1);
        v.extend_from_slice(&[0x01, 0x04, 0]); // mask: bits 0 and 10
        v.extend_from_slice(&[0, 1]); // counter
        let mut w = WriteBuffer::new(CARRIER_ENDIAN);
        w_vlq64(&mut w, 1 << 14);
        v.extend_from_slice(w.as_slice());
        v.push(0); // words
        v.extend_from_slice(&[128; 4]);
        v.push(3);
        assert!(matches!(
            ClientInput::decode(&v),
            Err(ClientInputError::Unseen(_))
        ));
    }

    #[test]
    fn a_contact_value_below_its_bound_is_refused() {
        for (index, low_value) in [(0usize, -2.0f32), (1, -0.5), (2, -0.5)] {
            let mut contact = Contact::default();
            contact.values[index] = low_value;
            let mut c = WriteBuffer::new(CARRIER_ENDIAN);
            encode_contact(&mut c, &contact);
            let mut v = head(1);
            v.extend_from_slice(&[0, 0, 0, 0, 1, 0]); // mask 0, counter 1, no words
            v.extend_from_slice(&[128; 4]);
            v.push(3);
            v.extend_from_slice(&[0, 1]); // first list empty, second holds 1
            v.extend_from_slice(c.as_slice());
            v.push(0); // third list
            assert!(
                matches!(ClientInput::decode(&v), Err(ClientInputError::Unseen(_))),
                "value {index}"
            );
        }
    }

    #[test]
    fn z_converts_between_metres_and_the_raw_word() {
        assert_eq!(Position::new(0.0, 0.0, -100.0).z_raw, 0);
        assert_eq!(Position::new(0.0, 0.0, 1000.0).z_raw, u16::MAX);
        let p = Position::new(0.0, 0.0, 42.0);
        assert!((p.z() - 42.0).abs() < 0.01);
    }
}
