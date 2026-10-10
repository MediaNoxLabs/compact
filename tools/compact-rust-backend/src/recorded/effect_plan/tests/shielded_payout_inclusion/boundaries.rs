// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Native trait limits, malformed source and valid expansions have distinct premises.
use super::*;

#[test]
fn native_public_trait_limits_are_not_claimed_as_old_native_valid_inclusion() {
    for (label, ty) in [
        (
            "public-tuple12",
            json!({"kind":"tuple","elements":vec![json!({"kind":"field"});12]}),
        ),
        (
            "named-tuple9",
            json!({"kind":"struct","name":"UnencodablePublic","fields":[{"name":"tuple","ty":{"kind":"tuple","elements":vec![json!({"kind":"field"});9]}}]}),
        ),
    ] {
        let mut c = source();
        circuit(&mut c, "withdraw")["parameters"] = json!([{"name":"unused","ty":ty}]);
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        assert!(
            env.old(env.stateful["withdraw"]).is_some(),
            "{label}: old token planner does not enforce Rust encoding traits"
        );
        let Err(PlanError::Unsupported(gap)) = env.checked(env.stateful["withdraw"]) else {
            panic!("{label}: public boundary must remain closed")
        };
        assert_eq!(gap.detail, "unsupported parameter type");
        assert_eq!(gap.path, "parameters[0]");
        // Previously qualified FAB tuple<=11 / named derive tuple<=8 boundaries.
        // This test does not claim native compilation of these negative sources.
    }
}
#[test]
fn slot_authority_and_witness_arity_are_typed_source_errors() {
    for (mode, expected) in [
        (0, "unknown ledger field or inconsistent index"),
        (
            1,
            "witness result must be supported and arguments must match arity",
        ),
    ] {
        let mut c = source();
        let (ty, value) = if mode == 0 {
            (
                serde_json::to_value(crate::coin_shapes::qualified_coin_type()).unwrap(),
                json!({"kind":"cell_read","field":"pot","index":99}),
            )
        } else {
            (
                json!({"kind":"bytes","length":32}),
                json!({"kind":"witness_call","name":"local_secret_key","arguments":[{"kind":"boolean","value":true}]}),
            )
        };
        prepend(&mut c, "withdraw", vec![binding("bad_source", ty, value)]);
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        assert!(env.old(env.stateful["withdraw"]).is_none());
        let Err(PlanError::Unsupported(gap)) = env.checked(env.stateful["withdraw"]) else {
            panic!("malformed {mode} must be a source refusal")
        };
        assert_eq!(gap.detail, expected, "{gap:?}");
        assert!(gap.path.contains("bindings[0].value"), "{gap:?}");
    }
}
#[test]
fn valid_generic_compositions_are_separate_from_old_shape_admission() {
    for mode in 0..3 {
        let mut c = source();
        match mode {
            0 => {
                circuit(&mut c, "concede")["actions"] =
                    json!([{"kind":"native_witness_call","builtin":"own_public_key"}])
            }
            1 => {
                c["stateful_circuits"].as_array_mut().unwrap().push(json!({"name":"unit_helper","internal":true,"parameters":[],"result":{"kind":"unit"},"actions":[],"return_value":{"kind":"unit"}}));
                let original = root(&mut c, "concede").take();
                *root(&mut c, "concede") = json!({"kind":"sequence","steps":[call("unit_helper",vec![])],"value":original});
            }
            2 => {
                let original = root(&mut c, "concede").take();
                let ty = circuit(&mut c, "concede")["result"].clone();
                *root(&mut c, "concede") = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":original,"otherwise":{"kind":"default","ty":ty}});
            }
            _ => unreachable!(),
        }
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let c: Contract = serde_json::from_value(c).unwrap();
                let env = Environment::new(&c);
                assert!(
                    env.old(env.stateful["concede"]).is_none(),
                    "expansion {mode}"
                );
                render(&env.checked(env.stateful["concede"]).unwrap()).unwrap();
                current(&c, "concede", &format!("shape-expansion-{mode}"));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
#[test]
fn old_actionful_phase_rejects_extra_or_conditional_writes_and_recursive_calls() {
    for mode in 0..4 {
        let mut c = source();
        let helper = circuit(&mut c, "red_concede");
        match mode {
            0 => {
                let mut write = None;
                assert!(first(&mut helper["actions"], &mut |v| {
                    if v["kind"] == "cell_write" {
                        write = Some(v.clone());
                        true
                    } else {
                        false
                    }
                }));
                helper["actions"]
                    .as_array_mut()
                    .unwrap()
                    .push(write.unwrap());
            }
            1 => assert!(first(&mut helper["actions"], &mut |v| {
                if v["kind"] == "cell_write" {
                    let write = v.clone();
                    *v = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":write,"otherwise":write});
                    true
                } else {
                    false
                }
            })),
            2 => helper["return_value"]["value"] = call("concede", vec![]),
            3 => {
                let actions = helper["actions"].take();
                let key = serde_json::to_value(
                    crate::ir::NativeWitnessBuiltin::OwnPublicKey.result_type(),
                )
                .unwrap();
                helper["actions"] = json!([{"kind":"let","bindings":[binding("hidden_key",key,json!({"kind":"native_witness_call","builtin":"own_public_key"}))],"action":{"kind":"sequence","actions":actions}}]);
            }
            _ => unreachable!(),
        }
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        assert!(
            env.old(env.stateful["concede"]).is_none(),
            "old action phase {mode}"
        );
    }
}
