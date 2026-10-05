//! Objectives generated-message wire validation.

#[cfg(test)]
mod tests {
    use crate::generated_messages::ObjectivesComponentServerFacetUpdateTrackedObjectives;
    use crate::serialize::{CARRIER_ENDIAN, Marshal, ReadBuffer, Unmarshal, WriteBuffer};

    #[test]
    fn update_tracked_objectives_reads_every_objective() {
        // A request id, a count of 2 and two made-up u64 ids.
        let mut body = (1..=16).collect::<Vec<u8>>();
        body.push(2);
        body.extend_from_slice(&[0, 0, 0, 0, 0x12, 0x34, 0x56, 0x78]);
        body.extend_from_slice(&[0, 0, 0, 0, 0x9a, 0xbc, 0xde, 0xf0]);

        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, &body);
        let value = ObjectivesComponentServerFacetUpdateTrackedObjectives::unmarshal(&mut rb)
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
}
