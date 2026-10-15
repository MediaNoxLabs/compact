// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_aligned_hashes_source_fixture::runtime as r;
/// Parse captured canonical decimal fields without a new numeric dependency.
fn field(text: &str) -> r::Field {
    text.bytes().fold(r::Field::from(0_u64), |value, digit| {
        assert!(digit.is_ascii_digit());
        value * r::Field::from(10_u64) + r::Field::from(u64::from(digit - b'0'))
    })
}
fn answer(input: u64) -> serde_json::Value {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../oracle/typescript.json")).unwrap();
    let input_text = input.to_string();
    let rows = oracle["knownAnswers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["input"].as_str() == Some(input_text.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    rows[0].clone()
}
/// Official TS answers are frozen separately from the Rust hash under test.
pub fn point(input: u64) -> r::JubjubPoint {
    let row = answer(input);
    r::construct_jubjub_point(
        field(row["point"]["x"].as_str().unwrap()),
        field(row["point"]["y"].as_str().unwrap()),
    )
    .unwrap()
}
pub fn digest(input: u64) -> r::FixedBytes<32> {
    let row = answer(input);
    r::FixedBytes(std::array::from_fn(|i| {
        u8::try_from(row["digest"][i].as_u64().unwrap()).unwrap()
    }))
}
