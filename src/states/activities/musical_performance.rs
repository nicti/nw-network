//! `Javelin::MusicalPerformanceReplicatedState` (type 4896), written by hand.
//!
//! Generated otherwise, except the performer seeds. The constructor
//! registers one descriptor per performer slot: for each slot `i` below the
//! runtime setting `javelin.musical-performances.max_performers` it creates
//! a descriptor (group `4 + i`) and registers `performerSeed_i` in it; the
//! first slot's group also carries `performanceZonePlayerCount`. The schema
//! lists four slots (groups 4..=7, from a runtime profile), so the generated
//! state read one descriptor mask byte, but live writes two: every live
//! fragment carries a second, zero, descriptor byte after its fields. Read
//! with one, each fragment ended a byte early and the rest of its bundle was
//! lost. Two bytes means at least five slots (group 8) and at most twelve
//! (group 15); the state declares twelve, groups 4..=15. A slot past the
//! live setting never has a dirty bit, so the extra slots only fix the mask
//! width. Group 3 is created by the constructor but holds no field.

use ::nw_network::{az_rtti, replicated_state, type_registry};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerformerScoreValue {
    pub field_0: u32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct PerformerScoreValueMarshaler;
impl ::nw_network::serialize::Codec<PerformerScoreValue> for PerformerScoreValueMarshaler {
    const MARSHAL_SIZE: usize = <::nw_network::serialize::DefaultMarshaler<
        u32,
    > as ::nw_network::serialize::Codec<u32>>::MARSHAL_SIZE;
    fn marshal(value: &PerformerScoreValue, wb: &mut ::nw_network::serialize::WriteBuffer) {
        <::nw_network::serialize::DefaultMarshaler<
            u32,
        > as ::nw_network::serialize::Codec<u32>>::marshal(&value.field_0, wb);
    }
    fn unmarshal(
        rb: &mut ::nw_network::serialize::ReadBuffer,
    ) -> Result<PerformerScoreValue, ::nw_network::serialize::MarshalerError> {
        let field_field_0 =
            <::nw_network::serialize::DefaultMarshaler<u32> as ::nw_network::serialize::Codec<
                u32,
            >>::unmarshal(rb)?;
        Ok(PerformerScoreValue {
            field_0: field_field_0,
        })
    }
}
impl ::nw_network::serialize::Marshal for PerformerScoreValue {
    const MARSHAL_SIZE: usize = <PerformerScoreValueMarshaler as ::nw_network::serialize::Codec<
        PerformerScoreValue,
    >>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut ::nw_network::serialize::WriteBuffer) {
        <PerformerScoreValueMarshaler as ::nw_network::serialize::Codec<
            PerformerScoreValue,
        >>::marshal(self, wb);
    }
}
impl ::nw_network::serialize::Unmarshal for PerformerScoreValue {
    fn unmarshal(
        rb: &mut ::nw_network::serialize::ReadBuffer,
    ) -> Result<Self, ::nw_network::serialize::MarshalerError> {
        <PerformerScoreValueMarshaler as ::nw_network::serialize::Codec<
            PerformerScoreValue,
        >>::unmarshal(rb)
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerformancePagesValue {
    pub field_0: u16,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct PerformancePagesValueMarshaler;
impl ::nw_network::serialize::Codec<PerformancePagesValue> for PerformancePagesValueMarshaler {
    const MARSHAL_SIZE: usize = <::nw_network::serialize::DefaultMarshaler<
        u16,
    > as ::nw_network::serialize::Codec<u16>>::MARSHAL_SIZE;
    fn marshal(value: &PerformancePagesValue, wb: &mut ::nw_network::serialize::WriteBuffer) {
        <::nw_network::serialize::DefaultMarshaler<
            u16,
        > as ::nw_network::serialize::Codec<u16>>::marshal(&value.field_0, wb);
    }
    fn unmarshal(
        rb: &mut ::nw_network::serialize::ReadBuffer,
    ) -> Result<PerformancePagesValue, ::nw_network::serialize::MarshalerError> {
        let field_field_0 =
            <::nw_network::serialize::DefaultMarshaler<u16> as ::nw_network::serialize::Codec<
                u16,
            >>::unmarshal(rb)?;
        Ok(PerformancePagesValue {
            field_0: field_field_0,
        })
    }
}
impl ::nw_network::serialize::Marshal for PerformancePagesValue {
    const MARSHAL_SIZE: usize = <PerformancePagesValueMarshaler as ::nw_network::serialize::Codec<
        PerformancePagesValue,
    >>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut ::nw_network::serialize::WriteBuffer) {
        <PerformancePagesValueMarshaler as ::nw_network::serialize::Codec<
            PerformancePagesValue,
        >>::marshal(self, wb);
    }
}
impl ::nw_network::serialize::Unmarshal for PerformancePagesValue {
    fn unmarshal(
        rb: &mut ::nw_network::serialize::ReadBuffer,
    ) -> Result<Self, ::nw_network::serialize::MarshalerError> {
        <PerformancePagesValueMarshaler as ::nw_network::serialize::Codec<
            PerformancePagesValue,
        >>::unmarshal(rb)
    }
}
#[replicated_state]
#[az_rtti("2C91F563-3901-413D-941E-0CEF9366896D")]
#[type_registry(4896u32)]
#[derive(Debug, Clone, Default)]
pub struct MusicalPerformanceReplicatedState {
    pub performers: ::nw_network::serialize::ReplicatedContainer<
        ::std::vec::Vec<u64>,
        { ::nw_network::serialize::WIRE_VEC_CAP },
        ::nw_network::serialize::DefaultMarshaler<::nw_network::serialize::VlqU64>,
        ::nw_network::serialize::DefaultMarshaler<u64>,
    >,
    pub radius: ::nw_network::serialize::ReplicatedFieldHandler<f32>,
    pub zone_type: ::nw_network::serialize::ReplicatedFieldHandler<u32>,
    pub selected_song: ::nw_network::serialize::ReplicatedFieldHandler<u32>,
    pub packed_performer_sheet_indices: ::nw_network::serialize::ReplicatedFieldHandler<u16>,
    pub start_time:
        ::nw_network::serialize::ReplicatedFieldHandler<::nw_network::source::TimePoint>,
    pub performance_state: ::nw_network::serialize::ReplicatedFieldHandler<u8>,
    pub flags: ::nw_network::serialize::ReplicatedFieldHandler<u8>,
    #[replicated_state(group = 1u32)]
    pub leader_index: ::nw_network::serialize::ReplicatedFieldHandler<u8>,
    #[replicated_state(group = 1u32)]
    pub error_sync: ::nw_network::serialize::ReplicatedFieldHandler<u32>,
    #[replicated_state(group = 1u32)]
    pub selected_reward_id: ::nw_network::serialize::ReplicatedFieldHandler<u32>,
    #[replicated_state(group = 1u32)]
    pub performer_score: ::nw_network::serialize::ReplicatedContainer<
        ::std::vec::Vec<PerformerScoreValue>,
        { ::nw_network::serialize::WIRE_VEC_CAP },
        ::nw_network::serialize::DefaultMarshaler<::nw_network::serialize::VlqU64>,
        PerformerScoreValueMarshaler,
    >,
    #[replicated_state(group = 1u32)]
    pub performance_score: ::nw_network::serialize::ReplicatedFieldHandler<f32>,
    #[replicated_state(group = 2u32)]
    pub performance_pages: ::nw_network::serialize::ReplicatedContainer<
        ::std::vec::Vec<PerformancePagesValue>,
        { ::nw_network::serialize::WIRE_VEC_CAP },
        ::nw_network::serialize::DefaultMarshaler<::nw_network::serialize::VlqU64>,
        PerformancePagesValueMarshaler,
    >,
    #[replicated_state(group = 2u32)]
    pub song_book_filter_type: ::nw_network::serialize::ReplicatedFieldHandler<u8>,
    #[replicated_state(group = 2u32)]
    pub song_book_filter_type_id: ::nw_network::serialize::ReplicatedFieldHandler<u32>,
    #[replicated_state(group = 2u32)]
    pub song_book_overrides: ::nw_network::serialize::ReplicatedContainer<
        ::std::vec::Vec<u32>,
        { ::nw_network::serialize::WIRE_VEC_CAP },
        ::nw_network::serialize::DefaultMarshaler<::nw_network::serialize::VlqU64>,
        ::nw_network::serialize::DefaultMarshaler<u32>,
    >,
    #[replicated_state(group = 2u32)]
    pub reward_filter_type: ::nw_network::serialize::ReplicatedFieldHandler<u8>,
    #[replicated_state(group = 2u32)]
    pub reward_filter_type_id: ::nw_network::serialize::ReplicatedFieldHandler<u32>,
    #[replicated_state(group = 2u32)]
    pub reward_overrides: ::nw_network::serialize::ReplicatedContainer<
        ::std::vec::Vec<u32>,
        { ::nw_network::serialize::WIRE_VEC_CAP },
        ::nw_network::serialize::DefaultMarshaler<::nw_network::serialize::VlqU64>,
        ::nw_network::serialize::DefaultMarshaler<u32>,
    >,
    #[replicated_state(group = 2u32)]
    pub rank: ::nw_network::serialize::ReplicatedFieldHandler<u16>,
    #[replicated_state(group = 2u32)]
    pub end_reason: ::nw_network::serialize::ReplicatedFieldHandler<u8>,
    #[replicated_state(group = 4u32)]
    pub performer_seed_0: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 4u32)]
    pub performance_zone_player_count: ::nw_network::serialize::ReplicatedFieldHandler<u16>,
    #[replicated_state(group = 5u32)]
    pub performer_seed_1: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 6u32)]
    pub performer_seed_2: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 7u32)]
    pub performer_seed_3: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 8u32)]
    pub performer_seed_4: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 9u32)]
    pub performer_seed_5: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 10u32)]
    pub performer_seed_6: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 11u32)]
    pub performer_seed_7: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 12u32)]
    pub performer_seed_8: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 13u32)]
    pub performer_seed_9: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 14u32)]
    pub performer_seed_10: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 15u32)]
    pub performer_seed_11: ::nw_network::serialize::ReplicatedFieldHandler<u64>,
}

#[cfg(test)]
mod tests {
    use super::MusicalPerformanceReplicatedState;
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    fn decode(bytes: &[u8]) -> (MusicalPerformanceReplicatedState, usize) {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, bytes);
        let fragment = FragmentTypeInfo::TypeIndex(4896)
            .decode_contents(&mut rb)
            .expect("musical performance contents decode");
        let state = fragment
            .downcast_ref::<MusicalPerformanceReplicatedState>()
            .expect("type index 4896 is MusicalPerformanceReplicatedState")
            .clone();
        (state, rb.position())
    }

    /// A state change in the layout live sends: two descriptor bytes, the
    /// second zero. The values are made up.
    const PERFORMANCE_STATE: &[u8] = &[
        0x01, // groups 0..=7: group 0 dirty
        0x40, // field 6 (performanceState)
        0x03, // performance state 3
        0x00, // groups 8..=15: none dirty
    ];

    /// A fifth performer's seed, in group 8 behind the second descriptor
    /// byte. The values are made up.
    const FIFTH_SEED: &[u8] = &[
        0x00, // groups 0..=7: none dirty
        0x01, // groups 8..=15: group 8 dirty
        0x01, // its field 0 (performerSeed_4)
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // seed
    ];

    #[test]
    fn live_fragments_carry_two_descriptor_bytes() {
        let (state, consumed) = decode(PERFORMANCE_STATE);
        assert_eq!(consumed, PERFORMANCE_STATE.len());
        assert_eq!(state.performance_state.value(), Some(&3));
    }

    #[test]
    fn performer_slot_groups_follow_group_seven() {
        let (state, consumed) = decode(FIFTH_SEED);
        assert_eq!(consumed, FIFTH_SEED.len());
        assert_eq!(state.performer_seed_4.value(), Some(&0x0102_0304_0506_0708));
        assert_eq!(state.performer_seed_3.value(), None);
    }
}
