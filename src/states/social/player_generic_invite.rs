//! Generic player invite replication.

use crate::Marshaler;
use crate::{az_rtti, replicated_state, type_registry};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerGenericInviteParticipants;

impl crate::Marshal for PlayerGenericInviteParticipants {
    fn marshal(&self, _wb: &mut crate::WriteBuffer) {}
}

impl crate::Unmarshal for PlayerGenericInviteParticipants {
    fn unmarshal(_rb: &mut crate::ReadBuffer) -> Result<Self, crate::MarshalerError> {
        Ok(Self)
    }
}

/// `m_inviteParticipants`, in live's layout: three lists.
///
/// The schema's wire shape for the field is
/// `composite<vec<composite<entity-ref, composite<u32, string>>>,
/// vec<entity-ref>, vec<composite<entity-ref, u32>>>`, but its nested type
/// shape lists only the first list, so the generated state read one count.
/// Live writes all three: an invite with no participants is `00 00 00`, and
/// read with one count every invite fragment ended two bytes early and the
/// rest of its bundle was read from the wrong offset. The lists' meanings
/// are not known; they keep neutral names.
#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct GenericInviteParticipantState {
    pub field_0: Vec<(crate::EntityRef, (u32, String))>,
    pub field_1: Vec<crate::EntityRef>,
    pub field_2: Vec<(crate::EntityRef, u32)>,
}

/// `Javelin::PlayerGenericInviteReplicatedState` (type 3408), written by
/// hand for [`GenericInviteParticipantState`]; the other fields keep the
/// generated shapes.
#[replicated_state]
#[az_rtti("A02118E0-94AB-4945-B421-1FABFB0C4806")]
#[type_registry(3408u32)]
#[derive(Debug, Clone, Default)]
pub struct PlayerGenericInviteReplicatedState {
    pub invite_id: crate::serialize::ReplicatedFieldHandler<[u8; 16]>,
    #[replicated_state(group = 1u32)]
    pub activity_crc: crate::serialize::ReplicatedFieldHandler<u32>,
    #[replicated_state(group = 1u32)]
    pub forward_type: crate::serialize::ReplicatedFieldHandler<u8>,
    #[replicated_state(group = 1u32)]
    pub invite_participants:
        crate::serialize::ReplicatedFieldHandler<GenericInviteParticipantState>,
    #[replicated_state(group = 1u32)]
    pub expiry_time_point: crate::serialize::ReplicatedFieldHandler<crate::source::TimePoint>,
}

#[cfg(test)]
mod tests {
    use super::{GenericInviteParticipantState, PlayerGenericInviteReplicatedState};
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// An invite in the layout live sends: the id, then group 1 with three
    /// empty participant lists ahead of the expiry. The values are made up.
    const INVITE: &[u8] = &[
        0x03, // groups 0 and 1 dirty
        0x01, // group 0: field 0 (invite id)
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, //
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, // invite id
        0x0f, // group 1: fields 0..=3
        0x00, 0x00, 0x00, 0x2a, // activity crc (big-endian)
        0x02, // forward type
        0x00, 0x00, 0x00, // participants: three empty lists
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // expiry time point
    ];

    #[test]
    fn invite_participants_are_three_lists() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, INVITE);
        let fragment = FragmentTypeInfo::TypeIndex(3408)
            .decode_contents(&mut rb)
            .expect("player generic invite contents decode");
        assert_eq!(rb.position(), INVITE.len());
        let state = fragment
            .downcast_ref::<PlayerGenericInviteReplicatedState>()
            .expect("type index 3408 is PlayerGenericInviteReplicatedState");
        assert_eq!(
            state.invite_id.value(),
            Some(&std::array::from_fn(|i| 0x10 + i as u8))
        );
        assert_eq!(state.activity_crc.value(), Some(&0x2a));
        assert_eq!(state.forward_type.value(), Some(&2));
        assert_eq!(
            state.invite_participants.value(),
            Some(&GenericInviteParticipantState::default())
        );
        assert!(state.expiry_time_point.value().is_some());
    }
}
