// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn old_signature_prefix_and_continuation_refusals_do_not_hide_invalid_source() {
    for case in [
        "root_result",
        "witness_result",
        "witness_arity",
        "hash_parameter",
        "hash_result",
        "authority_slot",
        "phase_type",
        "value_annotation",
        "write_value",
    ] {
        let mut value = fixture();
        match case {
            "root_result" => root_mut(&mut value)["result"] = json!({"kind":"boolean"}),
            "witness_result" => {
                circuit_mut(&mut value, "witnesses", "private$secret_key")["result"] =
                    json!({"kind":"boolean"})
            }
            "witness_arity" => {
                circuit_mut(&mut value, "witnesses", "private$secret_key")["parameters"] =
                    json!([{"name":"extra","ty":{"kind":"field"}}])
            }
            "hash_parameter" => {
                circuit_mut(&mut value, "circuits", "public_key")["parameters"][0]["ty"] =
                    json!({"kind":"boolean"})
            }
            "hash_result" => {
                circuit_mut(&mut value, "circuits", "public_key")["result"] =
                    json!({"kind":"field"})
            }
            "authority_slot" => {
                continuation(&mut value)[0]["condition"]["right"]["index"] = json!(99)
            }
            "phase_type" => {
                continuation(&mut value)[1]["condition"]["right"] =
                    json!({"kind":"boolean","value":true})
            }
            "value_annotation" => {
                continuation(&mut value)[2]["bindings"][0]["ty"] = json!({"kind":"boolean"})
            }
            "write_value" => {
                continuation(&mut value)[2]["action"]["value"] =
                    json!({"kind":"parameter","name":"t"})
            }
            _ => unreachable!(),
        }
        let c = contract(value);
        assert!(old(&c).unwrap().is_none(), "old {case}");
        assert!(checked(&c).is_err(), "checked {case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
}

#[test]
fn former_eager_error_sites_fail_before_any_recorded_attempt() {
    for case in [
        "witness_name",
        "hash_name",
        "authority_name",
        "phase_name",
        "write_name",
        "enum_name",
        "struct_name",
        "member_name",
        "variant_name",
        "missing_variant",
    ] {
        let mut value = fixture();
        let replacement = match case {
            "witness_name" => Some(("private$secret_key", "bad witness")),
            "hash_name" => Some(("public_key", "bad hash")),
            "authority_name" => Some(("authority", "bad authority")),
            "phase_name" => Some(("state", "bad phase")),
            "write_name" => Some(("topic", "bad slot")),
            "enum_name" => Some(("PublicState", "bad enum")),
            "struct_name" => Some(("Maybe", "bad struct")),
            "member_name" => Some(("value", "bad member")),
            "variant_name" => Some(("setup", "bad variant")),
            _ => None,
        };
        if let Some(pair) = replacement {
            replace_strings(&mut value, &[pair]);
        }
        if case == "missing_variant" {
            continuation(&mut value)[1]["condition"]["right"]["variant"] = json!("Absent");
        }
        let c = contract(value);
        assert!(old(&c).is_err(), "whole-old fallible site {case}");
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "public native validation {case}");
        assert!(
            rows.as_array().unwrap().is_empty(),
            "no recorded selector ran for {case}"
        );
    }
}

#[test]
fn public_signature_checks_and_hidden_children_remain_explicit() {
    let mut value = fixture();
    root_mut(&mut value)["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"extra","ty":{"kind":"field"}}));
    let c = contract(value);
    assert!(old(&c).unwrap().is_none());
    assert!(
        checked(&c).is_ok(),
        "existing Unit interface independently supports this signature"
    );

    for unselected in [false, true] {
        let mut value = fixture();
        let unsupported = json!({"kind":"expression","value":{"kind":"persistent_commit",
            "value":{"kind":"field_literal","value":"0"},
            "opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}});
        let action = if unselected {
            json!({"kind":"if", "condition":{"kind":"boolean","value":true},
            "then":{"kind":"sequence","actions":[]}, "otherwise":unsupported})
        } else {
            unsupported
        };
        continuation(&mut value)
            .as_array_mut()
            .unwrap()
            .push(action);
        let c = contract(value);
        assert!(old(&c).unwrap().is_none());
        assert!(checked(&c).is_err());
        let rendered = crate::render_with_capabilities(&c).unwrap();
        let root = rendered
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "set_topic")
            .unwrap();
        assert!(
            !root.recorded && !root.observed_call,
            "unselected={unselected}"
        );
    }
}
