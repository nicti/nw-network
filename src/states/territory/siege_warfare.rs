//! `MB::SiegeWarfareDataReplicatedState` (type 4103), written by hand.
//!
//! The schema registers `capturePointStatesColdData` as a plain field, and
//! the generated state read it as a list of `(u8, 8 bytes, optional
//! sequence, 26 bytes)`, the per-entry shape of a container delta. Live
//! writes a replicated container (mode, sequence, count, then an 8-byte key
//! and a 26-byte value per entry), like the hot data beside it. Read as a
//! list, the snapshot's mode byte was taken for an empty list and the rest
//! of the fragment from the wrong offset (`invalid enum discriminant`). The
//! value's members keep neutral names; the other fields keep the generated
//! shapes.

use crate::Marshaler;
use crate::generated::states::siege_warfare_data_replicated_state as generated;
use crate::serialize::{DefaultMarshaler, IndexMap, ReplicatedContainer, WIRE_VEC_CAP};
use crate::{az_rtti, replicated_state, type_registry};

/// A `capturePointStatesColdData` entry's value, in live's 26-byte layout.
#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct CapturePointColdData {
    pub field_0: f32,
    pub field_1: f32,
    pub field_2: f32,
    pub field_3: u32,
    pub field_4: u8,
    pub field_5: u64,
    pub field_6: u8,
}

#[replicated_state]
#[az_rtti("F421ABB9-B5B6-470A-9F66-6B1D725CF6E8")]
#[type_registry(4103u32)]
#[derive(Debug, Clone, Default)]
pub struct SiegeWarfareDataReplicatedState {
    pub capture_point_states_cold_data: ReplicatedContainer<
        IndexMap<u64, CapturePointColdData>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u64>,
        DefaultMarshaler<CapturePointColdData>,
    >,
    pub capture_point_states_hot_data: ReplicatedContainer<
        IndexMap<u64, u8>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u64>,
        DefaultMarshaler<u8>,
    >,
    pub fort_major_structure_states: ReplicatedContainer<
        IndexMap<u64, generated::FortMajorStructureStatesValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u64>,
        generated::FortMajorStructureStatesValueMarshaler,
    >,
    pub siege_weapon_states: ReplicatedContainer<
        IndexMap<u64, generated::SiegeWeaponStatesValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u64>,
        generated::SiegeWeaponStatesValueMarshaler,
    >,
    pub num_attacker_buildables_bound_by_team_limit: ReplicatedContainer<
        IndexMap<u32, u8>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u32>,
        DefaultMarshaler<u8>,
    >,
    pub num_defender_buildables_bound_by_team_limit: ReplicatedContainer<
        IndexMap<u32, u8>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<u32>,
        DefaultMarshaler<u8>,
    >,
}

#[cfg(test)]
mod tests {
    use super::{CapturePointColdData, SiegeWarfareDataReplicatedState};
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// A cold data snapshot in the layout live sends, one entry, then an
    /// empty hot data snapshot. The values are made up.
    const COLD_DATA: &[u8] = &[
        0x01, // group 0 dirty
        0x03, // fields 0 and 1 (cold and hot data), no more masks
        0x00, // cold data: snapshot
        0x01, 0x04, // sequence 4
        0x01, // one entry
        0, 0, 0, 0, 0, 0, 0, 0x07, // key
        0x3f, 0x80, 0x00, 0x00, // 1.0
        0x40, 0x00, 0x00, 0x00, // 2.0
        0x40, 0x40, 0x00, 0x00, // 3.0
        0, 0, 0, 0x03, // field 3
        0x04, // field 4
        0, 0, 0, 0, 0, 0, 0, 0x05, // field 5
        0x01, // field 6
        0x00, // hot data: snapshot
        0x01, 0x04, // sequence 4
        0x00, // no entries
    ];

    #[test]
    fn cold_data_is_a_container() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, COLD_DATA);
        let fragment = FragmentTypeInfo::TypeIndex(4103)
            .decode_contents(&mut rb)
            .expect("siege warfare contents decode");
        assert_eq!(rb.position(), COLD_DATA.len());
        let state = fragment
            .downcast_ref::<SiegeWarfareDataReplicatedState>()
            .expect("type index 4103 is SiegeWarfareDataReplicatedState");
        let cold = state.capture_point_states_cold_data.clone().into_values();
        assert_eq!(
            cold.get(&7),
            Some(&CapturePointColdData {
                field_0: 1.0,
                field_1: 2.0,
                field_2: 3.0,
                field_3: 3,
                field_4: 4,
                field_5: 5,
                field_6: 1,
            })
        );
    }
}
