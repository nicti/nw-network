//! Projectile transform, lifetime, and hit-data replication.
//!
//! Written by hand rather than generated: the generator reads the nested
//! value layout of a field (`AZ::Vector3` is four floats natively, a VLQ
//! field's value is a plain `u32`) instead of the field's wire shape, so it
//! emitted `spawnVelocity` and `magnetRootOffset` as four halves (8 bytes)
//! where `Vec3CompMarshaler` writes three (6), and `rangedAttackIndex` and
//! `ammoIdCRC` as fixed `u32` where `VlqU32Marshaler` is used. A live
//! projectile spawn decoded 14 bytes short and broke the rest of its bundle.
//! The field order and wire shapes below are the schema's (`wireShape` of
//! each registered field).

use glam::Vec3;

use crate::Marshaler;
use crate::serialize::{
    HalfF32Marshaler, PackedPositionMarshaller, ReplicatedContainer, ReplicatedFieldHandler,
    Vec3CompMarshaler, VlqU32,
};
use crate::{az_rtti, replicated_state, type_registry};

/// Value stored by [`ProjectileReplicatedState::piercing_hits`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct PiercingHitData {
    pub target_entity: u64,
    pub flag: u8,
    pub volume_index: u16,
}

/// Height range of [`ProjectileReplicatedState::position_anchor`]: -100 to 2000.
pub type ProjectilePositionMarshaller = PackedPositionMarshaller<0xc2c8_0000, 0x44fa_0000>;

/// Projectile ranged-attack replicated state.
#[replicated_state]
#[derive(Debug, Clone, Default)]
#[az_rtti("39B4C919-3A6D-46B5-92D0-3B4ACB284B1D")]
#[type_registry(16)]
pub struct ProjectileReplicatedState {
    pub position_anchor: ReplicatedFieldHandler<Vec3, ProjectilePositionMarshaller>,
    pub anchor_height_delta: ReplicatedFieldHandler<f32, HalfF32Marshaler>,
    pub spawn_velocity: ReplicatedFieldHandler<Vec3, Vec3CompMarshaler>,
    pub gravity_override: ReplicatedFieldHandler<f32, HalfF32Marshaler>,
    pub time_anchor_microseconds: ReplicatedFieldHandler<u64>,
    pub time_offset_start_accel_milliseconds: ReplicatedFieldHandler<u16>,
    pub time_offset_bounce_milliseconds: ReplicatedFieldHandler<u16>,
    pub collided_with_gde_id: ReplicatedFieldHandler<u64>,
    pub collided_with_hit_volume_index: ReplicatedFieldHandler<u8>,
    pub flags: ReplicatedFieldHandler<u8>,
    pub owner: ReplicatedFieldHandler<u64>,
    pub ranged_attack_index: ReplicatedFieldHandler<VlqU32>,
    pub weapon_effect_crc: ReplicatedFieldHandler<u32>,
    pub ammo_id_crc: ReplicatedFieldHandler<VlqU32>,
    pub damage_table_id: ReplicatedFieldHandler<u32>,
    pub damage_table_row_index: ReplicatedFieldHandler<u16>,
    pub piercing_hits: ReplicatedContainer<Vec<PiercingHitData>>,
    pub magnet_entity_id: ReplicatedFieldHandler<u64>,
    pub magnet_root_offset: ReplicatedFieldHandler<Vec3, Vec3CompMarshaler>,
    pub combat_settings_exp_idx: ReplicatedFieldHandler<u8>,
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::ProjectileReplicatedState;
    use crate::hub::FragmentTypeInfo;
    use crate::serialize::{CARRIER_ENDIAN, ReadBuffer};

    /// A spawn update in the shape live sends: position anchor and spawn
    /// velocity, owner, ranged attack index, weapon effect CRC, damage
    /// table id and row. The values are made up; the layout is the
    /// captured one (40 bytes).
    const SPAWN: &[u8] = &[
        0x01, // group 0 dirty
        0x85, // fields 0 and 2, more masks follow
        0x44, 0x7a, 0x00, 0x00, // position anchor x = 1000.0
        0x44, 0xfa, 0x00, 0x00, // y = 2000.0
        0x80, 0x00, // packed height
        0x3c, 0x00, 0x40, 0x00, 0xc2, 0x00, // spawn velocity (1, 2, -3), three halves
        0xb8, // fields 10, 11, 12, more masks follow
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // owner
        0xbb, 0x03, // ranged attack index 251, VLQ
        0xde, 0xad, 0xbe, 0xef, // weapon effect CRC
        0x03, // fields 14 and 15
        0x11, 0x22, 0x33, 0x44, // damage table id
        0x00, 0x42, // damage table row index
    ];

    /// The fields no capture has shown yet: ammo id CRC (VLQ) and magnet
    /// root offset (three halves).
    const AMMO_AND_MAGNET: &[u8] = &[
        0x01, // group 0 dirty
        0x80, // no field of 0..=6, more masks follow
        0xc0, // field 13, more masks follow
        0x05, // ammo id CRC 5, VLQ
        0x10, // field 18
        0x3c, 0x00, 0x00, 0x00, 0xbc, 0x00, // magnet root offset (1, 0, -1)
    ];

    /// Decodes through the type-index registry, as a bundle walk does, and
    /// returns the state and the bytes consumed.
    fn decode(bytes: &[u8]) -> (ProjectileReplicatedState, usize) {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, bytes);
        let fragment = FragmentTypeInfo::TypeIndex(16)
            .decode_contents(&mut rb)
            .expect("projectile contents decode");
        let state = fragment
            .downcast_ref::<ProjectileReplicatedState>()
            .expect("type index 16 is ProjectileReplicatedState")
            .clone();
        (state, rb.position())
    }

    #[test]
    fn a_spawn_update_decodes_to_its_own_length() {
        let (state, consumed) = decode(SPAWN);
        assert_eq!(consumed, SPAWN.len());
        let anchor = state.position_anchor.value().expect("anchor set");
        assert_eq!((anchor.x, anchor.y), (1000.0, 2000.0));
        assert_eq!(
            state.spawn_velocity.value(),
            Some(&Vec3::new(1.0, 2.0, -3.0))
        );
        assert_eq!(state.owner.value(), Some(&0x0102_0304_0506_0708));
        assert_eq!(
            state.ranged_attack_index.value().map(|v| v.get()),
            Some(251)
        );
        assert_eq!(state.weapon_effect_crc.value(), Some(&0xdead_beef));
        assert_eq!(state.damage_table_id.value(), Some(&0x1122_3344));
        assert_eq!(state.damage_table_row_index.value(), Some(&0x42));
        assert_eq!(state.time_anchor_microseconds.value(), None);
        assert_eq!(state.ammo_id_crc.value(), None);
    }

    #[test]
    fn ammo_crc_and_magnet_offset_decode_to_their_own_length() {
        let (state, consumed) = decode(AMMO_AND_MAGNET);
        assert_eq!(consumed, AMMO_AND_MAGNET.len());
        assert_eq!(state.ammo_id_crc.value().map(|v| v.get()), Some(5));
        assert_eq!(
            state.magnet_root_offset.value(),
            Some(&Vec3::new(1.0, 0.0, -1.0))
        );
    }
}
