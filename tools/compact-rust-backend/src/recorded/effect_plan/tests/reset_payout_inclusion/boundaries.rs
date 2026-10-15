// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Old shape refusals are not typed invariant failures or inclusion evidence.
use super::*;

#[test]
fn false_computed_and_repeated_resets_are_expansions_not_old_inclusion() {
    for mode in 0..3 {
        let mut c = source();
        if mode == 2 {
            let original = root(&mut c).take();
            *root(&mut c) = json!({"kind":"sequence","steps":[call("reset_state",vec![json!({"kind":"boolean","value":true})])],"value":original});
        } else {
            assert!(first(root(&mut c), &mut |v| {
                if v["kind"] == "call" && v["name"] == "reset_state" {
                    v["arguments"][0] = if mode == 0 {
                        json!({"kind":"boolean","value":false})
                    } else {
                        json!({"kind":"equal","left":{"kind":"boolean","value":true},"right":{"kind":"boolean","value":true}})
                    };
                    true
                } else {
                    false
                }
            }));
        }
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let c: Contract = serde_json::from_value(c).unwrap();
                let env = Environment::new(&c);
                let root = env.stateful["cash_out"];
                assert!(env.old(root).is_none(), "shape expansion {mode}");
                render(&env.checked(root).unwrap()).unwrap();
                current(&c, "cash_out", &format!("shape-expansion-{mode}"));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
#[test]
fn malformed_slots_and_escaped_caller_values_remain_typed_source_refusals() {
    for (mode, expected) in [
        (0, "unknown ledger field or inconsistent index"),
        (1, "unbound value"),
    ] {
        let mut c = source();
        if mode == 0 {
            assert!(first(root(&mut c), &mut |v| {
                if v["kind"] == "cell_read" && v["field"] == "beneficiary" {
                    v["index"] = json!(10);
                    true
                } else {
                    false
                }
            }));
        } else {
            assert!(first(circuit(&mut c, "reset_state"), &mut |v| {
                if v["kind"] == "cell_write" && v["field"] == "pot_has_coin" {
                    // A Boolean in the caller still must not enter the helper scope.
                    v["value"] = reference("caller_only");
                    true
                } else {
                    false
                }
            }));
            prepend(
                &mut c,
                vec![binding(
                    "caller_only",
                    json!({"kind":"boolean"}),
                    json!({"kind":"boolean","value":false}),
                )],
            );
        }
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        let root = env.stateful["cash_out"];
        assert!(env.old(root).is_none());
        let Err(PlanError::Unsupported(gap)) = env.checked(root) else {
            panic!("malformed source {mode} must refuse before checked-plan validation")
        };
        assert!(gap.detail.contains(expected), "{gap:?}");
        assert!(gap.path.starts_with("return_value"), "{gap:?}");
    }
}
#[test]
fn old_root_and_reset_grammar_boundaries_are_audited_even_when_unused() {
    for mode in 0..7 {
        let mut c = source();
        match mode {
            0 => circuit(&mut c, "cash_out")["internal"] = json!(true),
            1 => {
                circuit(&mut c, "cash_out")["parameters"] =
                    json!([{"name":"unused","ty":{"kind":"boolean"}}])
            }
            2 => circuit(&mut c, "reset_state")["internal"] = json!(false),
            3 => {
                let helper = circuit(&mut c, "reset_state");
                let actions = helper["actions"].take();
                helper["actions"] = json!([{"kind":"let","bindings":[binding("hidden",serde_json::to_value(crate::ir::NativeWitnessBuiltin::OwnPublicKey.result_type()).unwrap(),json!({"kind":"native_witness_call","builtin":"own_public_key"}))],"action":{"kind":"sequence","actions":actions}}]);
            }
            4 => assert!(first(circuit(&mut c, "reset_state"), &mut |v| {
                if v["kind"] == "counter_increment" {
                    v["amount"] = json!({"kind":"literal","value":2});
                    true
                } else {
                    false
                }
            })),
            5 => {
                let original = root(&mut c).take();
                let ty = circuit(&mut c, "cash_out")["result"].clone();
                *root(&mut c) = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":original,"otherwise":{"kind":"default","ty":ty}});
            }
            6 => {
                let helper = circuit(&mut c, "reset_state");
                helper["actions"] = json!([{"kind":"sequence","actions":[]}]);
            }
            _ => unreachable!(),
        }
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        assert!(
            env.old(env.stateful["cash_out"]).is_none(),
            "old boundary {mode}"
        );
        // These are old admission boundaries only, not universal new refusals.
    }
}
