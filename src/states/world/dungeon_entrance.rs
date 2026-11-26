//! `Javelin::DungeonEntranceReplicatedState` (type 3105), written by hand.
//!
//! The schema gives its one field, `dungeonsSpawned`, the shape
//! `vec<composite<fixed-bytes-16, u8, fixed-bytes-16>>` (33 bytes per
//! entry), but live writes 41 bytes per entry: eight bytes more, ahead of
//! the first 16-byte id. Read as 33, every entrance fragment ended eight
//! bytes per entry early and the rest of its bundle was read from the wrong
//! offset. The live entries put an 8-byte value first (zero or small), then
//! two 16-byte ids around the byte; what the leading value means is not
//! known, so it keeps a neutral name.

use crate::Marshaler;
use crate::{az_rtti, replicated_state, type_registry};

/// One `dungeonsSpawned` entry, in live's 41-byte layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Marshaler)]
pub struct DungeonSpawned {
    /// The 8 bytes the schema does not list.
    pub field_0: u64,
    pub field_1: [u8; 16],
    pub field_2: u8,
    pub field_3: [u8; 16],
}

#[replicated_state]
#[az_rtti("276E6468-892B-43BA-80B2-F31F4CD8430D")]
#[type_registry(3105u32)]
#[derive(Debug, Clone, Default)]
pub struct DungeonEntranceReplicatedState {
    pub dungeons_spawned: crate::serialize::ReplicatedFieldHandler<Vec<DungeonSpawned>>,
}

#[cfg(test)]
mod tests {
    use super::{DungeonEntranceReplicatedState, DungeonSpawned};
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// Two entries in the layout live sends: a count, then 41 bytes each.
    /// The values are made up.
    const TWO_ENTRIES: &[u8] = &[
        0x01, // group 0 dirty
        0x01, // field 0 (dungeonsSpawned)
        0x02, // two entries
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, // entry 0: leading 8 bytes
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, //
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, // first id
        0x02, // byte
        0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, //
        0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, // second id
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry 1: leading 8 bytes
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, //
        0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, // first id
        0x00, // byte
        0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, //
        0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, // second id
    ];

    #[test]
    fn dungeons_spawned_entries_are_41_bytes() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, TWO_ENTRIES);
        let fragment = FragmentTypeInfo::TypeIndex(3105)
            .decode_contents(&mut rb)
            .expect("dungeon entrance contents decode");
        assert_eq!(rb.position(), TWO_ENTRIES.len());
        let state = fragment
            .downcast_ref::<DungeonEntranceReplicatedState>()
            .expect("type index 3105 is DungeonEntranceReplicatedState");
        let entries = state.dungeons_spawned.value().expect("entries set");
        assert_eq!(entries.len(), 2);
        let ids = |base: u8| std::array::from_fn::<u8, 16, _>(|i| base + i as u8);
        assert_eq!(
            entries[0],
            DungeonSpawned {
                field_0: 7,
                field_1: ids(0x10),
                field_2: 2,
                field_3: ids(0x20),
            }
        );
        assert_eq!(entries[1].field_1, ids(0x30));
        assert_eq!(entries[1].field_3, ids(0x40));
    }
}
