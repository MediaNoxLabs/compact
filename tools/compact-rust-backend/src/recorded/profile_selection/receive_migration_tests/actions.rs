// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn old_sequences_scopes_and_nested_stateful_calls_remain_included() {
    for mode in 0..4 {
        let mut c = source();
        match mode {
            0 => {
                let original = helper_mut(&mut c).actions.clone();
                helper_mut(&mut c).actions = vec![StateAction::Sequence {
                    actions: vec![
                        StateAction::Sequence { actions: vec![] },
                        StateAction::Sequence { actions: original },
                        StateAction::Sequence { actions: vec![] },
                    ],
                }];
            }
            1 => {
                let original = helper_mut(&mut c).actions.clone();
                helper_mut(&mut c).actions = vec![StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "shadow".into(),
                        ty: Type::Field,
                        value: field(1),
                    }],
                    action: Box::new(StateAction::Sequence {
                        actions: vec![
                            StateAction::Let {
                                bindings: vec![LocalBinding {
                                    name: "shadow".into(),
                                    ty: Type::Boolean,
                                    value: Expr::Boolean { value: true },
                                }],
                                action: Box::new(StateAction::Sequence { actions: vec![] }),
                            },
                            StateAction::Let {
                                bindings: vec![LocalBinding {
                                    name: "restored".into(),
                                    ty: Type::Field,
                                    value: parameter("shadow"),
                                }],
                                action: Box::new(StateAction::Sequence { actions: original }),
                            },
                        ],
                    }),
                }];
            }
            2 => {
                let mut bridge = root(&c).clone();
                bridge.name = "receiveBridge".into();
                bridge.internal = true;
                c.stateful_circuits.push(bridge);
                let StateAction::CircuitCall { name, arguments } = &mut root_mut(&mut c).actions[0]
                else {
                    unreachable!()
                };
                *name = "receiveBridge".into();
                arguments[0] = local("unused", Type::Field, field(4), arguments[0].clone());
            }
            3 => {
                let mut empty = helper_mut(&mut c).clone();
                empty.name = "noEffects".into();
                empty.actions = vec![StateAction::Sequence { actions: vec![] }];
                c.stateful_circuits.push(empty);
                for _ in 0..2 {
                    helper_mut(&mut c).actions.push(StateAction::CircuitCall {
                        name: "noEffects".into(),
                        arguments: vec![parameter("coin")],
                    });
                }
            }
            _ => unreachable!(),
        }
        included(&c, "accept", &format!("action composition {mode}"));
    }
}
#[test]
fn declaration_renaming_does_not_control_admission() {
    let mut c = source();
    helper_mut(&mut c).name = "renamedReceive".into();
    for root in c.stateful_circuits.iter_mut().filter(|c| !c.internal) {
        let StateAction::CircuitCall { name, .. } = &mut root.actions[0] else {
            unreachable!()
        };
        *name = "renamedReceive".into();
    }
    included(&c, "accept", "renamed stateful declaration");
}
