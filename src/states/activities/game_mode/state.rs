//! `Javelin::GameModeReplicatedState` (type 2343), written by hand.
//!
//! The schema lists only the 27 fields the game mode registers itself, but
//! the state derives from `MB::SlayerScriptReplicatedState`, whose
//! constructor registers `curScriptStateId`, `curScriptId` and
//! `spawnedEntityIdsBySpawnerId` first (as the schema does list for
//! `InstancedSlayerScriptReplicatedState`, type 6234). A generated game mode
//! read every field mask bit one field late: a live update of the script
//! state id (one byte) decoded as a four-byte `gameModeId`, and the rest of
//! its bundle was read from the wrong offset.
//!
//! The game mode's own fields keep the generated shapes, except `values` and
//! `syncedTimers`, which are the scripted timer maps of
//! [`SyncedTimerMap`]: their handler reads a delta change's timer as a VLQ
//! and a snapshot entry's timer as a fixed `u64`.

use crate::states::scripting::SyncedTimerMap;
use crate::types::{Crc32, EntityId};
use crate::{az_rtti, replicated_state, type_registry};
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParticipantFacetRefsValue {
    pub field_0: [u8; 16],
    pub field_1: u64,
    pub field_2: u64,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticipantFacetRefsValueMarshaler;
impl crate::serialize::Codec<ParticipantFacetRefsValue> for ParticipantFacetRefsValueMarshaler {
    const MARSHAL_SIZE: usize = <crate::serialize::DefaultMarshaler<
        [u8; 16],
    > as crate::serialize::Codec<[u8; 16]>>::MARSHAL_SIZE
        + <crate::serialize::DefaultMarshaler<
            u64,
        > as crate::serialize::Codec<u64>>::MARSHAL_SIZE
        + <crate::serialize::DefaultMarshaler<
            u64,
        > as crate::serialize::Codec<u64>>::MARSHAL_SIZE;
    fn marshal(value: &ParticipantFacetRefsValue, wb: &mut crate::serialize::WriteBuffer) {
        <crate::serialize::DefaultMarshaler<
            [u8; 16],
        > as crate::serialize::Codec<[u8; 16]>>::marshal(&value.field_0, wb);
        <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<u64>>::marshal(
            &value.field_1,
            wb,
        );
        <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<u64>>::marshal(
            &value.field_2,
            wb,
        );
    }
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<ParticipantFacetRefsValue, crate::serialize::MarshalerError> {
        let field_field_0 = <crate::serialize::DefaultMarshaler<
            [u8; 16],
        > as crate::serialize::Codec<[u8; 16]>>::unmarshal(rb)?;
        let field_field_1 = <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<
            u64,
        >>::unmarshal(rb)?;
        let field_field_2 = <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<
            u64,
        >>::unmarshal(rb)?;
        Ok(ParticipantFacetRefsValue {
            field_0: field_field_0,
            field_1: field_field_1,
            field_2: field_field_2,
        })
    }
}
impl crate::serialize::Marshal for ParticipantFacetRefsValue {
    const MARSHAL_SIZE: usize = <ParticipantFacetRefsValueMarshaler as crate::serialize::Codec<
        ParticipantFacetRefsValue,
    >>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut crate::serialize::WriteBuffer) {
        <ParticipantFacetRefsValueMarshaler as crate::serialize::Codec<
            ParticipantFacetRefsValue,
        >>::marshal(self, wb);
    }
}
impl crate::serialize::Unmarshal for ParticipantFacetRefsValue {
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Self, crate::serialize::MarshalerError> {
        <ParticipantFacetRefsValueMarshaler as crate::serialize::Codec<
            ParticipantFacetRefsValue,
        >>::unmarshal(rb)
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParticipantTeamIndexesValue {
    pub field_0: u8,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticipantTeamIndexesValueMarshaler;
impl crate::serialize::Codec<ParticipantTeamIndexesValue> for ParticipantTeamIndexesValueMarshaler {
    const MARSHAL_SIZE: usize =
        <crate::serialize::DefaultMarshaler<u8> as crate::serialize::Codec<u8>>::MARSHAL_SIZE;
    fn marshal(value: &ParticipantTeamIndexesValue, wb: &mut crate::serialize::WriteBuffer) {
        <crate::serialize::DefaultMarshaler<u8> as crate::serialize::Codec<u8>>::marshal(
            &value.field_0,
            wb,
        );
    }
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<ParticipantTeamIndexesValue, crate::serialize::MarshalerError> {
        let field_field_0 =
            <crate::serialize::DefaultMarshaler<u8> as crate::serialize::Codec<u8>>::unmarshal(rb)?;
        Ok(ParticipantTeamIndexesValue {
            field_0: field_field_0,
        })
    }
}
impl crate::serialize::Marshal for ParticipantTeamIndexesValue {
    const MARSHAL_SIZE: usize = <ParticipantTeamIndexesValueMarshaler as crate::serialize::Codec<
        ParticipantTeamIndexesValue,
    >>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut crate::serialize::WriteBuffer) {
        <ParticipantTeamIndexesValueMarshaler as crate::serialize::Codec<
            ParticipantTeamIndexesValue,
        >>::marshal(self, wb);
    }
}
impl crate::serialize::Unmarshal for ParticipantTeamIndexesValue {
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Self, crate::serialize::MarshalerError> {
        <ParticipantTeamIndexesValueMarshaler as crate::serialize::Codec<
            ParticipantTeamIndexesValue,
        >>::unmarshal(rb)
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaidId {
    pub field_0: u64,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct RaidIdMarshaler;
impl crate::serialize::Codec<RaidId> for RaidIdMarshaler {
    const MARSHAL_SIZE: usize =
        <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<u64>>::MARSHAL_SIZE;
    fn marshal(value: &RaidId, wb: &mut crate::serialize::WriteBuffer) {
        <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<u64>>::marshal(
            &value.field_0,
            wb,
        );
    }
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<RaidId, crate::serialize::MarshalerError> {
        let field_field_0 = <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<
            u64,
        >>::unmarshal(rb)?;
        Ok(RaidId {
            field_0: field_field_0,
        })
    }
}
impl crate::serialize::Marshal for RaidId {
    const MARSHAL_SIZE: usize = <RaidIdMarshaler as crate::serialize::Codec<RaidId>>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut crate::serialize::WriteBuffer) {
        <RaidIdMarshaler as crate::serialize::Codec<RaidId>>::marshal(self, wb);
    }
}
impl crate::serialize::Unmarshal for RaidId {
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Self, crate::serialize::MarshalerError> {
        <RaidIdMarshaler as crate::serialize::Codec<RaidId>>::unmarshal(rb)
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Event1Value {
    pub field_0: u32,
    pub field_1: u64,
    pub field_2: u32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Event1ValueMarshaler;
impl crate::serialize::Codec<Event1Value> for Event1ValueMarshaler {
    const MARSHAL_SIZE: usize = <crate::serialize::DefaultMarshaler<
        u32,
    > as crate::serialize::Codec<u32>>::MARSHAL_SIZE
        + <crate::serialize::DefaultMarshaler<
            u64,
        > as crate::serialize::Codec<u64>>::MARSHAL_SIZE
        + <crate::serialize::DefaultMarshaler<
            u32,
        > as crate::serialize::Codec<u32>>::MARSHAL_SIZE;
    fn marshal(value: &Event1Value, wb: &mut crate::serialize::WriteBuffer) {
        <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<u32>>::marshal(
            &value.field_0,
            wb,
        );
        <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<u64>>::marshal(
            &value.field_1,
            wb,
        );
        <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<u32>>::marshal(
            &value.field_2,
            wb,
        );
    }
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Event1Value, crate::serialize::MarshalerError> {
        let field_field_0 = <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<
            u32,
        >>::unmarshal(rb)?;
        let field_field_1 = <crate::serialize::DefaultMarshaler<u64> as crate::serialize::Codec<
            u64,
        >>::unmarshal(rb)?;
        let field_field_2 = <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<
            u32,
        >>::unmarshal(rb)?;
        Ok(Event1Value {
            field_0: field_field_0,
            field_1: field_field_1,
            field_2: field_field_2,
        })
    }
}
impl crate::serialize::Marshal for Event1Value {
    const MARSHAL_SIZE: usize =
        <Event1ValueMarshaler as crate::serialize::Codec<Event1Value>>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut crate::serialize::WriteBuffer) {
        <Event1ValueMarshaler as crate::serialize::Codec<Event1Value>>::marshal(self, wb);
    }
}
impl crate::serialize::Unmarshal for Event1Value {
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Self, crate::serialize::MarshalerError> {
        <Event1ValueMarshaler as crate::serialize::Codec<Event1Value>>::unmarshal(rb)
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlobalAfflictionDataValue {
    pub field_0: (f32, u64, f32, u64, f32, u8, bool),
}
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalAfflictionDataValueMarshaler;
impl crate::serialize::Codec<GlobalAfflictionDataValue> for GlobalAfflictionDataValueMarshaler {
    const MARSHAL_SIZE: usize = <crate::serialize::DefaultMarshaler<(
        f32,
        u64,
        f32,
        u64,
        f32,
        u8,
        bool,
    )> as crate::serialize::Codec<(f32, u64, f32, u64, f32, u8, bool)>>::MARSHAL_SIZE;
    fn marshal(value: &GlobalAfflictionDataValue, wb: &mut crate::serialize::WriteBuffer) {
        <crate::serialize::DefaultMarshaler<
            (f32, u64, f32, u64, f32, u8, bool),
        > as crate::serialize::Codec<
            (f32, u64, f32, u64, f32, u8, bool),
        >>::marshal(&value.field_0, wb);
    }
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<GlobalAfflictionDataValue, crate::serialize::MarshalerError> {
        let field_field_0 = <crate::serialize::DefaultMarshaler<(
            f32,
            u64,
            f32,
            u64,
            f32,
            u8,
            bool,
        )> as crate::serialize::Codec<(f32, u64, f32, u64, f32, u8, bool)>>::unmarshal(
            rb
        )?;
        Ok(GlobalAfflictionDataValue {
            field_0: field_field_0,
        })
    }
}
impl crate::serialize::Marshal for GlobalAfflictionDataValue {
    const MARSHAL_SIZE: usize = <GlobalAfflictionDataValueMarshaler as crate::serialize::Codec<
        GlobalAfflictionDataValue,
    >>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut crate::serialize::WriteBuffer) {
        <GlobalAfflictionDataValueMarshaler as crate::serialize::Codec<
            GlobalAfflictionDataValue,
        >>::marshal(self, wb);
    }
}
impl crate::serialize::Unmarshal for GlobalAfflictionDataValue {
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Self, crate::serialize::MarshalerError> {
        <GlobalAfflictionDataValueMarshaler as crate::serialize::Codec<
            GlobalAfflictionDataValue,
        >>::unmarshal(rb)
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IconsValue {
    pub field_0: u32,
    pub field_1: ::glam::Vec2,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct IconsValueMarshaler;
impl crate::serialize::Codec<IconsValue> for IconsValueMarshaler {
    const MARSHAL_SIZE: usize =
        <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<u32>>::MARSHAL_SIZE
            + <crate::serialize::DefaultMarshaler<::glam::Vec2> as crate::serialize::Codec<
                ::glam::Vec2,
            >>::MARSHAL_SIZE;
    fn marshal(value: &IconsValue, wb: &mut crate::serialize::WriteBuffer) {
        <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<u32>>::marshal(
            &value.field_0,
            wb,
        );
        <crate::serialize::DefaultMarshaler<::glam::Vec2> as crate::serialize::Codec<
            ::glam::Vec2,
        >>::marshal(&value.field_1, wb);
    }
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<IconsValue, crate::serialize::MarshalerError> {
        let field_field_0 = <crate::serialize::DefaultMarshaler<u32> as crate::serialize::Codec<
            u32,
        >>::unmarshal(rb)?;
        let field_field_1 =
            <crate::serialize::DefaultMarshaler<::glam::Vec2> as crate::serialize::Codec<
                ::glam::Vec2,
            >>::unmarshal(rb)?;
        Ok(IconsValue {
            field_0: field_field_0,
            field_1: field_field_1,
        })
    }
}
impl crate::serialize::Marshal for IconsValue {
    const MARSHAL_SIZE: usize =
        <IconsValueMarshaler as crate::serialize::Codec<IconsValue>>::MARSHAL_SIZE;
    fn marshal(&self, wb: &mut crate::serialize::WriteBuffer) {
        <IconsValueMarshaler as crate::serialize::Codec<IconsValue>>::marshal(self, wb);
    }
}
impl crate::serialize::Unmarshal for IconsValue {
    fn unmarshal(
        rb: &mut crate::serialize::ReadBuffer,
    ) -> Result<Self, crate::serialize::MarshalerError> {
        <IconsValueMarshaler as crate::serialize::Codec<IconsValue>>::unmarshal(rb)
    }
}
/// One participant: index, character id, optional VLQ id, entity.
pub type ParticipantCharacterId = (u8, u64, Option<u64>, crate::EntityRef);

/// `participantCharacterIds`, in its generated wire shape.
pub type ParticipantCharacterIdsField = crate::serialize::ReplicatedFieldHandler<
    Vec<ParticipantCharacterId>,
    crate::serialize::SequenceCodec<
        crate::serialize::TupleCodec<(
            crate::serialize::DefaultMarshaler<u8>,
            crate::serialize::DefaultMarshaler<u64>,
            crate::serialize::OptionalCodec<crate::serialize::VlqU64Marshaler>,
            crate::serialize::DefaultMarshaler<crate::EntityRef>,
        )>,
    >,
>;

#[replicated_state]
#[az_rtti("78EA6535-BB84-4D6A-A5A3-747AF2F5167C")]
#[type_registry(2343u32)]
#[derive(Debug, Clone, Default)]
pub struct GameModeReplicatedState {
    pub cur_script_state_id: crate::serialize::ReplicatedFieldHandler<i8>,
    pub cur_script_id: crate::serialize::ReplicatedFieldHandler<Crc32>,
    pub spawned_entity_ids_by_spawner_id:
        crate::serialize::ReplicatedContainer<crate::serialize::IndexMap<Crc32, EntityId>>,
    pub game_mode_id: crate::serialize::ReplicatedFieldHandler<u32>,
    pub game_mode_map_id: crate::serialize::ReplicatedFieldHandler<u32>,
    pub participant_facet_refs: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<ParticipantFacetRefsValue>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        ParticipantFacetRefsValueMarshaler,
    >,
    pub participant_statuses: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<[u8; 1]>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        crate::serialize::DefaultMarshaler<[u8; 1]>,
    >,
    pub participant_team_indexes: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<ParticipantTeamIndexesValue>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        ParticipantTeamIndexesValueMarshaler,
    >,
    pub participant_character_ids: ParticipantCharacterIdsField,
    pub raid_ids: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<RaidId>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        RaidIdMarshaler,
    >,
    pub values: SyncedTimerMap,
    pub synced_timers: SyncedTimerMap,
    pub event_1: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_10: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_11: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_12: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_13: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_14: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_15: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_16: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_17: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub event_1_18: crate::serialize::ReplicatedFieldHandler<Event1Value, Event1ValueMarshaler>,
    pub global_affliction_data: crate::serialize::ReplicatedFieldHandler<
        GlobalAfflictionDataValue,
        GlobalAfflictionDataValueMarshaler,
    >,
    pub map_origin: crate::serialize::ReplicatedFieldHandler<u32>,
    pub tile_size_meters: crate::serialize::ReplicatedFieldHandler<u8>,
    pub map_size_in_tiles: crate::serialize::ReplicatedFieldHandler<u16>,
    pub tile_ui_filename_id_and_rotation: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<ParticipantTeamIndexesValue>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        ParticipantTeamIndexesValueMarshaler,
    >,
    pub tile_visited: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<ParticipantTeamIndexesValue>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        ParticipantTeamIndexesValueMarshaler,
    >,
    pub icons: crate::serialize::ReplicatedContainer<
        ::std::vec::Vec<IconsValue>,
        { crate::serialize::WIRE_VEC_CAP },
        crate::serialize::DefaultMarshaler<crate::serialize::VlqU64>,
        IconsValueMarshaler,
    >,
    pub linked_mode: crate::serialize::ReplicatedFieldHandler<bool>,
}

#[cfg(test)]
mod tests {
    use super::GameModeReplicatedState;
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer, VlqU64};
    use crate::types::Crc32;

    /// A script state update with a timer snapshot, in the layout live
    /// sends. The values are made up.
    const SCRIPT_STATE_AND_TIMER_SNAPSHOT: &[u8] = &[
        0x01, // group 0 dirty
        0x81, // field 0 (curScriptStateId), more masks follow
        0x07, // script state id 7, one byte
        0x10, // field 11 (syncedTimers)
        0x00, // snapshot
        0x01, 0x05, // last-modified sequence 5
        0x01, // one entry
        0x11, 0x22, 0x33, 0x44, // CRC key
        0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // timer, fixed u64
    ];

    /// Two timer changes of `values`, in the layout live sends: each a CRC
    /// key, a sequence and a VLQ timer. The values are made up.
    const VALUE_DELTAS: &[u8] = &[
        0x01, // group 0 dirty
        0x80, // no field of 0..=6, more masks follow
        0x08, // field 10 (values)
        0x02, // two changes
        0x03, // both live
        0xaa, 0xbb, 0xcc, 0xdd, // CRC key
        0x01, 0x10, // sequence 16
        0xbb, 0x03, // timer 251, VLQ
        0x01, 0x02, 0x03, 0x04, // CRC key
        0x01, 0x11, // sequence 17
        0x05, // timer 5, VLQ
    ];

    /// Decodes through the type-index registry, as a bundle walk does, and
    /// returns the state and the bytes consumed.
    fn decode(bytes: &[u8]) -> (GameModeReplicatedState, usize) {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, bytes);
        let fragment = FragmentTypeInfo::TypeIndex(2343)
            .decode_contents(&mut rb)
            .expect("game mode contents decode");
        let state = fragment
            .downcast_ref::<GameModeReplicatedState>()
            .expect("type index 2343 is GameModeReplicatedState")
            .clone();
        (state, rb.position())
    }

    #[test]
    fn the_inherited_script_fields_come_first() {
        let (state, consumed) = decode(SCRIPT_STATE_AND_TIMER_SNAPSHOT);
        assert_eq!(consumed, SCRIPT_STATE_AND_TIMER_SNAPSHOT.len());
        assert_eq!(state.cur_script_state_id.value(), Some(&7));
        assert_eq!(state.game_mode_id.value(), None);
        let timers = state.synced_timers.values();
        assert_eq!(timers.len(), 1);
        assert_eq!(
            timers.get(&Crc32::new(0x1122_3344)),
            Some(&VlqU64::new(0x0102_0304_0506))
        );
    }

    #[test]
    fn value_deltas_carry_vlq_timers() {
        let (state, consumed) = decode(VALUE_DELTAS);
        assert_eq!(consumed, VALUE_DELTAS.len());
        let changes = state.values.current_changes();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].key(), &Crc32::new(0xaabb_ccdd));
        assert_eq!(changes[0].value(), Some(&VlqU64::new(251)));
        assert_eq!(changes[1].key(), &Crc32::new(0x0102_0304));
        assert_eq!(changes[1].value(), Some(&VlqU64::new(5)));
    }
}
