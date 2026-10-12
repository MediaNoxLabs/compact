// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_aligned_witness_results_source_fixture::{runtime as r, types};
use serde_json::Value;
pub type Tuple11 = (
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
);
pub fn field(value: &Value) -> r::Field {
    value
        .as_str()
        .unwrap()
        .bytes()
        .fold(r::Field::from(0_u64), |v, d| {
            assert!(d.is_ascii_digit());
            v * r::Field::from(10_u64) + r::Field::from(u64::from(d - b'0'))
        })
}
fn point(value: &Value) -> r::JubjubPoint {
    // Decode frozen TS coordinates through the checked official FAB boundary;
    // expected points are never recomputed by hashing under test.
    let atoms = r::fab::Value::from((field(&value["x"]), field(&value["y"])));
    r::JubjubPoint::try_from(&*atoms).unwrap()
}
fn bytes(value: &Value) -> r::FixedBytes<32> {
    r::FixedBytes(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}
#[derive(Clone)]
pub enum Payload {
    Text(r::OpaqueString),
    Entry(types::Entry),
    Nested(types::Nested),
    Tuple(Box<Tuple11>),
}
impl Payload {
    pub fn from_json(circuit: &str, value: &Value) -> Self {
        match circuit {
            "text" | "unit_text" => Self::Text(value.as_str().unwrap().into()),
            "entry" => Self::Entry(types::Entry {
                text: value["text"].as_str().unwrap().into(),
                point: point(&value["point"]),
            }),
            "nested" => Self::Nested(types::Nested {
                items: r::FixedVector(std::array::from_fn(|i| {
                    (
                        value["items"][i][0].as_str().unwrap().into(),
                        point(&value["items"][i][1]),
                    )
                })),
            }),
            "tuple11" => Self::Tuple(Box::new((
                bytes(&value[0]),
                bytes(&value[1]),
                bytes(&value[2]),
                bytes(&value[3]),
                bytes(&value[4]),
                bytes(&value[5]),
                bytes(&value[6]),
                bytes(&value[7]),
                bytes(&value[8]),
                bytes(&value[9]),
                bytes(&value[10]),
            ))),
            _ => panic!("unknown source root {circuit}"),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Text(_) => "text_value",
            Self::Entry(_) => "entry_value",
            Self::Nested(_) => "nested_value",
            Self::Tuple(_) => "tuple_value",
        }
    }
    pub fn aligned(&self) -> r::fab::AlignedValue {
        match self {
            Self::Text(v) => r::exact_private_output(v.clone()),
            Self::Entry(v) => r::exact_private_output(v.clone()),
            Self::Nested(v) => r::exact_private_output(v.clone()),
            Self::Tuple(v) => (*v.as_ref()).into(),
        }
    }
}
pub fn hashes(row: &Value) -> types::Hashes {
    types::Hashes {
        x: field(&row["result"]["x"]),
        y: field(&row["result"]["y"]),
        transient: field(&row["result"]["transient"]),
    }
}
pub fn digest(row: &Value) -> types::Digest {
    types::Digest {
        value: bytes(&row["result"]["value"]),
    }
}
