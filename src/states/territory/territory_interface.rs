//! `MB::TerritoryInterfaceComponentReplicatedState` (type 1746), written by
//! hand.
//!
//! The schema's wire shape for `territoryGovernanceData` is
//! `composite<u64, u64, u64, vec<composite<u8, u64>>, u64,
//! vec<composite<u8, composite<u64, u64, u64>>>, u64,
//! vec<composite<u64, vec<composite<u8, composite<u64, u64, u64>>>, u64>>,
//! u32>`, but its nested type shape lists only one `u32` member, so the
//! generated state read 4 bytes. Live writes the whole composite: read as
//! a `u32`, every governance update ended early and the rest of its bundle
//! was read from the wrong offset. The members' meanings are not known;
//! they keep neutral names. The other fields keep the generated shapes.

use crate::Marshaler;
use crate::{az_rtti, replicated_state, type_registry};

/// `(u8, (u64, u64, u64))`: an entry of [`TerritoryGovernanceData`]'s
/// second and nested lists.
pub type TerritoryGovernanceTriple = (u8, (u64, u64, u64));

/// `territoryGovernanceData`, in live's layout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct TerritoryGovernanceData {
    pub field_0: u64,
    pub field_1: u64,
    pub field_2: u64,
    pub field_3: Vec<(u8, u64)>,
    pub field_4: u64,
    pub field_5: Vec<TerritoryGovernanceTriple>,
    pub field_6: u64,
    pub field_7: Vec<(u64, Vec<TerritoryGovernanceTriple>, u64)>,
    pub field_8: u32,
}

#[replicated_state]
#[az_rtti("D6E566ED-C562-4940-BB3E-F3BC3699DB76")]
#[type_registry(1746u32)]
#[derive(Debug, Clone, Default)]
pub struct TerritoryInterfaceComponentReplicatedState {
    #[replicated_state(group = 1u32)]
    pub territory_progression_data:
        crate::serialize::ReplicatedFieldHandler<crate::source::PersistentTerritoryProgressionData>,
    #[replicated_state(group = 2u32)]
    pub territory_governance_data:
        crate::serialize::ReplicatedFieldHandler<TerritoryGovernanceData>,
    pub territory_functionality_down: crate::serialize::ReplicatedFieldHandler<bool>,
    pub territory_refs_set: crate::serialize::ReplicatedFieldHandler<bool>,
}

#[cfg(test)]
mod tests {
    use super::{TerritoryGovernanceData, TerritoryInterfaceComponentReplicatedState};
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// A governance update in the layout live sends, one entry per list,
    /// then the two flags of group 0. The values are made up.
    const GOVERNANCE: &[u8] = &[
        0x05, // groups 0 and 2 dirty
        0x03, // group 0: fields 0 and 1 (the flags), no more masks
        0x01, // functionality down
        0x00, // refs set
        0x01, // group 2: field 0 (governance data)
        0, 0, 0, 0, 0, 0, 0, 0x01, // field 0
        0, 0, 0, 0, 0, 0, 0, 0x02, // field 1
        0, 0, 0, 0, 0, 0, 0, 0x03, // field 2
        0x01, // field 3: one entry
        0x07, 0, 0, 0, 0, 0, 0, 0, 0x04, //
        0, 0, 0, 0, 0, 0, 0, 0x05, // field 4
        0x01, // field 5: one entry
        0x08, 0, 0, 0, 0, 0, 0, 0, 0x06, 0, 0, 0, 0, 0, 0, 0, 0x07, 0, 0, 0, 0, 0, 0, 0,
        0x08, //
        0, 0, 0, 0, 0, 0, 0, 0x09, // field 6
        0x01, // field 7: one entry
        0, 0, 0, 0, 0, 0, 0, 0x0a, // its first value
        0x00, // its list: empty
        0, 0, 0, 0, 0, 0, 0, 0x0b, // its last value
        0, 0, 0, 0x0c, // field 8
    ];

    #[test]
    fn governance_data_is_the_whole_composite() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, GOVERNANCE);
        let fragment = FragmentTypeInfo::TypeIndex(1746)
            .decode_contents(&mut rb)
            .expect("territory interface contents decode");
        assert_eq!(rb.position(), GOVERNANCE.len());
        let state = fragment
            .downcast_ref::<TerritoryInterfaceComponentReplicatedState>()
            .expect("type index 1746 is TerritoryInterfaceComponentReplicatedState");
        assert_eq!(state.territory_functionality_down.value(), Some(&true));
        assert_eq!(state.territory_refs_set.value(), Some(&false));
        assert_eq!(
            state.territory_governance_data.value(),
            Some(&TerritoryGovernanceData {
                field_0: 1,
                field_1: 2,
                field_2: 3,
                field_3: vec![(7, 4)],
                field_4: 5,
                field_5: vec![(8, (6, 7, 8))],
                field_6: 9,
                field_7: vec![(10, Vec::new(), 11)],
                field_8: 12,
            })
        );
    }
}
