//! Concrete network message payloads.

pub mod actor_mover;
#[cfg(test)]
mod chat;
pub mod client_input;
#[cfg(test)]
mod cutscene;
#[cfg(test)]
mod entitlement;
#[cfg(test)]
mod objectives;
pub mod registration;
#[cfg(test)]
mod request_id;
#[cfg(test)]
mod server_context;

pub use crate::generated_messages::CheckMovementStatusMsg;
pub use registration::{
    AuthToken, ClientVersionTokenMap, ConnTicket, EntityId, ImpersonatedValues, LoginToken,
    RegistrationRequestV3Msg, TypeIndexCrc,
};
