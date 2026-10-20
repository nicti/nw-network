//! Entitlement generated-message wire validation.

#[cfg(test)]
mod tests {
    use crate::generated_messages::EntitlementComponentServerFacetClientInitializeTransaction;
    use crate::serialize::{CARRIER_ENDIAN, Marshal, ReadBuffer, Unmarshal, WriteBuffer};

    fn string(out: &mut Vec<u8>, value: &str) {
        out.push(u8::try_from(value.len()).unwrap());
        out.extend_from_slice(value.as_bytes());
    }

    /// A request as the client writes it (NewWorld+0x3019b00), with made-up
    /// values: request id, three short strings, a big-endian u32, a 16-byte
    /// transaction id, three id strings, a currency alias, a big-endian u32
    /// and a bool.
    fn body(product: &str, alias: &str, flag: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        string(&mut out, "xx-YY");
        string(&mut out, "ZZZ");
        string(&mut out, "WW");
        out.extend_from_slice(&[0, 0, 0, 2]);
        out.extend_from_slice(&[0xA0; 16]);
        string(&mut out, product);
        string(&mut out, product);
        string(&mut out, "");
        string(&mut out, alias);
        out.extend_from_slice(&[0, 0, 1, 4]);
        out.push(flag);
        out
    }

    fn assert_reads_whole_and_roundtrips(body: &[u8]) {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, body);
        let value = EntitlementComponentServerFacetClientInitializeTransaction::unmarshal(&mut rb)
            .expect("decodes");
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
    fn client_initialize_transaction_reads_every_field() {
        assert_reads_whole_and_roundtrips(&body("abcdefghijklmnopqrstuv", "currency_alias_ab", 1));
        assert_reads_whole_and_roundtrips(&body("abcdefghijklmnopqrstu", "currency_alias_ab", 1));
        // The request that names no currency.
        assert_reads_whole_and_roundtrips(&body("abcdefghijklmnopqrstuv", "", 0));
    }

    #[test]
    fn client_initialize_transaction_sizes_match_the_layout() {
        assert_eq!(body("abcdefghijklmnopqrstuv", "", 0).len(), 102);
        assert_eq!(
            body("abcdefghijklmnopqrstuv", "0123456789abcdef", 1).len(),
            118
        );
        assert_eq!(
            body("abcdefghijklmnopqrstu", "0123456789abcdef", 1).len(),
            116
        );
    }

    #[test]
    fn client_initialize_transaction_fields_land_where_the_client_writes_them() {
        let bytes = body("abcdefghijklmnopqrstuv", "gold", 1);
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, &bytes);
        let v =
            EntitlementComponentServerFacetClientInitializeTransaction::unmarshal(&mut rb).unwrap();
        assert_eq!(v.accept_language.as_slice(), b"xx-YY");
        assert_eq!(v.currency.as_slice(), b"ZZZ");
        assert_eq!(v.country.as_slice(), b"WW");
        assert_eq!(v.transaction_type, 2);
        assert_eq!(v.client_transaction_id, [0xA0; 16]);
        assert_eq!(v.product_id.as_slice(), b"abcdefghijklmnopqrstuv");
        assert_eq!(v.offer_id.as_slice(), b"abcdefghijklmnopqrstuv");
        assert!(v.external_offer_id.is_empty());
        assert_eq!(v.in_game_currency_alias.as_slice(), b"gold");
        assert_eq!(v.expected_amount, 260);
        assert!(v.flag);
    }
}
