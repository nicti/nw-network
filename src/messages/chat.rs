//! Chat generated-message wire validation.

#[cfg(test)]
mod tests {
    use crate::generated_messages::ChatComponentClientFacetReceiveBatchedChatMessages;
    use crate::serialize::{CARRIER_ENDIAN, Marshal, ReadBuffer, Unmarshal, WriteBuffer};

    /// A request id, then made-up values throughout.
    const HEADER: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    fn string(out: &mut Vec<u8>, value: &str) {
        out.push(u8::try_from(value.len()).unwrap());
        out.extend_from_slice(value.as_bytes());
    }

    /// One chat message as the client reads it: three strings and a u8
    /// around them, a u32, two strings, five bools, a counted list of up
    /// to 20 `(u32, u16 x 6, u8)` entries, a u32, a string, a bool, a u8.
    fn chat_message(text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        string(&mut out, "alpha");
        string(&mut out, "beta");
        out.push(3);
        string(&mut out, "");
        out.extend_from_slice(&[0, 0, 0, 7]);
        string(&mut out, text);
        string(&mut out, "gamma");
        out.extend_from_slice(&[1, 0, 1, 0, 1]);
        out.push(1);
        out.extend_from_slice(&[0, 0, 0, 9, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 2]);
        out.extend_from_slice(&[0, 0, 1, 0]);
        string(&mut out, "delta");
        out.push(1);
        out.push(4);
        out
    }

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

    #[test]
    fn batched_chat_messages_read_every_message() {
        let mut body = HEADER.to_vec();
        body.push(2);
        body.extend(chat_message("one"));
        body.extend(chat_message("two"));
        assert_reads_whole_and_roundtrips::<ChatComponentClientFacetReceiveBatchedChatMessages>(
            &body,
        );
    }
}
