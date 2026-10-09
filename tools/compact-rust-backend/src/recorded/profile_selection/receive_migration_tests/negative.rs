// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn refused(c: &Contract, label: &str) {
    let maps = Context::new(c);
    assert!(maps.old(root(c)).is_none(), "old malformed {label}");
    assert!(
        matches!(
            maps.algebra(root(c)),
            Err(effect_plan::PlanError::Unsupported(_))
        ),
        "checked malformed {label}"
    );
    assert!(
        maps.domain(root(c)).unwrap().is_none(),
        "domain malformed {label}"
    );
}
#[test]
fn declarations_graphs_and_lexical_scopes_are_not_shape_permissions() {
    for mode in 0..9 {
        let mut c = source();
        match mode {
            0 => {
                let StateAction::CircuitCall { name, .. } = &mut root_mut(&mut c).actions[0] else {
                    unreachable!()
                };
                *name = "unknown".into();
            }
            1 => {
                let StateAction::CircuitCall { name, .. } = &mut root_mut(&mut c).actions[0] else {
                    unreachable!()
                };
                *name = "coinCommitment".into();
            }
            2 => {
                let StateAction::CircuitCall { name, .. } = &mut root_mut(&mut c).actions[0] else {
                    unreachable!()
                };
                *name = "accept".into();
            }
            3 => {
                let StateAction::CircuitCall { arguments, .. } = &mut root_mut(&mut c).actions[0]
                else {
                    unreachable!()
                };
                arguments.clear();
            }
            4 => {
                let StateAction::CircuitCall { arguments, .. } = &mut root_mut(&mut c).actions[0]
                else {
                    unreachable!()
                };
                arguments[0] = field(1);
            }
            5 => prepend(&mut c, Type::Field, parameter("unknown")),
            6 => prepend(
                &mut c,
                Type::Field,
                local("escaped", Type::Field, field(1), parameter("wrong")),
            ),
            7 => {
                let p = c
                    .circuits
                    .iter_mut()
                    .find(|p| p.name == "coinCommitment")
                    .unwrap();
                p.body = Expr::Call {
                    name: p.name.clone(),
                    arguments: p.parameters.iter().map(|p| parameter(&p.name)).collect(),
                };
            }
            8 => {
                let p = c
                    .circuits
                    .iter_mut()
                    .find(|p| p.name == "coinCommitment")
                    .unwrap();
                p.body = Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "hidden".into(),
                        ty: crate::ir::contract_address_type(),
                        value: Expr::KernelSelf {
                            ty: crate::ir::contract_address_type(),
                        },
                    }],
                    body: Box::new(p.body.clone()),
                };
            }
            _ => unreachable!(),
        }
        refused(&c, &format!("graph/scope {mode}"));
    }
}
#[test]
fn member_index_width_and_unselected_children_are_independently_checked() {
    for mode in 0..5 {
        let mut c = source();
        let expression = match mode {
            0 => Expr::StructField {
                value: Box::new(parameter("coin")),
                field: "nonce".into(),
                index: 1,
            },
            1 => Expr::StructField {
                value: Box::new(parameter("coin")),
                field: "absent".into(),
                index: 0,
            },
            2 => Expr::Coerce {
                value: Box::new(number(1, "65535")),
                ty: Type::Unsigned { max: "255".into() },
            },
            3 => Expr::Equal {
                left: Box::new(number(1, "255")),
                right: Box::new(number(1, "65535")),
            },
            4 => Expr::If {
                condition: Box::new(Expr::Boolean { value: true }),
                then: Box::new(Expr::Boolean { value: true }),
                otherwise: Box::new(parameter("unknown")),
            },
            _ => unreachable!(),
        };
        let ty = match mode {
            0 | 1 => Type::Bytes { length: 32 },
            2 => Type::Unsigned { max: "255".into() },
            _ => Type::Boolean,
        };
        prepend(&mut c, ty, expression);
        refused(&c, &format!("operand {mode}"));
    }
}
