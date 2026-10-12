// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn malformed_declaration_and_lexical_authorities_remain_refused() {
    for case in [
        "root_type",
        "root_result",
        "member_index",
        "insert_index",
        "witness_arity",
        "witness_result",
        "hash_parameter",
        "hash_result",
        "helper_arity",
        "helper_result",
        "missing_insert_local",
    ] {
        let mut v = fixture();
        match case {
            "root_type" => {
                named(&mut v, "stateful_circuits", "add_participant")["parameters"][0]["ty"] =
                    json!({"kind":"field"})
            }
            "root_result" => {
                named(&mut v, "stateful_circuits", "add_participant")["result"] =
                    json!({"kind":"boolean"})
            }
            "member_index" => {
                named(&mut v, "stateful_circuits", "add_participant")["actions"][0]["condition"]["body"]
                    ["index"] = json!(99)
            }
            "insert_index" => {
                named(&mut v, "stateful_circuits", "add_participant")["actions"][1]["index"] =
                    json!(99)
            }
            "witness_arity" => {
                named(&mut v, "witnesses", "local_sk")["parameters"] =
                    json!([{"name":"extra","ty":{"kind":"field"}}])
            }
            "witness_result" => {
                named(&mut v, "witnesses", "local_sk")["result"] = json!({"kind":"boolean"})
            }
            "hash_parameter" => {
                named(&mut v, "circuits", "public_key")["parameters"][0]["ty"] =
                    json!({"kind":"boolean"})
            }
            "hash_result" => {
                named(&mut v, "circuits", "public_key")["result"] = json!({"kind":"field"})
            }
            "helper_arity" => {
                named(&mut v, "stateful_circuits", "local_sk_or_error")["parameters"] =
                    json!([{"name":"extra","ty":{"kind":"field"}}])
            }
            "helper_result" => {
                named(&mut v, "stateful_circuits", "local_sk_or_error")["result"] =
                    json!({"kind":"boolean"})
            }
            "missing_insert_local" => {
                named(&mut v, "stateful_circuits", "add_participant")["actions"][1]["value"]["name"] =
                    json!("missing")
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c, "add_participant").unwrap().is_none(), "old {case}");
        assert!(checked(&c, "add_participant").is_err(), "checked {case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
}

#[test]
fn former_fallible_emission_sites_still_fail_before_recorded_selection() {
    for (from, to) in [
        ("local_sk", "bad witness"),
        ("public_key", "bad hash"),
        ("organizer_pks", "bad authority slot"),
        ("eligible_participants", "bad destination slot"),
        ("MaybeCompact1", "bad optional type"),
    ] {
        let mut v = fixture();
        rename(&mut v, &[(from, to)]);
        let c = contract(v);
        assert!(old(&c, "add_participant").is_err(), "old fallible {from}");
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err());
        assert!(rows.as_array().unwrap().is_empty());
    }
    // Old organizer inlines the helper and does not emit its identifier, but
    // mandatory native rendering rejects an invalid declaration first.
    let mut v = fixture();
    rename(&mut v, &[("local_sk_or_error", "bad helper")]);
    let c = contract(v);
    assert!(old(&c, "add_participant").unwrap().is_some());
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(result.is_err());
    assert!(rows.as_array().unwrap().is_empty());
}

#[test]
fn fixed_member_spellings_are_not_new_semantic_constraints_and_hidden_effects_are_checked() {
    let mut v = fixture();
    rename(&mut v, &[("is_some", "present"), ("value", "payload")]);
    let c = contract(v);
    for name in ROOTS {
        assert!(old(&c, name).unwrap().is_none());
        assert!(checked(&c, name).is_ok());
    }
    assert_actual(&c);

    let mut v = fixture();
    named(&mut v,"stateful_circuits","add_participant")["actions"].as_array_mut().unwrap().push(json!({
        "kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"sequence","actions":[]},
        "otherwise":{"kind":"expression","value":{"kind":"persistent_commit","value":{"kind":"field_literal","value":"0"},"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}
    }));
    let c = contract(v);
    assert!(old(&c, "add_participant").unwrap().is_none());
    assert!(checked(&c, "add_participant").is_err());
    let rendered = crate::render_with_capabilities(&c).unwrap();
    let cap = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "add_participant")
        .unwrap();
    assert!(!cap.recorded && !cap.observed_call);
}
