// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn declared_witness_slot_and_projection_authorities_remain_independent() {
    for case in [
        "witness_arity",
        "argument_type",
        "presence_name",
        "presence_index",
        "tree_index",
        "phase_index",
        "insert_type",
        "leaf",
        "digest",
        "direction",
    ] {
        let mut v = fixture();
        match case {
            "witness_arity" => {
                named_mut(&mut v, "witnesses", PATH_WITNESS)["parameters"] = json!([])
            }
            "argument_type" => {
                root_actions(&mut v)[0]["condition"]["condition"]["value"]["arguments"][0] =
                    json!({"kind":"boolean","value":false})
            }
            "presence_name" => {
                root_actions(&mut v)[0]["condition"]["condition"]["field"] = json!("missing")
            }
            "presence_index" => {
                root_actions(&mut v)[0]["condition"]["condition"]["index"] = json!(9)
            }
            "tree_index" => continuation(&mut v)[2]["index"] = json!(99),
            "phase_index" => continuation(&mut v)[1]["condition"]["left"]["index"] = json!(99),
            "insert_type" => {
                continuation(&mut v)[2]["value"] = json!({"kind":"boolean","value":true})
            }
            // Mutating one occurrence while another witness retains the original
            // named type checks canonical definition authority, not unused data.
            "leaf" => {
                named_mut(&mut v, "witnesses", PATH_WITNESS)["result"]["fields"][1]["ty"]["fields"]
                    [0]["ty"] = json!({"kind":"field"})
            }
            "digest" => {
                named_mut(&mut v, "witnesses", PATH_WITNESS)["result"]["fields"][1]["ty"]["fields"]
                    [1]["ty"]["element"]["fields"][0]["ty"]["fields"][0]["ty"] =
                    json!({"kind":"boolean"})
            }
            "direction" => {
                named_mut(&mut v, "witnesses", PATH_WITNESS)["result"]["fields"][1]["ty"]["fields"]
                    [1]["ty"]["element"]["fields"][1]["ty"] = json!({"kind":"field"})
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
fn old_fallible_sites_keep_native_error_precedence() {
    for (from, to) in [
        (PATH_WITNESS, "bad witness"),
        ("is_some", "bad presence"),
        ("MaybeCompact1", "bad optional"),
        ("state", "bad phase"),
        ("eligible_voters", "bad tree"),
        ("PublicState", "bad enum"),
        ("setup", "bad variant"),
    ] {
        let c = contract(renamed(fixture(), &[(from, to)]));
        assert!(old(&c).is_err(), "old error {from}");
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err());
        assert!(rows.as_array().unwrap().is_empty());
    }
    let mut v = fixture();
    continuation(&mut v)[1]["condition"]["right"]["variant"] = json!("Missing");
    let c = contract(v);
    assert!(
        old(&c).is_err(),
        "old renderer evaluates this enum expression"
    );
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(result.is_err());
    assert!(rows.as_array().unwrap().is_empty());
}

#[test]
fn original_signature_and_hidden_direct_effect_refusals_remain_explicit() {
    let mut v = fixture();
    named_mut(&mut v, "stateful_circuits", "add_voter")["result"] = json!({"kind":"boolean"});
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    assert!(checked(&c).is_err());
    assert!(crate::render_with_capabilities(&c).is_err());

    let mut v = fixture();
    root_actions(&mut v).as_array_mut().unwrap().push(json!({
        "kind":"if","condition":{"kind":"boolean","value":true},
        "then":{"kind":"sequence","actions":[]},
        "otherwise":{"kind":"expression","value":{"kind":"persistent_commit",
            "value":(serde_json::to_value(crate::recorded::test_support::unsupported_fixed_input()).unwrap()),"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}
    }));
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    assert!(checked(&c).is_err());
    let result = crate::render_with_capabilities(&c).unwrap();
    let cap = result
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "add_voter")
        .unwrap();
    assert!(!cap.recorded && !cap.observed_call);
}
