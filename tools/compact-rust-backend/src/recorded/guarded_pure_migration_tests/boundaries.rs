// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn declared_counter_paths_and_resource_limits_remain_authoritative() {
    let mut v = fixture();
    v["ledger_fields"].as_array_mut().unwrap().insert(
        0,
        json!({"id":"padding","index":0,"path":[0],"declaration":{"kind":"counter"}}),
    );
    v["ledger_fields"][1]["index"] = json!(1);
    v["ledger_fields"][1]["path"] = json!([1]);
    root(&mut v)["actions"][1]["action"]["index"] = json!(1);
    assert_old_positive(&contract(v), "recordFreshEnough", "record_increment");
    for case in ["missing_slot", "wrong_kind", "wrong_index", "wrong_amount"] {
        let mut v = fixture();
        match case {
            "missing_slot" => root(&mut v)["actions"][1]["action"]["field"] = json!("missing"),
            "wrong_kind" => {
                v["ledger_fields"][0]["declaration"] = json!({"kind":"cell","ty":{"kind":"field"}})
            }
            "wrong_index" => root(&mut v)["actions"][1]["action"]["index"] = json!(99),
            "wrong_amount" => {
                root(&mut v)["actions"][1]["action"]["amount"] =
                    json!({"kind":"parameter","name":"policy"})
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        if case != "wrong_amount" {
            assert!(
                old_prefix(&c, "recordFreshEnough").unwrap().is_some(),
                "old prefix does not validate slot: {case}"
            );
        }
        assert!(checked(&c, "recordFreshEnough").is_err(), "{case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
    // Reuse #597's whole native-valid oversized Map input as an explicit checked
    // policy boundary. Retirement does not promise out-of-budget inclusion.
    let c = contract(crate::capabilities::resource_diagnostic_tests::oversized());
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&c, "assertStoredRecordFresh")
    else {
        panic!("typed resource refusal")
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

#[test]
fn native_errors_precede_routing_and_preserved_oracle_is_not_production_authority() {
    for case in [
        "bad_pure_name",
        "bad_slot_name",
        "bad_field",
        "pure_type",
        "recursive_helper",
    ] {
        let mut v = fixture();
        match case {
            "bad_pure_name" => rename(&mut v, &[("assertFreshEnough", "bad pure")]),
            "bad_slot_name" => rename(&mut v, &[("accepted", "bad slot")]),
            "bad_field" => {
                pure(&mut v)["body"]["steps"][0]["condition"]["right"]["field"] = json!("missing")
            }
            "pure_type" => {
                pure(&mut v)["body"]["steps"][0]["condition"] =
                    json!({"kind":"field_literal","value":"0"})
            }
            "recursive_helper" => {
                helper(&mut v, "finish", json!([call("finish")]));
                root(&mut v)["actions"][1] = call("finish");
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "native {case}");
        assert!(rows.as_array().unwrap().is_empty(), "native first {case}");
    }
    // Direct old oracle still recognizes its exact shape, while the live trace
    // has only checked ownership and no eager guard evaluation.
    let c = contract(fixture());
    assert!(old_complete(&c, "recordFreshEnough").unwrap());
    assert_current(&c, "recordFreshEnough");
}
