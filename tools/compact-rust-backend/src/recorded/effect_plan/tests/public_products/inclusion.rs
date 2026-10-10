// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite complete historic-spend controls; whole old admission is retained as a test oracle.
use super::*;
use serde_json::{Value as Json, json};

fn add_member(value: &mut Json, ty: &Json) {
    match value {
        Json::Object(object) => {
            if object.get("kind") == Some(&json!("struct"))
                && object.get("name") == Some(&json!("public_key"))
            {
                object
                    .get_mut("fields")
                    .unwrap()
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name":"unused_extra","ty":ty}));
            }
            for child in object.values_mut() {
                add_member(child, ty);
            }
        }
        Json::Array(items) => {
            for child in items {
                add_member(child, ty);
            }
        }
        _ => {}
    }
}
fn old_first(c: &Contract, label: &str) {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|r| (r.name.as_str(), r)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    assert!(
        crate::recorded::typed_plan::lower(
            circuits["spend"],
            &ledger,
            &witnesses,
            &pure,
            &circuits
        )
        .is_some(),
        "{label}: whole old oracle first"
    );
    let checked = check_with_calls(circuits["spend"], &ledger, &witnesses, &pure, &circuits)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    render(&checked).unwrap();
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(c)
    });
    let output = output.unwrap();
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["circuit"] == "spend" && row["phase"] == "public_emission" && row["helper"] == false
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "algebra.membership");
    if let Some(destination) = std::env::var_os("COMPACT_PUBLIC_PRODUCT_PROBE_OUTPUT") {
        let base = std::path::PathBuf::from(destination);
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join(label);
        std::fs::create_dir(&dir).expect("use a fresh public product export directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(c).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
    }
}
#[test]
fn complete_old_historic_spend_public_products_now_pass_checked_lowering() {
    let bytes = json!({"kind":"opaque_bytes"});
    for (label, ty) in [
        (
            "tuple",
            json!({"kind":"tuple","elements":[bytes,{"kind":"field"}]}),
        ),
        (
            "vector",
            json!({"kind":"vector","element":bytes,"length":2}),
        ),
        (
            "wide",
            json!({"kind":"unsigned","max":"340282366920938463463374607431768211456"}),
        ),
        (
            "empty",
            json!({"kind":"struct","name":"UnusedEmpty","fields":[]}),
        ),
    ] {
        let mut source: Json = serde_json::from_str(include_str!(
            "../../../../../tests/zerocash-oracle-schema20-ir.json"
        ))
        .unwrap();
        add_member(&mut source, &ty);
        old_first(&serde_json::from_value(source).unwrap(), label);
    }
}

#[test]
fn genuine_product_source_uses_existing_composite_ownership() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-public-products-schema20-ir.json"
    ))
    .unwrap();
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(&source)
    });
    let output = output.unwrap();
    assert_eq!(output.capabilities.circuits.len(), 4);
    for name in ["run", "tuple11", "vector9", "empty"] {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call, "{name}");
        let owners = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["circuit"] == name
                    && row["phase"] == "public_emission"
                    && row["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(owners.len(), 1, "{name}");
        assert_eq!(
            owners[0]["body_owner"], "algebra.composite-values",
            "{name}"
        );
    }
}
