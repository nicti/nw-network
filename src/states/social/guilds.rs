//! Guild roster, invite, crest, and territory influence replication.
//!
//! `MB::GuildsComponentReplicatedState` (type 3147) is written by hand for
//! `guildInvites`: the schema registers it as a plain field,
//! `vec<composite<u8, fixed-bytes-16, optional<vlq-u64>, invite>>` (the
//! shape of one container delta change), but live writes a replicated
//! container (mode, sequence, count), keyed by a 16-byte id with a
//! [`GuildInviteStateData`] per entry. Read as a list, the container's mode
//! byte was taken for an empty list and the rest of the bundle was read from
//! the wrong offset. The other fields keep the generated shapes.

use crate::Marshaler;
use crate::generated::states::guilds_component_replicated_state as generated;
use crate::serialize::{
    DefaultMarshaler, IndexMap, ReplicatedContainer, ReplicatedFieldHandler, VlqU64, WIRE_VEC_CAP,
};
use crate::source::{GuildId, WallClockTimePoint};
use crate::{EntityRef, az_rtti, replicated_state, type_registry};

#[derive(Debug, Clone, Copy, Default, PartialEq, Marshaler)]
pub struct GuildCrestColor {
    pub rgba: [f32; 4],
}

#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct GuildCrestData {
    pub foreground_id: u16,
    pub foreground_color: GuildCrestColor,
    pub background_id: u16,
    pub background_color: GuildCrestColor,
}

#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct GuildPlayerIdentification {
    pub player_name: String,
    pub display_name: String,
    pub status: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct ReplicatedGuildInfluence {
    pub territory_id: u16,
    pub influence: f32,
    pub next_decay_time: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct EligibleTerritoryWar {
    pub war_id: u64,
    pub war_state: u8,
    pub territory_id: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct GuildInviteSenderData {
    pub rank: u32,
    pub display_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Marshaler)]
pub struct GuildInviteStateData {
    pub guild_name: String,
    pub guild_crest_data: GuildCrestData,
    pub sender_name: String,
    pub sender_data: GuildInviteSenderData,
    pub sent_time: u64,
}

/// `guildInvites`: a replicated container keyed by a 16-byte id.
pub type GuildInvites = ReplicatedContainer<
    IndexMap<[u8; 16], GuildInviteStateData>,
    { WIRE_VEC_CAP },
    DefaultMarshaler<[u8; 16]>,
    DefaultMarshaler<GuildInviteStateData>,
>;

#[replicated_state]
#[az_rtti("F5398BCB-8AF9-47C7-84C6-B8AFBF249F34")]
#[type_registry(3147u32)]
#[derive(Debug, Clone, Default)]
pub struct GuildsComponentReplicatedState {
    pub guild_id: ReplicatedFieldHandler<GuildId>,
    pub guild_rank: ReplicatedFieldHandler<u16>,
    #[replicated_state(group = 1u32)]
    pub using_gm_commands: ReplicatedFieldHandler<bool>,
    #[replicated_state(group = 1u32)]
    pub guild_forced_rename: ReplicatedFieldHandler<bool>,
    #[replicated_state(group = 1u32)]
    pub guild_message_of_the_day: ReplicatedFieldHandler<String>,
    #[replicated_state(group = 1u32)]
    pub guild_message_of_the_day_author: ReplicatedFieldHandler<EntityRef>,
    #[replicated_state(group = 1u32)]
    pub guild_message_of_the_day_time: ReplicatedFieldHandler<WallClockTimePoint>,
    #[replicated_state(group = 1u32)]
    pub guild_siege_window: ReplicatedFieldHandler<f32>,
    #[replicated_state(group = 1u32)]
    pub guild_last_siege_window_set: ReplicatedFieldHandler<WallClockTimePoint>,
    #[replicated_state(group = 1u32)]
    pub guild_treasury_current_funds: ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 1u32)]
    pub guild_treasury_daily_withdrawal_limit: ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 1u32)]
    pub last_guild_leave_time: ReplicatedFieldHandler<WallClockTimePoint>,
    #[replicated_state(group = 1u32)]
    pub guild_treasury_withdrawn_today: ReplicatedFieldHandler<u64>,
    #[replicated_state(group = 1u32)]
    pub is_guild_master_seat_overthrowable: ReplicatedFieldHandler<bool>,
    #[replicated_state(group = 1u32)]
    pub is_guild_master_seat_vacant: ReplicatedFieldHandler<bool>,
    #[replicated_state(group = 1u32)]
    pub governor_leave_guild_time: ReplicatedFieldHandler<WallClockTimePoint>,
    #[replicated_state(group = 1u32)]
    pub guild_members_character_id_string: ReplicatedContainer<
        Vec<generated::GuildMembersCharacterIdStringValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        generated::GuildMembersCharacterIdStringValueMarshaler,
    >,
    #[replicated_state(group = 1u32)]
    pub guild_members_rank: ReplicatedContainer<
        Vec<generated::GuildMembersRankValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        generated::GuildMembersRankValueMarshaler,
    >,
    #[replicated_state(group = 1u32)]
    pub guild_members_online_status: ReplicatedContainer<
        Vec<u8>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        DefaultMarshaler<u8>,
    >,
    #[replicated_state(group = 1u32)]
    pub guild_members_last_online_time: ReplicatedContainer<
        Vec<u64>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        DefaultMarshaler<u64>,
    >,
    #[replicated_state(group = 1u32)]
    pub guild_member_was_kicked: ReplicatedFieldHandler<bool>,
    #[replicated_state(group = 1u32)]
    pub guild_influence_data: ReplicatedContainer<
        Vec<generated::GuildInfluenceDataValue>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        generated::GuildInfluenceDataValueMarshaler,
    >,
    #[replicated_state(group = 1u32)]
    pub eligible_territory_wars: ReplicatedContainer<
        Vec<generated::WarDeclarationEvent>,
        { WIRE_VEC_CAP },
        DefaultMarshaler<VlqU64>,
        generated::WarDeclarationEventMarshaler,
    >,
    #[replicated_state(group = 1u32)]
    pub guild_war_lottery_deadline: ReplicatedFieldHandler<WallClockTimePoint>,
    #[replicated_state(group = 1u32)]
    pub guild_invites: GuildInvites,
    #[replicated_state(group = 1u32)]
    pub number_of_outstanding_invites: ReplicatedFieldHandler<u64>,
}

#[cfg(test)]
mod tests {
    use super::{GuildInviteStateData, GuildsComponentReplicatedState};
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// A guild invite update in the layout live sends: group 1, field 22
    /// (`guildInvites`), a container snapshot with one entry. The values are
    /// made up.
    const GUILD_INVITE: &[u8] = &[
        0x02, // group 1 dirty
        0x80, 0x80, 0x80, 0x02, // field 22 (guild invites), no more masks
        0x00, // snapshot
        0x01, 0x05, // sequence 5
        0x01, // one entry
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, //
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, // key
        0x03, b'A', b'B', b'C', // guild name
        0x00, 0x07, // crest foreground id
        0x3f, 0x80, 0, 0, 0x3f, 0x80, 0, 0, 0x3f, 0x80, 0, 0, 0x3f, 0x80, 0, 0, // colour
        0x00, 0x09, // crest background id
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x3f, 0x80, 0, 0, // colour
        0x02, b'x', b'y', // sender name
        0, 0, 0, 0x03, // sender rank
        0x01, b'z', // sender display name
        0, 0, 0, 0, 0, 0, 0, 0x2a, // sent time
    ];

    #[test]
    fn guild_invites_are_a_container() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, GUILD_INVITE);
        let fragment = FragmentTypeInfo::TypeIndex(3147)
            .decode_contents(&mut rb)
            .expect("guilds contents decode");
        assert_eq!(rb.position(), GUILD_INVITE.len());
        let state = fragment
            .downcast_ref::<GuildsComponentReplicatedState>()
            .expect("type index 3147 is GuildsComponentReplicatedState");
        let key: [u8; 16] = std::array::from_fn(|i| 0x10 + i as u8);
        let invite: GuildInviteStateData = state
            .guild_invites
            .clone()
            .into_values()
            .get(&key)
            .cloned()
            .expect("the invite is keyed by its id");
        assert_eq!(invite.guild_name, "ABC");
        assert_eq!(invite.guild_crest_data.foreground_id, 7);
        assert_eq!(invite.guild_crest_data.background_id, 9);
        assert_eq!(invite.sender_name, "xy");
        assert_eq!(invite.sender_data.rank, 3);
        assert_eq!(invite.sender_data.display_name, "z");
        assert_eq!(invite.sent_time, 42);
    }
}
