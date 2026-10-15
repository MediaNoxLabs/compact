// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

include!(concat!(env!("OUT_DIR"), "/contract.rs"));
pub mod support;
pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-owned-values-schema20-ir.json"
);

#[cfg(test)]
mod tests {
    use super::runtime::{Field, FixedVector, OpaqueBytes, OpaqueString};
    use super::{pure_circuits as pure, types};
    #[test]
    fn generated_pure_owned_interfaces_roundtrip_nonempty_payloads() {
        // Direct native pure API test: source recording uses default opaque payloads.
        let text = OpaqueString::from("owned ✓");
        let bytes = OpaqueBytes::from(vec![0, 1, 255]);
        assert_eq!(pure::textIdentity(text.clone()).unwrap(), text);
        assert_eq!(pure::bytesIdentity(bytes.clone()).unwrap(), bytes);
        let leaf = types::OwnedLeaf { text, bytes };
        let value = types::Owned {
            leaf: leaf.clone(),
            leaves: FixedVector::new([leaf.clone(), leaf]),
            values: FixedVector::new([Field::from(7_u64), Field::from(11_u64)]),
            marker: Field::from(3_u64),
        };
        assert_eq!(pure::ownedIdentity(value.clone()).unwrap(), value);
        assert_eq!(pure::ownedIdentity(value.clone()).unwrap(), value);
        assert_eq!(
            pure::vectorSum(value.values.clone()).unwrap(),
            Field::from(18_u64)
        );
    }
}
