// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_call_arg_declared_type_fixture::pure_circuits as p;
use midnight_compact_runtime::{Field, FixedVector};
use serde_json::Value;
use std::collections::BTreeSet;
fn field(v: &Value) -> Field {
    v.as_str().unwrap().bytes().fold(Field::from(0u64), |n, d| {
        assert!(d.is_ascii_digit());
        n * Field::from(10u64) + Field::from(u64::from(d - b'0'))
    })
}
#[test]
fn seven_pure_exports_have_direct_independent_typescript_results() {
    let capture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/direct-call-registry.json"
    ))
    .unwrap();
    let rows = capture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["group"] == "call")
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 13);
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        let name = row["export"].as_str().unwrap();
        names.insert(name);
        assert!(ids.insert(row["id"].as_str().unwrap()));
        let a = &row["args"];
        let output = match name {
            "idf" => p::idf(field(&a[0])),
            "sumVec" => p::sumVec(FixedVector::new([field(&a[0][0]), field(&a[0][1])])),
            "sumTup" => p::sumTup((field(&a[0][0]), field(&a[0][1]))),
            "vecFromPureBody" => p::vecFromPureBody(),
            "fieldOnlyFromPureBody" => p::fieldOnlyFromPureBody(),
            "tupleIntoVec" => p::tupleIntoVec(),
            "vecIntoTuple" => p::vecIntoTuple(),
            _ => panic!("unreviewed export {name}"),
        }
        .unwrap();
        assert_eq!(row["ok"], true);
        assert_eq!(
            hex::encode(output.as_le_bytes()),
            row["result"],
            "{}",
            row["id"]
        );
    }
    assert_eq!(
        names,
        BTreeSet::from([
            "idf",
            "sumVec",
            "sumTup",
            "vecFromPureBody",
            "fieldOnlyFromPureBody",
            "tupleIntoVec",
            "vecIntoTuple"
        ])
    );
}
