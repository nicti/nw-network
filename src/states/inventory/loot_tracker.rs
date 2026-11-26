//! Loot roll, diversion, and per-entity loot-limit replication.
//!
//! `MB::LootTrackerComponentReplicatedState` (type 982) is written by hand
//! for `m_slayerScriptDataMap`: the schema registers it as a plain field of
//! length-prefixed bytes, but live writes a replicated container (mode,
//! sequence, count), keyed by a 16-byte id with a [`SlayerScriptLootData`]
//! (`u8`, `u64`) per entry, the sequence shared with the state's other
//! containers. Read as bytes, its mode byte was taken for an empty length
//! and the rest of the bundle was read from the wrong offset. The other
//! fields keep the generated shapes.

use crate::Marshaler;
use crate::generated::states::loot_tracker_component_replicated_state as generated;
use crate::serialize::{
    DefaultMarshaler, IndexMap, ReplicatedContainer, ReplicatedFieldHandler, VlqU64, WIRE_VEC_CAP,
};
use crate::{az_rtti, replicated_state, type_registry};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Marshaler)]
pub struct LootTrackerKey(pub [u8; 16]);

#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct LootRollData {
    pub loot_table_id: u32,
    pub roll_id: u32,
    pub source_time: u64,
    pub expiration_time: u64,
    pub active: bool,
    pub weights: [f32; 3],
    pub item_id: u32,
    pub tier: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct SlayerScriptLootData {
    pub state: u8,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct LootDivertMapValue {
    pub divert_type: u8,
    pub target_id: u64,
    pub quantity: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct LootLimitStateData {
    pub window_start: u64,
    pub window_end: u64,
    pub limit: u16,
    pub state: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LootDivertEntry {
    pub key: u32,
    pub data: LootDivertMapValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LootTrackerSnapshot {
    pub loot_data_map_sequence: u64,
    pub loot_divert_map_sequence: u64,
    pub loot_diverts: Vec<LootDivertEntry>,
}

#[replicated_state]
#[az_rtti("756DEAE5-A1F0-4863-BE20-B44D871C46A1")]
#[type_registry(982u32)]
#[derive(Debug, Clone, Default)]
pub struct LootTrackerComponentReplicatedState {
    pub loot_data_map: ReplicatedContainer<
        IndexMap<[u8; 16], generated::LootDataMapValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<[u8; 16]>,
        generated::LootDataMapValueMarshaler,
    >,
    pub loot_collectibles: ReplicatedContainer<
        Vec<u64>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        DefaultMarshaler<u64>,
    >,
    pub failed_roll_bonus_percent: ReplicatedFieldHandler<f32>,
    pub slayer_script_data_map: ReplicatedContainer<
        IndexMap<LootTrackerKey, SlayerScriptLootData>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<LootTrackerKey>,
        DefaultMarshaler<SlayerScriptLootData>,
    >,
    pub loot_divert_map: ReplicatedContainer<
        IndexMap<u32, generated::LootDivertMapValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u32>,
        generated::LootDivertMapValueMarshaler,
    >,
    pub loot_limit_data_map: ReplicatedContainer<
        IndexMap<u32, generated::LootLimitData>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u32>,
        generated::LootLimitDataMarshaler,
    >,
}

#[cfg(test)]
mod tests {
    use super::{LootTrackerComponentReplicatedState, LootTrackerKey, SlayerScriptLootData};
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// A slayer script data update in the layout live sends: a container
    /// snapshot with one entry. The values are made up.
    const SLAYER_SCRIPT_DATA: &[u8] = &[
        0x01, // group 0 dirty
        0x08, // field 3 (slayer script data map), no more masks
        0x00, // snapshot
        0x01, 0x05, // sequence 5
        0x01, // one entry
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, //
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, // key
        0x02, // state
        0, 0, 0, 0, 0, 0, 0, 0x2a, // timestamp
    ];

    #[test]
    fn slayer_script_data_is_a_container() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, SLAYER_SCRIPT_DATA);
        let fragment = FragmentTypeInfo::TypeIndex(982)
            .decode_contents(&mut rb)
            .expect("loot tracker contents decode");
        assert_eq!(rb.position(), SLAYER_SCRIPT_DATA.len());
        let state = fragment
            .downcast_ref::<LootTrackerComponentReplicatedState>()
            .expect("type index 982 is LootTrackerComponentReplicatedState");
        let key = LootTrackerKey(std::array::from_fn(|i| 0x10 + i as u8));
        assert_eq!(
            state
                .slayer_script_data_map
                .clone()
                .into_values()
                .get(&key)
                .cloned(),
            Some(SlayerScriptLootData {
                state: 2,
                timestamp: 42,
            })
        );
    }
}
