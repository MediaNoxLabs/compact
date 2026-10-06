// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Direct native-wrapper tests: frozen TypeScript vectors, explicit FAB preimages,
//! and upstream curve/scalar values. These tests do not generate proofs.

use midnight_base_crypto::hash as upstream_persistent;
use midnight_compact_runtime::{
    CompactError, Field, FixedBytes, FixedVector, JubjubPoint, construct_jubjub_point,
    degrade_to_transient, ec_add, ec_mul, ec_mul_generator, ec_neg, hash_to_curve, jubjub_point_x,
    jubjub_point_y, jubjub_scalar_from_native, keccak256, persistent_commit, persistent_hash,
    transient_commit, transient_hash, upgrade_from_transient,
};
use midnight_transient_crypto::curve::{EmbeddedFr, EmbeddedGroupAffine, embedded};
use midnight_transient_crypto::hash as upstream_transient;
use serde_json::{Value, json};

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn field_hex(value: Field) -> String {
    hex::encode(value.as_le_bytes())
}

fn decode_field(value: &Value) -> Field {
    Field::from_le_bytes(&hex::decode(value.as_str().unwrap()).unwrap()).unwrap()
}

fn coordinates(point: JubjubPoint) -> Value {
    json!({"x": field_hex(jubjub_point_x(point)), "y": field_hex(jubjub_point_y(point))})
}

#[test]
fn transient_hashes_match_frozen_typescript_field_vector_and_bytes_values() {
    // Inputs and oracle provenance: fixtures/capture-transient-hash.mjs.
    let oracle = fixture(include_str!("fixtures/transient-hash.json"));
    assert_eq!(field_hex(transient_hash(Field::from(42))), oracle["field"]);
    assert_eq!(
        field_hex(transient_hash(FixedVector::new([
            Field::from(3),
            Field::from(5)
        ]))),
        oracle["pair"]
    );
    assert_eq!(
        field_hex(transient_hash(FixedBytes::new([1, 2, 0, 0]))),
        oracle["bytes"]
    );
    assert_eq!(
        field_hex(transient_commit(Field::from(42), Field::from(7))),
        oracle["commitField"]
    );
    assert_eq!(
        field_hex(transient_commit(
            FixedVector::new([Field::from(3), Field::from(5)]),
            Field::from(7)
        )),
        oracle["commitPair"]
    );
}

#[test]
fn persistent_hashes_and_commitments_match_frozen_typescript_values() {
    // Inputs and oracle provenance: fixtures/capture-persistent-hash.mjs.
    let oracle = fixture(include_str!("fixtures/persistent-hash.json"));
    let field = Field::from(42);
    let opening = FixedBytes::new(std::array::from_fn(|i| i as u8 + 1));
    let digest = persistent_hash(field);
    assert_eq!(hex::encode(digest.0), oracle["hashField"]);
    assert_eq!(
        hex::encode(persistent_hash(FixedVector::new([Field::from(3), Field::from(5)])).0),
        oracle["hashPair"]
    );
    assert_eq!(
        hex::encode(persistent_commit(field, opening).0),
        oracle["commitField"]
    );
    assert_eq!(field_hex(degrade_to_transient(digest)), oracle["degrade"]);
    assert_eq!(
        hex::encode(upgrade_from_transient(field).0),
        oracle["upgrade"]
    );
}

#[test]
fn mixed_fab_values_match_independently_written_preimages() {
    // FAB value-only representation: Boolean is one byte/field; Bytes<4>
    // pads binary bytes but occupies one field; Field occupies 32 binary bytes.
    // No runtime value_repr or hash wrapper constructs the expected preimage.
    let value = (true, FixedBytes::new([1, 2, 0, 0]), Field::from(42));
    let fields = [Field::from(1), Field::from(513), Field::from(42)];
    assert_eq!(
        transient_hash(value),
        upstream_transient::transient_hash(&fields)
    );
    let mut bytes = vec![1, 1, 2, 0, 0];
    bytes.extend_from_slice(&Field::from(42).as_le_bytes());
    assert_eq!(
        persistent_hash(value).0,
        upstream_persistent::persistent_hash(&bytes).0
    );
    let opening = FixedBytes::new([7; 32]);
    let mut committed_bytes = opening.0.to_vec();
    committed_bytes.extend_from_slice(&bytes);
    assert_eq!(
        persistent_commit(value, opening).0,
        upstream_persistent::persistent_hash(&committed_bytes).0
    );
    assert_eq!(
        transient_commit(value, Field::from(7)),
        upstream_transient::transient_hash(&[Field::from(7), fields[0], fields[1], fields[2]])
    );

    // This is application-provided domain data, not implicit type-domain separation.
    let changed_domain = (false, FixedBytes::new([1, 2, 0, 0]), Field::from(42));
    assert_ne!(
        transient_commit(value, Field::from(7)),
        transient_commit(changed_domain, Field::from(7))
    );
    assert_ne!(
        persistent_commit(value, opening),
        persistent_commit(changed_domain, opening)
    );
    assert_ne!(
        transient_commit(value, Field::from(7)),
        transient_commit(value, Field::from(8))
    );
    assert_ne!(
        persistent_commit(value, opening),
        persistent_commit(value, FixedBytes::new([8; 32]))
    );
}

#[test]
fn fab_alignment_preserves_padding_and_field_chunk_boundaries() {
    assert_eq!(
        persistent_hash(FixedBytes::new([1, 0])).0,
        upstream_persistent::persistent_hash(&[1, 0]).0
    );
    assert_ne!(
        persistent_hash(FixedBytes::new([1, 0])),
        persistent_hash(FixedBytes::new([1]))
    );
    let mut bytes31 = [0; 31];
    bytes31[0] = 7;
    let mut bytes32 = [0; 32];
    bytes32[0] = 7;
    bytes32[31] = 9;
    assert_eq!(
        transient_hash(FixedBytes::new(bytes31)),
        upstream_transient::transient_hash(&[Field::from(7)])
    );
    // Bytes<32> is the one-byte remainder followed by its 31-byte chunk.
    assert_eq!(
        transient_hash(FixedBytes::new(bytes32)),
        upstream_transient::transient_hash(&[Field::from(9), Field::from(7)])
    );
    assert_eq!(
        persistent_hash(FixedBytes::new(bytes32)).0,
        upstream_persistent::persistent_hash(&bytes32).0
    );
}

#[test]
fn hash_to_curve_and_coordinate_wrappers_match_frozen_typescript_points() {
    // Inputs and oracle provenance: fixtures/capture-jubjub-hash.mjs.
    let oracle = fixture(include_str!("fixtures/jubjub-hash.json"));
    assert_eq!(coordinates(hash_to_curve(Field::from(42))), oracle["point"]);
    assert_eq!(
        coordinates(hash_to_curve(FixedVector::new([
            Field::from(3),
            Field::from(5)
        ]))),
        oracle["pairPoint"]
    );
}

#[test]
fn curve_arithmetic_matches_frozen_typescript_vectors_and_group_laws() {
    // Inputs and oracle provenance: fixtures/capture-jubjub-arithmetic.mjs.
    let oracle = fixture(include_str!("fixtures/jubjub-arithmetic.json"));
    let left = hash_to_curve(Field::from(3));
    let right = hash_to_curve(Field::from(5));
    assert_eq!(coordinates(ec_add(left, right)), oracle["add"]);
    assert_eq!(coordinates(ec_neg(left)), oracle["negate"]);
    assert_eq!(
        coordinates(ec_mul(left, Field::from(7)).unwrap()),
        oracle["multiply"]
    );
    assert_eq!(
        coordinates(ec_mul_generator(Field::from(7)).unwrap()),
        oracle["generator"]
    );
    let identity = JubjubPoint::identity();
    assert_eq!(ec_add(left, identity), left);
    assert_eq!(ec_add(identity, left), left);
    assert_eq!(ec_add(left, ec_neg(left)), identity);
    assert_eq!(ec_neg(ec_neg(left)), left);
    assert_eq!(ec_mul(left, Field::from(0)), Ok(identity));
    assert_eq!(ec_mul(left, Field::from(1)), Ok(left));
    assert_eq!(ec_mul(left, Field::from(2)), Ok(ec_add(left, left)));
    assert_eq!(
        ec_mul_generator(Field::from(1)),
        Ok(EmbeddedGroupAffine::generator().into())
    );
}

#[test]
fn point_construction_accepts_both_identity_encodings_and_rejects_invalid_coordinates() {
    for y in [0, 1] {
        let identity = construct_jubjub_point(Field::from(0), Field::from(y)).unwrap();
        assert_eq!(identity, JubjubPoint::identity());
        assert_eq!(jubjub_point_x(identity), Field::from(0));
        assert_eq!(jubjub_point_y(identity), Field::from(1));
    }
    for (x, y) in [(0, 2), (1, 1), (1, 0)] {
        assert_eq!(
            construct_jubjub_point(Field::from(x), Field::from(y)),
            Err(CompactError::InvalidJubjubPoint)
        );
    }
    let oracle = fixture(include_str!("fixtures/jubjub-construct.json"));
    let point = construct_jubjub_point(
        decode_field(&oracle["input"]["x"]),
        decode_field(&oracle["input"]["y"]),
    )
    .unwrap();
    assert_eq!(coordinates(point), oracle["constructed"]);
}

fn increment_le(bytes: &mut [u8]) {
    for byte in bytes {
        let (next, carry) = byte.overflowing_add(1);
        *byte = next;
        if !carry {
            return;
        }
    }
    panic!("scalar boundary does not fit field encoding");
}

#[test]
fn canonical_scalar_order_boundary_requires_explicit_reduction() {
    // Upstream scalar -1 supplies q-1 without the runtime reduction helper.
    let mut bytes = EmbeddedFr(-embedded::Scalar::from(1_u64)).as_le_bytes();
    let last_canonical = Field::from_le_bytes(&bytes).unwrap();
    let generator: JubjubPoint = EmbeddedGroupAffine::generator().into();
    let negative_generator: JubjubPoint = (-EmbeddedGroupAffine::generator()).into();
    assert_eq!(ec_mul_generator(last_canonical), Ok(negative_generator));
    assert_eq!(ec_mul(generator, last_canonical), Ok(negative_generator));
    assert_eq!(jubjub_scalar_from_native(last_canonical), last_canonical);
    increment_le(&mut bytes);
    let order = Field::from_le_bytes(&bytes).unwrap();
    increment_le(&mut bytes);
    let order_plus_one = Field::from_le_bytes(&bytes).unwrap();
    for scalar in [order, order_plus_one] {
        assert_eq!(
            ec_mul_generator(scalar),
            Err(CompactError::InvalidJubjubScalar)
        );
        assert_eq!(
            ec_mul(generator, scalar),
            Err(CompactError::InvalidJubjubScalar)
        );
        // An identity point does not bypass the scalar check.
        assert_eq!(
            ec_mul(JubjubPoint::identity(), scalar),
            Err(CompactError::InvalidJubjubScalar)
        );
    }
    assert_eq!(jubjub_scalar_from_native(order), Field::from(0));
    assert_eq!(jubjub_scalar_from_native(order_plus_one), Field::from(1));
    assert_eq!(
        ec_mul_generator(jubjub_scalar_from_native(order)),
        Ok(JubjubPoint::identity())
    );
    assert_eq!(
        ec_mul_generator(jubjub_scalar_from_native(order_plus_one)),
        Ok(generator)
    );
}

#[test]
fn raw_native_field_maximum_matches_typescript_refusal_and_reduction() {
    let oracle = fixture(include_str!("fixtures/jubjub-arithmetic.json"));
    let high = decode_field(&oracle["highScalar"]);
    assert_eq!(oracle["highScalarRejected"], true);
    assert_eq!(oracle["highMultiplyRejected"], true);
    assert_eq!(
        ec_mul_generator(high),
        Err(CompactError::InvalidJubjubScalar)
    );
    assert_eq!(
        ec_mul(hash_to_curve(Field::from(3)), high),
        Err(CompactError::InvalidJubjubScalar)
    );
    let reduced = jubjub_scalar_from_native(high);
    assert_eq!(field_hex(reduced), oracle["reduced"]);
    assert_eq!(
        coordinates(ec_mul_generator(reduced).unwrap()),
        oracle["generatorReduced"]
    );
}

#[test]
fn digest_conversion_retains_only_the_documented_31_byte_payload() {
    let mut bytes = [0; 32];
    bytes[0] = 42;
    bytes[31] = 1;
    let wide_field = Field::from_le_bytes(&bytes).unwrap();
    let mut expected = bytes;
    expected[31] = 0;
    assert_eq!(
        upgrade_from_transient(wide_field),
        FixedBytes::new(expected)
    );
    assert_eq!(
        degrade_to_transient(FixedBytes::new(bytes)),
        Field::from(42)
    );
    assert_eq!(
        degrade_to_transient(upgrade_from_transient(wide_field)),
        Field::from(42)
    );
    assert_ne!(
        degrade_to_transient(upgrade_from_transient(wide_field)),
        wide_field
    );
}

#[test]
fn keccak_uses_concatenated_normalized_atoms_with_no_added_type_tags() {
    // Published Keccak-256 empty/ASCII "abc" vectors; these are not SHA3-256.
    assert_eq!(
        hex::encode(keccak256(()).0),
        "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
    );
    let expected = "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45";
    assert_eq!(hex::encode(keccak256(FixedBytes::new(*b"abc")).0), expected);
    assert_eq!(
        hex::encode(keccak256((FixedBytes::new(*b"ab"), FixedBytes::new(*b"c"))).0),
        expected
    );
    // FAB strips trailing zero bytes from a fixed-byte atom; Keccak hashes the
    // normalized atom itself, unlike persistent_hash's padded FAB encoding.
    assert_eq!(
        keccak256(FixedBytes::new([1, 0])),
        keccak256(FixedBytes::new([1]))
    );
}
