//! The 16-byte component pair that opens a message addressed to one
//! component of the player entity.
//!
//! The pair is `(E + c, E)`: `E` is the little-endian `u64` of the first
//! eight bytes of the actor id (the 16 bytes of the add-entry message), `c`
//! is a constant per component, and each `u64` is written big-endian, the
//! carrier's order.

/// A component of the player entity that has a pair constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    GridTracking,
    PlayerComponent,
    ActionList,
    PlayerSettings,
}

impl Component {
    /// The constant `c`.
    #[must_use]
    pub const fn constant(self) -> u64 {
        match self {
            Self::GridTracking => 0x3946_bc0c_0a40_83da,
            Self::PlayerComponent => 0xdf0c_e8c9_8d27_619d,
            Self::ActionList => 0x3c59_1a4e_a3c2_ad75,
            Self::PlayerSettings => 0x1224_1b80_cabb_cb96,
        }
    }
}

/// The pair for `component` of the entity with `actor_id`: `E + c`
/// (wrapping) then `E`, each big-endian.
#[must_use]
pub fn pair(actor_id: [u8; 16], component: Component) -> [u8; 16] {
    let e = u64::from_le_bytes(actor_id[..8].try_into().expect("8 of 16 bytes"));
    let mut out = [0; 16];
    out[..8].copy_from_slice(&e.wrapping_add(component.constant()).to_be_bytes());
    out[8..].copy_from_slice(&e.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up id: its first eight bytes are `E` little-endian.
    const ID: [u8; 16] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11,
        0x22,
    ];
    const E: u64 = 0x0807_0605_0403_0201;

    #[test]
    fn the_constants() {
        assert_eq!(Component::GridTracking.constant(), 0x3946bc0c0a4083da);
        assert_eq!(Component::PlayerComponent.constant(), 0xdf0ce8c98d27619d);
        assert_eq!(Component::ActionList.constant(), 0x3c591a4ea3c2ad75);
        assert_eq!(Component::PlayerSettings.constant(), 0x12241b80cabbcb96);
    }

    #[test]
    fn the_pair_is_e_plus_c_then_e_big_endian() {
        for component in [
            Component::GridTracking,
            Component::PlayerComponent,
            Component::ActionList,
            Component::PlayerSettings,
        ] {
            let out = pair(ID, component);
            let c = component.constant();
            assert_eq!(out[..8], E.wrapping_add(c).to_be_bytes());
            assert_eq!(out[8..], E.to_be_bytes());
        }
    }

    #[test]
    fn the_second_half_is_the_id_bytes_reversed() {
        let out = pair(ID, Component::GridTracking);
        assert_eq!(out[8..], [8, 7, 6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn the_bytes_after_the_first_eight_do_not_matter() {
        let mut other = ID;
        other[8..].fill(0);
        assert_eq!(
            pair(ID, Component::ActionList),
            pair(other, Component::ActionList)
        );
    }

    #[test]
    fn the_addition_wraps_at_2_pow_64() {
        let c = Component::GridTracking.constant();
        // E = 2^64 - c + 5 wraps to 5.
        let e = u64::MAX - c + 6;
        let mut id = [0; 16];
        id[..8].copy_from_slice(&e.to_le_bytes());
        let out = pair(id, Component::GridTracking);
        assert_eq!(out[..8], 5u64.to_be_bytes());
        assert_eq!(out[8..], e.to_be_bytes());
        // E = 2^64 - c sums to exactly 2^64, which wraps to 0.
        id[..8].copy_from_slice(&(0u64.wrapping_sub(c)).to_le_bytes());
        assert_eq!(pair(id, Component::GridTracking)[..8], [0; 8]);
    }
}
