// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn malformed_product_declarations_and_lexical_references_remain_errors() {
    for case in [
        "root_type",
        "root_result",
        "pure_arity",
        "pure_type",
        "pure_result",
        "binding_type",
        "self_reference",
        "free_comparison",
        "coercion_type",
        "continuation_local",
    ] {
        let mut v = fixture();
        match case {
            "root_type" => root(&mut v)["parameters"][0]["ty"] = json!({"kind":"boolean"}),
            "root_result" => root(&mut v)["result"] = json!({"kind":"boolean"}),
            "pure_arity" => {
                pure(&mut v)["parameters"].as_array_mut().unwrap().pop();
            }
            "pure_type" => pure(&mut v)["parameters"][0]["ty"] = json!({"kind":"boolean"}),
            "pure_result" => pure(&mut v)["result"] = json!({"kind":"boolean"}),
            "binding_type" => condition(&mut v)["bindings"][0]["ty"] = json!({"kind":"boolean"}),
            "self_reference" => {
                condition(&mut v)["bindings"][0]["value"]["left"]["value"]["name"] = json!("t_13")
            }
            "free_comparison" => condition(&mut v)["body"]["left"]["name"] = json!("escaped"),
            "coercion_type" => {
                root(&mut v)["actions"][0]["arguments"][0]["ty"] = json!({"kind":"boolean"})
            }
            "continuation_local" => {
                root(&mut v)["actions"][1]["action"]["amount"]["name"] = json!("missing")
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c).unwrap().is_none(), "old shape {case}");
        assert!(checked(&c).is_err(), "checked {case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
}

#[test]
fn old_prefix_admission_never_substitutes_for_native_slot_or_identifier_validation() {
    for case in [
        "missing_slot",
        "wrong_kind",
        "wrong_index",
        "bad_slot_name",
        "bad_pure_name",
    ] {
        let mut v = fixture();
        match case {
            "missing_slot" => root(&mut v)["actions"][1]["action"]["field"] = json!("missing"),
            "wrong_kind" => {
                root(&mut v)["actions"][1]["action"]["field"] = json!("lastDiff");
                root(&mut v)["actions"][1]["action"]["index"] = json!(0);
            }
            "wrong_index" => root(&mut v)["actions"][1]["action"]["index"] = json!(99),
            "bad_slot_name" => rename(&mut v, &[("mixedOps", "bad counter")]),
            "bad_pure_name" => rename(&mut v, &[("assertProductLE", "bad pure")]),
            _ => unreachable!(),
        }
        let c = contract(v);
        if case == "bad_pure_name" {
            assert!(old(&c).is_err(), "the old emitter's fallible ident site");
        } else {
            assert!(
                old(&c).unwrap().is_some(),
                "old prefix does not validate slot authority: {case}"
            );
        }
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "mandatory native validation {case}");
        assert!(
            rows.as_array().unwrap().is_empty(),
            "native error precedes recorded routing {case}"
        );
    }
}

#[test]
fn hidden_stateful_unsupported_children_still_refuse_recording() {
    let mut v = fixture();
    root(&mut v)["actions"].as_array_mut().unwrap().push(json!({
        "kind":"if", "condition":{"kind":"boolean","value":true},
        "then":{"kind":"sequence","actions":[]},
        "otherwise":{"kind":"expression","value":{"kind":"persistent_commit",
            "value":(serde_json::to_value(crate::recorded::test_support::unsupported_fixed_input()).unwrap()),
            "opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}
    }));
    let c = crate::recorded::test_support::with_unsupported_fold(contract(v));
    assert!(old(&c).unwrap().is_none());
    assert!(
        checked(&c).is_err(),
        "unselected stateful children are checked"
    );
    let output = crate::render_with_capabilities(&c).unwrap();
    let root = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "recordPinned")
        .unwrap();
    assert!(!root.recorded && !root.observed_call);
    assert!(
        output.source.contains("persistent_commit"),
        "native source retains the unsupported recording child"
    );
}
