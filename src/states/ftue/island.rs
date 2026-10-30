//! FTUE island phase and completion-state replication.
//!
//! Written by hand rather than generated: the generator emitted the three
//! region fields as fixed `u32` where the client registers them with
//! `VlqU32Marshaler` (the schema's `wireShape` is `vlq-u32`), so a live
//! arrival update decoded 8 bytes long and broke the rest of its bundle.

use crate::serialize::{ReplicatedFieldHandler, VlqU32};
use crate::source::RemoteServerGDERef;
use crate::{az_rtti, replicated_state, type_registry};

/// The FTUE island a player is placed on: its owner and its region.
#[replicated_state]
#[derive(Debug, Clone, Default)]
#[az_rtti("59B9807C-CE02-4611-81A3-F0DDCAC27900")]
#[type_registry(3194)]
pub struct FtueIslandComponentReplicatedState {
    pub player_gde: ReplicatedFieldHandler<RemoteServerGDERef>,
    pub region_x: ReplicatedFieldHandler<VlqU32>,
    pub region_y: ReplicatedFieldHandler<VlqU32>,
    pub region_size: ReplicatedFieldHandler<VlqU32>,
}

#[cfg(test)]
mod tests {
    use super::FtueIslandComponentReplicatedState;
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer, ReplicatedFieldHandler, VlqU32};

    /// The island's state as live sends it on arrival: every field set, the
    /// region fields as VLQ. Ids and values are made up; the layout is the
    /// captured one (30 bytes).
    const ARRIVAL: &[u8] = &[
        0x01, // group 0 dirty
        0x0f, // fields 0..=3
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, // player GDE ref: 16-byte id
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, //
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // and its u64
        0x07, // region x 7, VLQ
        0x00, // region y 0
        0x80, 0x10, // region size 1024, VLQ
    ];

    #[test]
    fn an_arrival_update_decodes_to_its_own_length() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, ARRIVAL);
        let fragment = FragmentTypeInfo::TypeIndex(3194)
            .decode_contents(&mut rb)
            .expect("island contents decode");
        assert_eq!(rb.position(), ARRIVAL.len());
        let state = fragment
            .downcast_ref::<FtueIslandComponentReplicatedState>()
            .expect("type index 3194 is FtueIslandComponentReplicatedState");
        assert!(state.player_gde.value().is_some());
        let region = |field: &ReplicatedFieldHandler<VlqU32>| field.value().map(|v| v.get());
        assert_eq!(region(&state.region_x), Some(7));
        assert_eq!(region(&state.region_y), Some(0));
        assert_eq!(region(&state.region_size), Some(1024));
    }
}
