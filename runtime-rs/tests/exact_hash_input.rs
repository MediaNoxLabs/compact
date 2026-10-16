// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use midnight_compact_runtime as r;
use r::fab::{Aligned, AlignedValue, Value};
use r::{CompactCellValue, CompactHashInput, HashInputValue};

#[derive(Clone, Debug, PartialEq, Eq, CompactCellValue, CompactHashInput)]
struct Payload {
    text: r::OpaqueString,
    data: r::OpaqueBytes,
}
#[derive(Clone, Debug, PartialEq, Eq, CompactCellValue, CompactHashInput)]
struct Envelope {
    payload: Payload,
    nonce: r::Field,
    samples: r::FixedVector<(r::OpaqueBytes, r::FixedBytes<2>), 2>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue, CompactHashInput)]
enum Mode {
    First,
    Last,
}

fn expected_field(s: &str) -> r::Field {
    s.bytes().fold(r::Field::from(0_u64), |value, byte| {
        assert!(byte.is_ascii_digit());
        value * r::Field::from(10_u64) + r::Field::from(u64::from(byte - b'0'))
    })
}
#[test]
fn exact_opaque_hashes_match_original_ts_without_changing_storage_conversion() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests-rust-backend/effect-plan-opaque-aligned-hashes-source/oracle/typescript.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in oracle["rows"].as_array().unwrap() {
        let bytes = if row["type"] == "string" {
            row["input"].as_str().unwrap().as_bytes().to_vec()
        } else {
            row["input"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect()
        };
        let original = r::OpaqueBytes(bytes.clone());
        let exact: Value = r::exact_hash_input(original.clone()).into();
        assert_eq!(exact.0[0].0, bytes, "{}", row["id"]);
        let aligned = AlignedValue::new(exact, r::OpaqueBytes::alignment()).unwrap();
        let round_trip =
            serde_json::from_value::<AlignedValue>(serde_json::to_value(&aligned).unwrap());
        assert_eq!(round_trip.is_ok(), bytes.last() != Some(&0));
        let ordinary: Value = original.clone().into();
        assert!(ordinary.0[0].is_in_normal_form());
        let expected =
            ["x", "y", "value"].map(|key| expected_field(row["result"][key].as_str().unwrap()));
        let actual = match row["op"].as_str().unwrap() {
            "curve" => {
                let p = r::hash_to_curve(r::exact_hash_input(original));
                [
                    r::jubjub_point_x(p),
                    r::jubjub_point_y(p),
                    r::Field::from(0_u64),
                ]
            }
            "transient" => [
                r::Field::from(0_u64),
                r::Field::from(0_u64),
                r::transient_hash(r::exact_hash_input(original)),
            ],
            "commit" => [
                r::Field::from(0_u64),
                r::Field::from(0_u64),
                r::transient_commit(r::exact_hash_input(original), r::Field::from(7_u64)),
            ],
            other => panic!("unknown primitive {other}"),
        };
        assert_eq!(actual, expected, "{}", row["id"]);
        count += 1;
    }
    assert_eq!(count, 24);
}
#[test]
fn derived_products_preserve_opaque_atoms_and_fixed_encoding_in_order() {
    let value = Envelope {
        payload: Payload {
            text: r::OpaqueString::from("a\0"),
            data: r::OpaqueBytes(vec![0]),
        },
        nonce: r::Field::from(7_u64),
        samples: r::FixedVector([
            (r::OpaqueBytes(vec![1, 0]), r::FixedBytes([2, 0])),
            (r::OpaqueBytes(vec![]), r::FixedBytes([0, 0])),
        ]),
    };
    let ordinary: Value = value.clone().into();
    let exact = value.into_hash_value();
    let atoms: Vec<_> = exact.0.iter().map(|v| v.0.clone()).collect();
    assert_eq!(
        atoms,
        vec![
            vec![97, 0],
            vec![0],
            vec![7],
            vec![1, 0],
            vec![2],
            vec![],
            vec![]
        ]
    );
    assert_ne!(ordinary, exact);
    assert!(AlignedValue::new(exact, Envelope::alignment()).is_some());
    for mode in [Mode::First, Mode::Last] {
        assert_eq!(mode.into_hash_value(), Value::from(mode));
    }
}
#[test]
fn tuple_and_vector_composition_reach_official_arity_without_changing_fixed_leaves() {
    let tuple = (
        r::OpaqueString::from("x\0"),
        true,
        r::Field::from(7_u64),
        r::FixedBytes([4, 0]),
        0_u8,
        0_u16,
        0_u32,
        0_u64,
        0_u128,
        (),
        r::OpaqueBytes(vec![1, 0]),
    );
    let value = tuple.into_hash_value();
    assert_eq!(value.0.len(), 10);
    assert_eq!(value.0.first().unwrap().0, vec![120, 0]);
    assert_eq!(value.0.last().unwrap().0, vec![1, 0]);
    let empty = r::FixedVector::<r::OpaqueBytes, 0>([]).into_hash_value();
    assert!(empty.0.is_empty());
    let singleton = (r::OpaqueBytes(vec![0]),).into_hash_value();
    assert_eq!(singleton.0[0].0, vec![0]);
    fn unchanged<T: HashInputValue + Into<Value> + Clone>(value: T) {
        assert_eq!(value.clone().into_hash_value(), value.into());
    }
    unchanged(r::BoundedUint::<255>::new(9).unwrap());
    unchanged(r::FixedBytes([1_u8, 0]));
    unchanged(r::FixedVector([
        r::Field::from(1_u64),
        r::Field::from(2_u64),
    ]));
}
