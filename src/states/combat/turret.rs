//! `MB::TurretReplicatedState` (type 4276), written by hand.
//!
//! Generated otherwise, except `initialOrientation`: the schema gives it the
//! shape `quat` and the generator emitted a plain `Quat` (four `f32`, 16
//! bytes), but live writes it as a smallest-three quantized quaternion
//! ([`QuatSmallestThreeQuantizedMarshaler`]: a flag byte, then one byte per
//! carried component that is not zero; 2 bytes for a turret turned about
//! its up axis). Read as 16 bytes, every turret snapshot overran by 14 bytes
//! and failed at `isOverheated` (invalid enum discriminant), and the rest of
//! its bundle was lost. The other fields keep the generated shapes and
//! order.
//!
//! [`QuatSmallestThreeQuantizedMarshaler`]:
//! crate::serialize::QuatSmallestThreeQuantizedMarshaler

use ::nw_network::{az_rtti, replicated_state, type_registry};

#[replicated_state]
#[az_rtti("A9F8D205-2922-41E1-B8C2-0DFAF1CC8475")]
#[type_registry(4276u32)]
#[derive(Debug, Clone, Default)]
pub struct TurretReplicatedState {
    pub yaw: ::nw_network::serialize::ReplicatedFieldHandler<
        f32,
        ::nw_network::serialize::HalfF32Marshaler,
    >,
    pub pitch: ::nw_network::serialize::ReplicatedFieldHandler<
        f32,
        ::nw_network::serialize::HalfF32Marshaler,
    >,
    pub heat: ::nw_network::serialize::ReplicatedFieldHandler<
        f32,
        ::nw_network::serialize::HalfF32Marshaler,
    >,
    pub initial_orientation: ::nw_network::serialize::ReplicatedFieldHandler<
        ::glam::Quat,
        ::nw_network::serialize::QuatSmallestThreeQuantizedMarshaler,
    >,
    pub interactor_gde_ref:
        ::nw_network::serialize::ReplicatedFieldHandler<::nw_network::source::RemoteServerGDERef>,
    pub is_overheated: ::nw_network::serialize::ReplicatedFieldHandler<bool>,
    pub is_interaction_blocked: ::nw_network::serialize::ReplicatedFieldHandler<bool>,
}

#[cfg(test)]
mod tests {
    use super::TurretReplicatedState;
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// A turret snapshot in the layout live sends: every field, the initial
    /// orientation as a smallest-three quaternion. The values are made up.
    const SNAPSHOT: &[u8] = &[
        0x01, // group 0 dirty
        0x7f, // fields 0..=6
        0x00, 0x00, // yaw 0, half
        0x3c, 0x00, // pitch 1, half
        0x00, 0x00, // heat 0, half
        0x66, // orientation: w omitted (index 3), x and y zero
        0xb4, // z, one byte
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, //
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, // interactor GDE ref id
        0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, // interactor GDE ref, u64
        0x01, // overheated
        0x00, // interaction not blocked
    ];

    #[test]
    fn initial_orientation_is_a_smallest_three_quaternion() {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, SNAPSHOT);
        let fragment = FragmentTypeInfo::TypeIndex(4276)
            .decode_contents(&mut rb)
            .expect("turret contents decode");
        assert_eq!(rb.position(), SNAPSHOT.len());
        let state = fragment
            .downcast_ref::<TurretReplicatedState>()
            .expect("type index 4276 is TurretReplicatedState");
        let q = *state.initial_orientation.value().expect("orientation set");
        assert_eq!((q.x, q.y), (0.0, 0.0));
        assert!(q.z > 0.0 && q.w > 0.0, "{q:?}");
        assert!((q.length() - 1.0).abs() < 1e-3, "{q:?}");
        assert_eq!(state.pitch.value(), Some(&1.0));
        assert_eq!(state.is_overheated.value(), Some(&true));
        assert_eq!(state.is_interaction_blocked.value(), Some(&false));
    }
}
