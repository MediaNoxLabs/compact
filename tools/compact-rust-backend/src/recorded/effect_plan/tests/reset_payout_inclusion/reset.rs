// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn reset_action_sequences_shadowing_and_literal_increment_compose() {
    for (mode, label) in [
        "reset-sequence",
        "reset-shadowed-binding",
        "reset-literal-increment",
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = source();
        let helper = circuit(&mut c, "reset_state");
        match mode {
            0 => {
                let actions = helper["actions"].take();
                helper["actions"] = json!([{"kind":"sequence","actions":actions}]);
            }
            1 => {
                let actions = helper["actions"].take();
                helper["actions"] = json!([{"kind":"let","bindings":[
                    binding("reset_pot", json!({"kind":"boolean"}), reference("reset_pot")),
                    binding("local_one",json!({"kind":"unsigned","max":"65535"}),json!({"kind":"unsigned_literal","value":"1","max":"65535"}))
                ],"action":{"kind":"sequence","actions":actions}}]);
            }
            2 => assert!(first(helper, &mut |v| {
                if v["kind"] == "counter_increment" {
                    v["amount"] = json!({"kind":"literal","value":1});
                    true
                } else {
                    false
                }
            })),
            _ => unreachable!(),
        }
        included(c, "cash_out", label);
    }
}
#[test]
fn both_reset_branches_and_qualified_default_values_are_typed() {
    let mut c = source();
    let helper = circuit(&mut c, "reset_state");
    assert!(first(helper, &mut |v| {
        if v["kind"] == "if" {
            v["condition"] = json!({"kind":"let","bindings":[binding("chosen",json!({"kind":"boolean"}),reference("reset_pot"))],"body":{
                "kind":"equal","left":reference("chosen"),"right":{"kind":"boolean","value":true}
            }});
            v["otherwise"] = v["then"].clone();
            true
        } else {
            false
        }
    }));
    included(c, "cash_out", "both-reset-branches");

    let mut c = source();
    let helper = circuit(&mut c, "reset_state");
    assert!(first(helper, &mut |v| {
        if v["kind"] == "default" && v["ty"]["name"] == "QualifiedShieldedCoinInfo" {
            let original = v.clone();
            let ty = v["ty"].clone();
            *v = json!({"kind":"let","bindings":[binding("default_coin",ty.clone(),original.clone())],"body":{
                "kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"coerce","value":reference("default_coin"),"ty":ty},"otherwise":original
            }});
            true
        } else {
            false
        }
    }));
    included(c, "cash_out", "qualified-default-through-values");
}
#[test]
fn reset_evidence_is_a_family_not_an_exact_number_of_writes() {
    let mut c = source();
    let helper = circuit(&mut c, "reset_state");
    let extra = helper["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["kind"] == "counter_reset")
        .unwrap()
        .clone();
    helper["actions"].as_array_mut().unwrap().push(extra);
    included(c, "cash_out", "extra-reset-write");
}
