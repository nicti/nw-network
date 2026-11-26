//! Social, chat, group, guild, and invite replicated state modules.

pub mod chat;
pub mod group_data;
pub mod groups;
pub mod guilds;
pub mod player_generic_invite;
pub mod state;
pub mod temporary_affiliation;

pub use crate::generated::states::{
    MagicComponentReplicatedState, OwnershipComponentReplicatedState,
};
pub use crate::types::{TemporaryAffiliationRelationship, TemporaryAffiliationType};
pub use chat::{ChatMuteEntry, ChatMutes, ChatReplicatedState};
pub use group_data::GroupDataComponentReplicatedState;
pub use groups::{
    GameInviteData, GroupFinderApplicationData, GroupInviteData, GroupMemberHouseId,
    GroupsComponentReplicatedState,
};
pub use guilds::{
    EligibleTerritoryWar, GuildCrestColor, GuildCrestData, GuildInviteSenderData,
    GuildInviteStateData, GuildInvites, GuildPlayerIdentification, GuildsComponentReplicatedState,
    ReplicatedGuildInfluence,
};
pub use player_generic_invite::{
    GenericInviteParticipantState, PlayerGenericInviteParticipants,
    PlayerGenericInviteReplicatedState,
};
pub use state::{SocialCollectionsSnapshot, SocialReplicatedState};
pub use temporary_affiliation::{
    MAX_TEMPORARY_AFFILIATION_CHANGES, TemporaryAffiliation, TemporaryAffiliationReplicatedState,
};
