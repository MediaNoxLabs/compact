// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_jubjub_negation_source_fixture::{runtime as r, types};
use serde_json::Value;
fn field(value: &Value) -> r::Field {
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
    let value = r::fab::Value::from((field(&value["x"]), field(&value["y"])));
    r::JubjubPoint::try_from(&*value).unwrap()
}
pub fn coordinates(value: &Value) -> types::Coordinates {
    types::Coordinates {
        x: field(&value["x"]),
        y: field(&value["y"]),
    }
}
