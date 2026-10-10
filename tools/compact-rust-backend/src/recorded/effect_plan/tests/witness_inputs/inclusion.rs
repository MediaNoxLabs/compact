// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old regression controls for the compiled #557 research variants.
use super::*;
use serde_json::{Value as Json, json};

fn old_first(c: &Contract, name: &str, label: &str) {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|r| (r.name.as_str(), r)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    assert!(
        crate::recorded::typed_plan::lower(circuits[name], &ledger, &witnesses, &pure, &circuits)
            .is_some(),
        "{label}: complete old oracle first"
    );
    let plan = check_with_calls(circuits[name], &ledger, &witnesses, &pure, &circuits)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    render(&plan).unwrap();
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(c)
    });
    let output = output.unwrap();
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "selector.membership-optional");
    if let Some(destination) = std::env::var_os("COMPACT_WITNESS_INPUT_PROBE_OUTPUT") {
        let base = std::path::PathBuf::from(destination);
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join(label);
        std::fs::create_dir(&dir).expect("use a fresh witness probe export directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(c).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
    }
}
fn bboard(ty: Json, argument: Json, helper: Option<Json>) -> Contract {
    let mut c: Json =
        serde_json::from_str(include_str!("../../../../../tests/bboard-schema20-ir.json")).unwrap();
    let fixed = json!({"kind":"bytes","length":32});
    c["witnesses"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"probe_audit","parameters":[{"name":"input","ty":ty}],"result":fixed}));
    if let Some(helper) = helper {
        c["circuits"].as_array_mut().unwrap().push(helper);
    }
    let root = c["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["name"] == "post")
        .unwrap();
    root["actions"] = json!([{"kind":"let","bindings":[{"name":"audit_result","ty":fixed,"value":{"kind":"witness_call","name":"probe_audit","arguments":[argument]}}],"action":{"kind":"sequence","actions":root["actions"]}}]);
    serde_json::from_value(c).unwrap()
}
#[test]
fn native_compiled_old_membership_witness_argument_shapes_now_compose() {
    let string = json!({"kind":"opaque_string"});
    let parameter = json!({"kind":"parameter","name":"new_message"});
    old_first(
        &bboard(string.clone(), parameter.clone(), None),
        "post",
        "string",
    );
    let product =
        json!({"kind":"struct","name":"AuditProduct","fields":[{"name":"value","ty":string}]});
    old_first(
        &bboard(
            product.clone(),
            json!({"kind":"struct_literal","ty":product,"fields":[parameter]}),
            None,
        ),
        "post",
        "string-product",
    );
    for (label, count, vector) in [
        ("tuple2", 2, false),
        ("tuple9", 9, false),
        ("vector2", 2, true),
    ] {
        let tuple_ty = json!({"kind":"tuple","elements":vec![string.clone();count]});
        let ty = if vector {
            json!({"kind":"vector","element":string,"length":count})
        } else {
            tuple_ty
        };
        let tuple =
            json!({"kind":"tuple","elements":vec![json!({"kind":"parameter","name":"s"});count]});
        let body = if vector {
            json!({"kind":"coerce","ty":ty,"value":tuple})
        } else {
            tuple
        };
        let helper = json!({"name":"probe_wrap","internal":true,"parameters":[{"name":"s","ty":string}],"result":ty,"body":body});
        let argument = json!({"kind":"call","name":"probe_wrap","arguments":[{"kind":"parameter","name":"new_message"}]});
        old_first(&bboard(ty, argument, Some(helper)), "post", label);
    }
    // Recursive ownership must compose without introducing an aggregate
    // CellValue bound: this local vector holds two standalone tuple9 values.
    let tuple_ty = json!({"kind":"tuple","elements":vec![string.clone();9]});
    let ty = json!({"kind":"vector","element":tuple_ty,"length":2});
    let tuple = json!({"kind":"tuple","elements":vec![json!({"kind":"parameter","name":"s"});9]});
    let body =
        json!({"kind":"coerce","ty":ty,"value":{"kind":"tuple","elements":[tuple.clone(),tuple]}});
    let helper = json!({"name":"probe_wrap","internal":true,"parameters":[{"name":"s","ty":string}],"result":ty,"body":body});
    let argument = json!({"kind":"call","name":"probe_wrap","arguments":[{"kind":"parameter","name":"new_message"}]});
    old_first(&bboard(ty, argument, Some(helper)), "post", "vector-tuple9");
}
#[test]
fn fresh_witness_source_uses_actual_composite_ownership() {
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-witness-inputs-schema20-ir.json"
    ))
    .unwrap();
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(&c)
    });
    let output = output.unwrap();
    assert_eq!(output.capabilities.circuits.len(), 1);
    assert_eq!(output.capabilities.circuits[0].name, "run");
    assert!(
        output.capabilities.circuits[0].recorded && output.capabilities.circuits[0].observed_call
    );
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["circuit"] == "run" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "algebra.composite-values");
}
