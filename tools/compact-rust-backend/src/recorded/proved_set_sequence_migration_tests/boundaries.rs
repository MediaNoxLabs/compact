// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn predicate_truth_does_not_override_set_or_witness_declaration_authority() {
    for mode in [
        "member_index",
        "insert_index",
        "member_kind",
        "insert_kind",
        "missing_witness",
        "witness_arity",
        "witness_type",
    ] {
        let mut v = fixture();
        match mode {
            "member_index" => root(&mut v)["actions"][0]["condition"]["index"] = json!(99),
            "insert_index" => root(&mut v)["actions"][1]["index"] = json!(99),
            "member_kind" => v["ledger_fields"][1]["declaration"] = json!({"kind":"counter"}),
            "insert_kind" => v["ledger_fields"][2]["declaration"] = json!({"kind":"counter"}),
            "missing_witness" => v["witnesses"]
                .as_array_mut()
                .unwrap()
                .retain(|w| w["name"] != "set_local_id"),
            "witness_arity" => named(&mut v, "witnesses", "set_local_id")["parameters"] = json!([]),
            "witness_type" => {
                named(&mut v, "witnesses", "set_local_id")["parameters"][0]["ty"] =
                    json!({"kind":"field"})
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        let cx = Context::new(&c);
        assert!(
            proved_opaque_set_sequence(cx.circuits["check_in"]),
            "predicate ignores declarations {mode}"
        );
        assert!(
            cx.checked(cx.circuits["check_in"]).is_err(),
            "checked {mode}"
        );
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "native {mode}");
        assert!(rows.as_array().unwrap().is_empty(), "native first {mode}");
    }
}
#[test]
fn malformed_names_layout_and_operand_scopes_fail_before_recording() {
    for mode in [
        "witness_name",
        "root_name",
        "layout",
        "free_key",
        "free_witness_arg",
    ] {
        let mut v = fixture();
        match mode {
            "witness_name" => rename(&mut v, "set_local_id", "bad witness"),
            "root_name" => root(&mut v)["name"] = json!("bad root"),
            "layout" => v["ledger_fields"][1]["path"] = json!([2, 7]),
            "free_key" => root(&mut v)["actions"][1]["value"]["name"] = json!("escaped"),
            "free_witness_arg" => {
                root(&mut v)["actions"][2]["value"]["arguments"][0]["value"]["name"] =
                    json!("escaped")
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "{mode}");
        assert!(rows.as_array().unwrap().is_empty(), "native first {mode}");
    }
}
#[test]
fn unsupported_first_effect_and_recursive_helper_do_not_skip_to_the_old_permit() {
    let mut v = fixture();
    root(&mut v)["actions"].as_array_mut().unwrap().insert(0,json!({"kind":"expression","value":{"kind":"persistent_commit","value":{"kind":"field_literal","value":"0"},"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}));
    let c = contract(v);
    let cx = Context::new(&c);
    let Err(effect_plan::PlanError::Unsupported(gap)) = cx.checked(cx.circuits["check_in"]) else {
        panic!("unsupported first effect")
    };
    assert_eq!(gap.path, "actions[0].value");
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == "check_in" && !r.recorded && !r.observed_call)
    );
    let mut v = fixture();
    root(&mut v)["actions"] = json!([{"kind":"circuit_call","name":"check_in","arguments":[{"kind":"parameter","name":"participant"}]}]);
    let c = contract(v);
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(result.is_err());
    assert!(rows.as_array().unwrap().is_empty());
}
