// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn received_bridge_prefix_and_identity_refusals_remain_distinct_from_inclusion() {
    for mode in 0..4 {
        let mut c = source();
        let root = circuit_mut(&mut c, "receive_then_merge");
        match mode {
            0 => root.actions.clear(),
            1 => root.actions.push(root.actions[0].clone()),
            2 => {
                let StateReturn::Expression {
                    value: Expr::Call { arguments, .. },
                } = &mut root.return_value
                else {
                    unreachable!()
                };
                arguments.swap(0, 1);
            }
            _ => {
                // Even an empty Let breaks the old outer received_bridge shape.
                let StateReturn::Expression { value } = &mut root.return_value else {
                    unreachable!()
                };
                *value = Expr::Let {
                    bindings: vec![],
                    body: Box::new(value.clone()),
                };
            }
        }
        let env = Environment::new(&c);
        assert!(
            env.old(env.stateful["receive_then_merge"]).is_none(),
            "old negative {mode}"
        );
        // Some are valid new compositions; never claim all old refusals must
        // remain refused by the generalized compiler.
    }
}
#[test]
fn direct_opaque_default_is_new_composition_while_recursion_remains_refused() {
    for recursive in [false, true] {
        let mut c = source();
        let body = returned_mut(&mut c, "mergeCoin");
        if recursive {
            *body = Expr::Call {
                name: "mergeCoin".into(),
                arguments: vec![parameter("a"), parameter("b")],
            };
        } else {
            *body = Expr::Let {
                bindings: vec![LocalBinding {
                    name: "hidden".into(),
                    ty: Type::OpaqueString,
                    value: Expr::Default {
                        ty: Type::OpaqueString,
                    },
                }],
                body: Box::new(body.clone()),
            };
        }
        let env = Environment::new(&c);
        for name in ["merge_qualified", "receive_then_merge"] {
            assert!(
                env.old(env.stateful[name]).is_none(),
                "{name}/recursive={recursive}"
            );
            assert_eq!(
                env.checked(env.stateful[name]).is_err(),
                recursive,
                "{name}/recursive={recursive}"
            );
        }
    }
}
#[test]
fn renamed_canonical_coin_carrier_is_an_old_negative_not_a_loss() {
    fn visit(v: &mut Json) {
        match v {
            Json::Object(fields) => {
                if fields.get("kind") == Some(&json!("struct"))
                    && fields.get("name") == Some(&json!("ShieldedCoinInfo"))
                {
                    fields.insert("name".into(), json!("RenamedCoin"));
                }
                fields.values_mut().for_each(visit);
            }
            Json::Array(values) => values.iter_mut().for_each(visit),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(source()).unwrap();
    visit(&mut v);
    let c: Contract = serde_json::from_value(v).unwrap();
    let env = Environment::new(&c);
    for name in ["merge_qualified", "receive_then_merge"] {
        assert!(env.old(env.stateful[name]).is_none());
    }
}
