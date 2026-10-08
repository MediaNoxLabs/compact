use passport::{pure_circuits as pure, runtime as rt, types};

// Deterministic test data only; an application supplies real commitments.
fn bytes32(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}

pub fn claim_root() -> Result<rt::FixedBytes<32>, rt::CompactError> {
    pure::digitalPassportClaimRoot(types::DigitalPassportClaimCommitments {
        firstNameCommitment: bytes32(1),
        lastNameCommitment: bytes32(2),
        dateOfBirthCommitment: bytes32(3),
        documentNumberCommitment: bytes32(4),
        issuingStateCommitment: bytes32(5),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_matches_the_pinned_typescript_vector() {
        let hex: String = claim_root().unwrap().into_array().iter()
            .map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex,
            "1719d1840150ddc72ed9bd11126c673158594527eb03c0e3935868201a64b28d");
    }

    #[test]
    fn schema_checks_still_run_for_well_typed_values() {
        let mut schema = types::SchemaRef {
            packageId: bytes32(1),
            schemaId: bytes32(2),
            majorVersion: rt::BoundedUint::<65535>::new(1).unwrap(),
            minorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
        };
        pure::assertValidSchemaRef(schema.clone()).unwrap();
        schema.majorVersion = rt::BoundedUint::<65535>::new(0).unwrap();
        assert!(matches!(pure::assertValidSchemaRef(schema),
            Err(rt::CompactError::AssertionFailed(message))
                if message == "Schema major version must be positive"));
    }
}
