// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_unit_value_operations_source_fixture::{runtime as r, types};
use serde_json::Value;
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
pub fn point(value: &Value) -> r::JubjubPoint {
    let atoms = r::fab::Value::from((field(&value["x"]), field(&value["y"])));
    r::JubjubPoint::try_from(&*atoms).unwrap()
}
pub fn bytes(value: &Value) -> r::FixedBytes<32> {
    let data: Vec<u8> = serde_json::from_value(value.clone()).unwrap();
    r::FixedBytes(data.try_into().unwrap())
}
pub fn output(row: &Value) -> types::Result {
    // Independent TS modular result and explicit bytes; never construct a Rust
    // curve point to obtain the expected output being tested.
    types::Result {
        x: field(&row["point"]["x"]),
        y: field(&row["point"]["y"]),
        blank: r::FixedBytes([0; 32]),
    }
}
