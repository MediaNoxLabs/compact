// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn effects() -> Vec<Expr> {
    let mut result = vec![];
    for kind in [
        KernelClaimKind::Nullifier,
        KernelClaimKind::CoinSpend,
        KernelClaimKind::CoinReceive,
    ] {
        for value in [parameter("domain"), witness("get_domain")] {
            result.push(claim(kind, value));
        }
    }
    for domain in [parameter("domain"), witness("get_domain")] {
        for amount in [parameter("amount"), witness("get_amount")] {
            result.push(mint(domain.clone(), amount));
        }
    }
    assert_eq!(result.len(), 10);
    result
}
fn wrap(effect: Expr, shape: usize) -> StateAction {
    match shape {
        0 => action(effect),
        1 => StateAction::Sequence {
            actions: vec![empty(), action(effect.clone()), action(effect)],
        },
        2 => StateAction::Let {
            bindings: vec![
                LocalBinding {
                    name: "domain".into(),
                    ty: bytes_type(),
                    value: witness("get_domain"),
                },
                LocalBinding {
                    name: "domain".into(),
                    ty: bytes_type(),
                    value: parameter("domain"),
                },
                LocalBinding {
                    name: "amount".into(),
                    ty: amount_type(),
                    value: witness("get_amount"),
                },
                LocalBinding {
                    name: "selected".into(),
                    ty: Type::Boolean,
                    value: witness("get_selected"),
                },
            ],
            action: Box::new(StateAction::Let {
                bindings: vec![],
                action: Box::new(action(effect)),
            }),
        },
        3 | 4 => StateAction::If {
            condition: if shape == 3 {
                parameter("selected")
            } else {
                witness("get_selected")
            },
            then: Box::new(action(effect)),
            otherwise: Box::new(empty()),
        },
        5 => StateAction::If {
            condition: parameter("selected"),
            then: Box::new(empty()),
            otherwise: Box::new(StateAction::If {
                condition: witness("get_selected"),
                then: Box::new(action(effect)),
                otherwise: Box::new(empty()),
            }),
        },
        _ => unreachable!(),
    }
}
#[test]
fn every_old_effect_operand_and_action_rule_is_admitted_first() {
    let mut count = 0;
    for (i, effect) in effects().into_iter().enumerate() {
        for shape in 0..6 {
            let mut contract = subject();
            contract.stateful_circuits[0].actions = vec![wrap(effect.clone(), shape)];
            included(&contract, "root", &format!("effect{i}/action{shape}"));
            count += 1;
        }
        let mut contract = subject();
        contract.stateful_circuits[0].return_value = StateReturn::Expression { value: effect };
        included(&contract, "root", &format!("effect{i}/expression-return"));
        count += 1;
    }
    assert_eq!(count, 70);
}
#[test]
fn mixed_effects_and_sibling_scope_restore_are_old_controls() {
    let mut contract = subject();
    contract.stateful_circuits[0].actions = vec![
        wrap(claim(KernelClaimKind::Nullifier, parameter("domain")), 2),
        action(mint(parameter("domain"), parameter("amount"))),
        action(claim(KernelClaimKind::CoinSpend, parameter("domain"))),
    ];
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: claim(KernelClaimKind::CoinReceive, parameter("domain")),
    };
    included(
        &contract,
        "root",
        "mixed effects with nonterminal local scope",
    );
}
#[test]
fn all_seven_retained_kernel_exports_are_explicit_old_controls() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/kernel-shielded-effects-schema20-ir.json"
    ))
    .unwrap();
    let actual: Vec<_> = contract
        .stateful_circuits
        .iter()
        .filter(|c| !c.internal)
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(
        actual,
        [
            "mint",
            "nullifier",
            "spend",
            "claim_receive",
            "batch",
            "selected",
            "witness_order",
            "read_state"
        ]
    );
    for name in [
        "mint",
        "nullifier",
        "spend",
        "claim_receive",
        "batch",
        "selected",
        "witness_order",
    ] {
        included(&contract, name, name);
    }
}
#[test]
fn malformed_operand_witness_scope_and_duplicate_parameter_are_refused() {
    for mode in 0..11 {
        let mut contract = subject();
        let mut value = claim(KernelClaimKind::Nullifier, parameter("domain"));
        match mode {
            0 => value = claim(KernelClaimKind::Nullifier, parameter("amount")),
            1 => value = mint(parameter("domain"), parameter("selected")),
            2 => value = mint(parameter("amount"), parameter("amount")),
            3 => value = claim(KernelClaimKind::Nullifier, witness("missing")),
            4 => {
                contract.witnesses[1].parameters.push(Parameter {
                    name: "input".into(),
                    ty: bytes_type(),
                });
                value = claim(KernelClaimKind::Nullifier, witness("get_domain"));
            }
            5 => {
                value = claim(
                    KernelClaimKind::Nullifier,
                    Expr::WitnessCall {
                        name: "get_domain".into(),
                        arguments: vec![parameter("domain")],
                    },
                )
            }
            6 => value = claim(KernelClaimKind::Nullifier, parameter("escaped")),
            7 => contract.stateful_circuits[0].parameters.push(Parameter {
                name: "domain".into(),
                ty: bytes_type(),
            }),
            8 => {
                contract.stateful_circuits[0].parameters[2].ty =
                    Type::Unsigned { max: "255".into() };
                value = mint(parameter("domain"), parameter("amount"));
            }
            9 | 10 => {
                let scoped = StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "escaped".into(),
                        ty: bytes_type(),
                        value: parameter("domain"),
                    }],
                    action: Box::new(empty()),
                };
                contract.stateful_circuits[0].actions = vec![
                    if mode == 9 {
                        scoped
                    } else {
                        StateAction::If {
                            condition: parameter("selected"),
                            then: Box::new(scoped),
                            otherwise: Box::new(empty()),
                        }
                    },
                    empty(),
                ];
                value = claim(KernelClaimKind::Nullifier, parameter("escaped"));
            }
            _ => unreachable!(),
        }
        contract.stateful_circuits[0].return_value = StateReturn::Expression { value };
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none(), "old negative {mode}");
        assert!(
            matches!(
                context.algebra(root),
                Err(effect_plan::PlanError::Unsupported(_))
            ),
            "new negative {mode}"
        );
    }
}
