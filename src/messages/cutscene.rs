//! Cutscene generated-message wire validation.

#[cfg(test)]
mod tests {
    use crate::generated_messages::{
        PlayerCutsceneComponentClientFacetOnServerCutsceneCanceled,
        PlayerCutsceneComponentClientFacetOnServerCutsceneEndRequestFinalized,
        PlayerCutsceneComponentServerFacetOnClientCutsceneEnded,
    };
    use crate::serialize::{CARRIER_ENDIAN, Marshal, ReadBuffer, Unmarshal, WriteBuffer};

    /// A request id: two made-up u64s, nothing after it.
    const BODY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    fn assert_reads_whole_and_roundtrips<T: Unmarshal + Marshal>(body: &[u8]) {
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
    }

    /// The three messages whose body is an `ActorRequestIdPayload` read
    /// the whole request id, both u64s (`ActorRequestId::Unmarshal`,
    /// NewWorld+0x15095d0, called from NewWorld+0x259e1e0), not the first
    /// eight bytes.
    #[test]
    fn cutscene_end_messages_read_the_whole_request_id() {
        assert_reads_whole_and_roundtrips::<PlayerCutsceneComponentServerFacetOnClientCutsceneEnded>(
            &BODY,
        );
        assert_reads_whole_and_roundtrips::<
            PlayerCutsceneComponentClientFacetOnServerCutsceneEndRequestFinalized,
        >(&BODY);
        assert_reads_whole_and_roundtrips::<
            PlayerCutsceneComponentClientFacetOnServerCutsceneCanceled,
        >(&BODY);
    }
}
