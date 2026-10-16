// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn looked_up_value_is_used_exactly_once_in_the_old_argument_rule() {
    for case in [
        "zero_lookup_args",
        "two_lookup_args",
        "no_assert",
        "outer_coerce",
    ] {
        let mut v = isolated();
        match case {
            "zero_lookup_args" => {
                let ty = sequence(&mut v)[1]["bindings"][0]["ty"].clone();
                root(&mut v)["parameters"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name":"supplied","ty":ty}));
                call(&mut v)["arguments"][1]["value"]["name"] = json!("supplied");
            }
            "two_lookup_args" => {
                let mut param = pure(&mut v)["parameters"][1].clone();
                param["name"] = json!("second_record");
                pure(&mut v)["parameters"]
                    .as_array_mut()
                    .unwrap()
                    .push(param);
                let arg = call(&mut v)["arguments"][1].clone();
                call(&mut v)["arguments"].as_array_mut().unwrap().push(arg);
            }
            "no_assert" => pure(&mut v)["body"] = json!({"kind":"unit"}),
            "outer_coerce" => {
                let body = pure(&mut v)["body"].clone();
                pure(&mut v)["body"] = json!({"kind":"coerce","ty":{"kind":"unit"},"value":body});
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c).unwrap().is_none(), "old {case}");
        // These are legitimate composed calls, not missing lookups: membership
        // and lookup still run exactly once before the certified pure function.
        assert_current(&c);
    }
}
#[test]
fn immutable_map_and_pure_lexical_authority_reject_malformed_inputs() {
    for case in [
        "wrong_slot",
        "wrong_index",
        "missing_local",
        "wrong_key_type",
        "wrong_lookup_type",
        "coerce_type",
        "pure_arity",
        "free_pure",
        "field_index",
        "shadow_pure_let",
    ] {
        let mut v = isolated();
        match case {
            "wrong_slot" => {
                sequence(&mut v)[1]["bindings"][0]["value"]["field"] = json!("custodyGrants")
            }
            "wrong_index" => sequence(&mut v)[1]["bindings"][0]["value"]["index"] = json!(99),
            "missing_local" => call(&mut v)["arguments"][1]["value"]["name"] = json!("escaped"),
            "wrong_key_type" => {
                root(&mut v)["actions"][0]["bindings"][0]["ty"] = json!({"kind":"boolean"})
            }
            "wrong_lookup_type" => {
                sequence(&mut v)[1]["bindings"][0]["ty"] = json!({"kind":"boolean"})
            }
            "coerce_type" => call(&mut v)["arguments"][1]["ty"] = json!({"kind":"boolean"}),
            "pure_arity" => {
                pure(&mut v)["parameters"].as_array_mut().unwrap().pop();
            }
            "free_pure" => {
                pure(&mut v)["body"]["steps"][0]["condition"]["left"]["name"] = json!("escaped")
            }
            "field_index" => {
                pure(&mut v)["body"]["steps"][0]["condition"]["right"]["index"] = json!(99)
            }
            "shadow_pure_let" => {
                pure(&mut v)["body"] = json!({"kind":"let","bindings":[{"name":"currentTime","ty":{"kind":"boolean"},"value":{"kind":"parameter","name":"currentTime"}}],"body":{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"guard"}});
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c).unwrap().is_none(), "old {case}");
        assert!(checked(&c).is_err(), "checked {case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
}
#[test]
fn native_identifier_errors_precede_all_recording_attempts() {
    for (from, to) in [
        ("records", "bad slot"),
        (PURE, "bad pure"),
        ("recordId", "bad parameter"),
        ("AssetRecord", "bad struct"),
        ("registeredAt", "bad member"),
    ] {
        let mut v = isolated();
        rename(&mut v, from, to);
        let c = contract(v);
        // Fallible old ident/type sites and mandatory native validation are
        // different authorities; do not replace a native failure with a gap.
        if matches!(from, "records" | PURE | "AssetRecord") {
            assert!(old(&c).is_err(), "old {from}");
        }
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "native {from}");
        assert!(
            rows.as_array().unwrap().is_empty(),
            "native precedence {from}"
        );
    }
}
