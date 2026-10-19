//! The guild lookup: `GuildsComponentServerFacet_RequestGetGuilds` (3639)
//! and its answer `GuildsComponentClientFacet_OnGetUnorderedGuildsMsg`
//! (3610).
//!
//! Both start with the 16-byte request id pair and a 16-byte lookup id.
//! The schema stopped there. The rest is a counted sequence:
//!
//! - 3639 (unmarshal NewWorld+0x3477c50): a packed-size count (more than 20
//!   is rejected), then one 16-byte guild id per element.
//! - 3610 (unmarshal NewWorld+0x3478ee0, sequence NewWorld+0x3489090,
//!   element NewWorld+0x5ce7390): a packed-size count, then per guild the
//!   16-byte guild id, `m_guildName`, `m_guildMasterCharacterIdString`,
//!   `m_faction` (u8), `m_crestData` (u16 background, RGBA, u16 foreground,
//!   RGBA, each colour four big-endian f32 the client requires in 0..=1),
//!   `m_numMembers`, `m_numClaims`, `m_numWars` (big-endian u32),
//!   `m_siegeWindow` (big-endian f32) and one more string. The names are
//!   the client's reflection names for the element's offsets.

#[cfg(test)]
mod tests {
    use crate::generated_messages::{
        GuildsComponentClientFacetOnGetUnorderedGuildsMsg as Reply,
        GuildsComponentServerFacetRequestGetGuilds as Request,
    };
    use crate::serialize::{CARRIER_ENDIAN, Marshal, ReadBuffer, Unmarshal, WriteBuffer};

    const HEADER: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    const LOOKUP: [u8; 16] = [0x20; 16];
    const GUILD: [u8; 16] = [0x30; 16];
    const CHARACTER: &str = "00000000-0000-4000-8000-000000000001";

    fn string(out: &mut Vec<u8>, value: &str) {
        out.push(u8::try_from(value.len()).unwrap());
        out.extend_from_slice(value.as_bytes());
    }

    fn colour(out: &mut Vec<u8>, rgba: [f32; 4]) {
        for c in rgba {
            out.extend_from_slice(&c.to_be_bytes());
        }
    }

    /// A request as the client writes it, with made-up ids.
    fn request(guilds: &[[u8; 16]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&HEADER);
        out.extend_from_slice(&LOOKUP);
        out.push(u8::try_from(guilds.len()).unwrap());
        for g in guilds {
            out.extend_from_slice(g);
        }
        out
    }

    /// One guild in the reply's layout, with made-up values.
    fn guild(out: &mut Vec<u8>, name: &str, last: &str) {
        out.extend_from_slice(&GUILD);
        string(out, name);
        string(out, CHARACTER);
        out.push(2);
        out.extend_from_slice(&237u16.to_be_bytes());
        colour(out, [0.25, 0.5, 0.75, 1.0]);
        out.extend_from_slice(&300u16.to_be_bytes());
        colour(out, [1.0, 0.75, 0.25, 1.0]);
        out.extend_from_slice(&50u32.to_be_bytes());
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&20.5f32.to_be_bytes());
        string(out, last);
    }

    fn reply(guilds: &[(&str, &str)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&HEADER);
        out.extend_from_slice(&LOOKUP);
        out.push(u8::try_from(guilds.len()).unwrap());
        for (name, last) in guilds {
            guild(&mut out, name, last);
        }
        out
    }

    fn reads_whole_and_roundtrips<T: Unmarshal + Marshal>(body: &[u8]) -> T {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, body);
        let value = T::unmarshal(&mut rb).expect("decodes");
        assert!(
            rb.remaining().is_empty(),
            "{} bytes unread",
            rb.remaining().len()
        );
        let mut wb = WriteBuffer::new(CARRIER_ENDIAN);
        value.marshal(&mut wb);
        assert_eq!(wb.into_vec(), body);
        value
    }

    #[test]
    fn request_get_guilds_reads_every_id() {
        let body = request(&[GUILD]);
        assert_eq!(body.len(), 49);
        let v: Request = reads_whole_and_roundtrips(&body);
        assert_eq!(v.header.target_local_id, 0x0102_0304_0506_0708);
        assert_eq!(v.header.source_actor_ref, 0x090a_0b0c_0d0e_0f10);
        assert_eq!(v.request_id, LOOKUP);
        assert_eq!(v.guild_ids, vec![GUILD]);
        let two: Request = reads_whole_and_roundtrips(&request(&[GUILD, [0x31; 16]]));
        assert_eq!(two.guild_ids.len(), 2);
    }

    #[test]
    fn unordered_guilds_reads_every_field() {
        let v: Reply = reads_whole_and_roundtrips(&reply(&[("Made Up", "")]));
        assert_eq!(v.request_id, LOOKUP);
        let [(id, name, master, faction, crest, members, claims, wars, siege, last)] =
            v.guilds.as_slice()
        else {
            panic!("one guild")
        };
        assert_eq!(*id, GUILD);
        assert_eq!(name, "Made Up");
        assert_eq!(master, CHARACTER);
        assert_eq!(*faction, 2);
        assert_eq!(
            *crest,
            (237, (0.25, 0.5, 0.75, 1.0), 300, (1.0, 0.75, 0.25, 1.0))
        );
        assert_eq!((*members, *claims, *wars), (50, 1, 0));
        assert_eq!(*siege, 20.5);
        assert!(last.is_empty());
    }

    #[test]
    fn unordered_guilds_sizes_match_the_layout() {
        // An unknown guild is answered with none: the two ids and a zero count.
        let none: Reply = reads_whole_and_roundtrips(&reply(&[]));
        assert!(none.guilds.is_empty());
        assert_eq!(reply(&[]).len(), 33);
        // One guild is 141 bytes plus its name and last string.
        let short = reply(&[("abcdefg", "")]);
        assert_eq!(short.len(), 148);
        reads_whole_and_roundtrips::<Reply>(&short);
        let long = reply(&[(
            "abcdefghijklmnopqrstuvwx",
            "0123456789abcdef0123456789abcdef0123456789a=",
        )]);
        assert_eq!(long.len(), 209);
        reads_whole_and_roundtrips::<Reply>(&long);
        let two = reply(&[("abc", ""), ("defg", "")]);
        assert_eq!(reads_whole_and_roundtrips::<Reply>(&two).guilds.len(), 2);
    }
}
