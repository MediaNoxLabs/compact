// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn grammar_expansions_are_distinct_from_native_type_and_slot_authority() {
    let mut v = fixture();
    actions(&mut v).insert(3,json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"extra removal guard"}));
    let c = contract(v);
    assert!(!Context::new(&c).prefix());
    let text = assert_current(&c);
    assert!(text.find("extra removal guard").unwrap() < text.find("record_remove").unwrap());
    for mode in ["key_source", "index", "kind", "helper_arity"] {
        let mut v = isolated();
        match mode {
            "key_source" => actions(&mut v)[3]["key"]["name"] = json!("escaped"),
            "index" => actions(&mut v)[3]["index"] = json!(99),
            "kind" => {
                v["ledger_fields"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|f| f["id"] == "records")
                    .unwrap()["declaration"] = json!({"kind":"counter"})
            }
            "helper_arity" => {
                actions(&mut v)[0]["arguments"] = json!([{"kind":"boolean","value":true}])
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(!Context::new(&c).prefix(), "old prefix {mode}");
        assert!(Context::new(&c).checked().is_err(), "checked {mode}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {mode}"
        );
    }
}
#[test]
fn native_errors_precede_routing_and_resource_policy_remains_separate() {
    for mode in ["slot_name", "witness_name", "helper_name", "recursion"] {
        let mut v = isolated();
        match mode {
            "slot_name" => rename(&mut v, "records", "bad map"),
            "witness_name" => rename(&mut v, "currentTimestamp", "bad witness"),
            "helper_name" => rename(&mut v, "assertWritable", "bad helper"),
            "recursion" => {
                named(&mut v, "recordWrite")["actions"] =
                    json!([{"kind":"circuit_call","name":"recordWrite","arguments":[]}])
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "native {mode}");
        assert!(rows.as_array().unwrap().is_empty(), "native first {mode}");
    }
    // This is an independent value-decoding boundary, not a claim that key-only
    // removal decodes oversized Map values or that resource limits were relaxed.
    let c = contract(crate::capabilities::resource_diagnostic_tests::oversized());
    let cx = Context::new(&c);
    let root = cx.circuits["assertStoredRecordFresh"];
    let Err(effect_plan::PlanError::Unsupported(gap)) =
        effect_plan::lower_with_calls(root, &cx.ledger, &cx.witnesses, &cx.pure, &cx.circuits)
    else {
        panic!("typed resource refusal");
    };
    assert!(gap.resource.is_some());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == "assertStoredRecordFresh" && !r.recorded)
    );
}
