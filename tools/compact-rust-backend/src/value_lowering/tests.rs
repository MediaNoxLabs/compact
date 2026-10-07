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

//! Boundary contracts for shared value lowering, independent of circuit admission.

use super::*;
use quote::ToTokens;

const TWO_128: &str = "340282366920938463463374607431768211456";
const MAX_248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const TWO_248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662656";
const TWO_256: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639936";

fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}

fn map(key: Type, value: Type) -> Type {
    Type::LedgerMap {
        key: Box::new(key),
        value: Box::new(value),
    }
}

fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}

fn tuple(elements: Vec<Type>) -> Type {
    Type::Tuple { elements }
}

fn assert_mismatch(actual: &Type, target: &Type, expected_actual: &Type, expected_target: &Type) {
    let error = coerce_expression(syn::parse_quote!(input), actual, target, 0)
        .err()
        .expect("lowering must reject this input");
    assert_eq!(
        error,
        RenderError::TypeMismatch {
            expected: expected_target.clone(),
            actual: expected_actual.clone(),
        }
    );
}

#[test]
fn canonical_decimal_values_keep_little_endian_byte_order() {
    for value in [0, 1, 255, 256, 65535, 65536, u64::MAX as u128, u128::MAX] {
        let mut expected = [0; 32];
        expected[..16].copy_from_slice(&value.to_le_bytes());
        assert_eq!(field_literal_bytes(&value.to_string()).unwrap(), expected);
    }
    let mut expected = [0; 32];
    expected[16] = 1;
    assert_eq!(field_literal_bytes(TWO_128).unwrap(), expected);
    expected = [255; 32];
    expected[31] = 0;
    assert_eq!(field_literal_bytes(MAX_248).unwrap(), expected);
}

#[test]
fn malformed_decimal_text_preserves_the_original_error_payload() {
    for text in [
        "", "00", "01", "+1", "-1", " 1", "1 ", "1\n", "1.0", "1e2", "0x10", "１", "١",
    ] {
        assert_eq!(
            field_literal_bytes(text),
            Err(RenderError::InvalidFieldLiteral(text.into())),
            "{text:?}"
        );
    }
}

#[test]
fn decimal_overflow_is_refused_instead_of_truncating_to_256_bits() {
    for text in [TWO_256.to_owned(), "9".repeat(200)] {
        assert_eq!(
            field_literal_bytes(&text),
            Err(RenderError::InvalidFieldLiteral(text))
        );
    }
}

#[test]
fn unsigned_carrier_changes_at_128_bits_and_stops_at_248_bits() {
    assert_eq!(unsigned_maximum("0"), Ok(UnsignedMaximum::Small(0)));
    assert_eq!(
        unsigned_maximum(&u128::MAX.to_string()),
        Ok(UnsignedMaximum::Small(u128::MAX))
    );
    assert_eq!(
        unsigned_maximum(TWO_128),
        Ok(UnsignedMaximum::Wide { high: 1, low: 0 })
    );
    assert_eq!(
        unsigned_maximum(MAX_248),
        Ok(UnsignedMaximum::Wide {
            high: (1u128 << 120) - 1,
            low: u128::MAX
        })
    );
    // 2^248 is a valid Field but is outside the supported unsigned carrier.
    assert!(field_literal_bytes(TWO_248).is_ok());
    for text in [TWO_248, TWO_256, "01", "bad"] {
        assert_eq!(
            unsigned_maximum(text),
            Err(RenderError::InvalidUnsignedMaximum(text.into()))
        );
    }
}

#[test]
fn nested_maps_have_slot_carriers_but_cannot_be_cell_values() {
    let nested = map(Type::OpaqueString, Type::Field);
    assert_eq!(
        rust_type(&nested)
            .err()
            .expect("lowering must reject this input"),
        RenderError::UnsupportedLedgerValueType(nested.clone())
    );
    let (key, value) = map_slot_types(&Type::OpaqueString, &nested)
        .unwrap()
        .unwrap();
    assert_eq!(key.to_token_stream().to_string(), "runtime :: OpaqueString");
    assert_eq!(
        value.to_token_stream().to_string(),
        "runtime :: slots :: MapNode < runtime :: OpaqueString , runtime :: Field >"
    );
}

#[test]
fn map_keys_and_aggregate_map_values_are_not_silently_flattened() {
    let nested = map(Type::OpaqueString, Type::Field);
    assert!(map_slot_types(&nested, &Type::Field).unwrap().is_none());
    assert!(
        map_slot_types(&Type::OpaqueString, &tuple(vec![nested.clone()]))
            .unwrap()
            .is_none()
    );
    assert!(
        map_slot_types(&Type::OpaqueString, &vector(nested.clone(), 1))
            .unwrap()
            .is_none()
    );
    assert!(
        map_slot_types(&Type::OpaqueString, &map(nested, Type::Field))
            .unwrap()
            .is_none()
    );
}

#[test]
fn malformed_map_bounds_remain_errors_instead_of_unsupported_carriers() {
    let bad = uint("01");
    for (key, value) in [
        (bad.clone(), Type::Field),
        (Type::OpaqueString, bad.clone()),
        (Type::OpaqueString, map(Type::OpaqueString, bad.clone())),
        (Type::OpaqueString, map(bad, Type::Field)),
    ] {
        assert_eq!(
            map_slot_types(&key, &value)
                .err()
                .expect("lowering must reject this input"),
            RenderError::InvalidUnsignedMaximum("01".into())
        );
    }
}

#[test]
fn malformed_nested_type_names_keep_identifier_diagnostics() {
    let bad = Type::Struct {
        name: "not a Rust name".into(),
        fields: vec![],
    };
    assert_eq!(
        map_slot_types(&Type::OpaqueString, &map(Type::OpaqueString, bad))
            .err()
            .expect("lowering must reject this input"),
        RenderError::InvalidIdentifier("not a Rust name".into())
    );
}

#[test]
fn implicit_unsigned_coercion_rejects_narrowing_in_both_carrier_widths() {
    for (actual, target) in [
        (uint("255"), uint("15")),
        (uint(TWO_128), uint("255")),
        (uint(MAX_248), uint(TWO_128)),
    ] {
        assert_mismatch(&actual, &target, &actual, &target);
        assert!(coerce_expression(syn::parse_quote!(input), &target, &actual, 0).is_ok());
    }
}

#[test]
fn aggregate_shape_mismatches_report_the_complete_types() {
    for (actual, target) in [
        (vector(uint("15"), 1), vector(uint("255"), 2)),
        (
            tuple(vec![uint("15")]),
            tuple(vec![uint("255"), uint("255")]),
        ),
        (tuple(vec![uint("15")]), vector(uint("255"), 2)),
        (vector(uint("15"), 2), tuple(vec![uint("255")])),
    ] {
        assert_mismatch(&actual, &target, &actual, &target);
    }
}

#[test]
fn nested_coercion_reports_the_failing_leaf_without_accepting_a_partial_conversion() {
    let source_leaf = uint("255");
    let target_leaf = uint("15");
    for (actual, target) in [
        (
            vector(source_leaf.clone(), 2),
            vector(target_leaf.clone(), 2),
        ),
        (
            tuple(vec![Type::Boolean, source_leaf.clone()]),
            tuple(vec![Type::Boolean, target_leaf.clone()]),
        ),
        (
            tuple(vec![source_leaf.clone()]),
            vector(target_leaf.clone(), 1),
        ),
        (
            vector(source_leaf.clone(), 1),
            tuple(vec![target_leaf.clone()]),
        ),
        (
            vector(tuple(vec![source_leaf.clone()]), 2),
            vector(tuple(vec![target_leaf.clone()]), 2),
        ),
    ] {
        assert_mismatch(&actual, &target, &source_leaf, &target_leaf);
        assert!(coerce_expression(syn::parse_quote!(input), &target, &actual, 0).is_ok());
    }
}

#[test]
fn aggregate_coercion_preserves_invalid_bound_diagnostics() {
    let actual = vector(tuple(vec![uint("01")]), 1);
    let target = vector(tuple(vec![Type::Field]), 1);
    assert_eq!(
        coerce_expression(syn::parse_quote!(input), &actual, &target, 0)
            .err()
            .expect("lowering must reject this input"),
        RenderError::InvalidUnsignedMaximum("01".into())
    );
}

mod vector_coercion;
