// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn well_typed_wrappers_and_partial_amount_are_expansion_not_old_inclusion() {
    for mode in 0..4 {
        let mut c = source();
        match mode {
            0 | 1 => {
                let name = if mode == 0 {
                    "receive_then_send"
                } else {
                    "sendImmediateShielded"
                };
                let body = returned_mut(&mut c, name);
                let original = std::mem::replace(body, Expr::Unit);
                *body = Expr::Let {
                    bindings: vec![],
                    body: Box::new(original),
                };
            }
            2 => {
                let r = circuit_mut(&mut c, "receive_then_send");
                r.actions = vec![StateAction::Sequence {
                    actions: std::mem::take(&mut r.actions),
                }];
            }
            3 => {
                let Expr::Call { arguments, .. } = returned_mut(&mut c, "receive_then_send") else {
                    unreachable!()
                };
                arguments[2] = Expr::UnsignedLiteral {
                    value: "1".into(),
                    max: u128::MAX.to_string(),
                };
            }
            _ => unreachable!(),
        }
        // Rendering a partial amount is not qualification of a valid funded proof.
        expansion(&c, &format!("separate compositional expansion {mode}"));
    }
}

#[test]
fn malformed_source_names_types_scopes_and_calls_fail_closed() {
    for mode in 0..7 {
        let mut c = source();
        match mode {
            0 => {
                let r = circuit_mut(&mut c, "receive_then_send");
                r.parameters[1].name = r.parameters[0].name.clone();
            }
            1 => {
                let Expr::Call { arguments, .. } = returned_mut(&mut c, "receive_then_send") else {
                    unreachable!()
                };
                arguments[2] = parameter("escaping_amount");
            }
            2 => circuit_mut(&mut c, "receive_then_send").parameters[0].ty = Type::Field,
            3 => {
                let helper = c
                    .circuits
                    .iter_mut()
                    .find(|r| r.name == "coinNullifier")
                    .unwrap();
                helper.body = Expr::Call {
                    name: helper.name.clone(),
                    arguments: helper
                        .parameters
                        .iter()
                        .map(|p| parameter(&p.name))
                        .collect(),
                };
            }
            4 => {
                let helper = circuit_mut(&mut c, "sendImmediateShielded");
                helper.parameters.push(Parameter {
                    name: "missing_argument".into(),
                    ty: Type::Boolean,
                });
            }
            5 => {
                let r = circuit_mut(&mut c, "receive_then_send");
                let Type::Struct { fields, .. } = &mut r.parameters[0].ty else {
                    unreachable!()
                };
                fields[0].ty = Type::Boolean;
            }
            6 => {
                let body = returned_mut(&mut c, "sendShielded");
                let original = std::mem::replace(body, Expr::Unit);
                *body = Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "hidden_state".into(),
                        ty: Type::Field,
                        value: Expr::CellRead {
                            field: "missing_ledger".into(),
                            index: 0,
                        },
                    }],
                    body: Box::new(original),
                };
            }
            _ => unreachable!(),
        }
        let env = Environment::new(&c);
        assert!(
            env.old(root(&c)).is_none(),
            "malformed source {mode}: old refusal"
        );
        assert!(
            matches!(env.checked(root(&c)), Err(PlanError::Unsupported(_))),
            "malformed source {mode}: checked source refusal"
        );
    }
}

#[test]
fn canonical_result_layout_and_projection_indices_are_checked() {
    for mode in 0..2 {
        let mut c = source();
        if mode == 0 {
            let Type::Struct { fields, .. } = &mut circuit_mut(&mut c, "receive_then_send").result
            else {
                unreachable!()
            };
            fields[0].name = "wrong_change_member".into();
        } else {
            let qualifier = c
                .circuits
                .iter_mut()
                .find(|r| r.name == "upcastQualifiedCoin")
                .unwrap();
            let Expr::StructLiteral { fields, .. } = &mut qualifier.body else {
                unreachable!()
            };
            let Expr::StructField { index, .. } = &mut fields[0] else {
                unreachable!()
            };
            *index = 1;
        }
        let env = Environment::new(&c);
        assert!(
            env.old(root(&c)).is_none(),
            "canonical/projection case {mode}"
        );
        assert!(matches!(
            env.checked(root(&c)),
            Err(PlanError::Unsupported(_))
        ));
    }
}
