// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn unchanged_whole_source_includes_both_complete_old_set_alternatives() {
    let c = fixture();
    for name in ["addName", "hasName"] {
        included(&c, name);
    }
}
#[test]
fn old_forms_follow_declared_names_layout_and_reachable_internal_helpers() {
    for mode in ["raw_names", "nonzero", "chunked", "internal_insert"] {
        let mut c = fixture();
        match mode {
            "raw_names" => {
                c.ledger_fields[0].id = "r#type".into();
                for r in &mut c.stateful_circuits {
                    r.parameters[0].name = "r#match".into();
                    for a in &mut r.actions {
                        if let StateAction::SetInsert { field, value, .. } = a {
                            *field = "r#type".into();
                            *value = parameter("r#match");
                        }
                    }
                    if let StateReturn::SetMember { field, value, .. } = &mut r.return_value {
                        *field = "r#type".into();
                        *value = parameter("r#match");
                    }
                }
            }
            "nonzero" => {
                c.ledger_fields[0].index = 1;
                c.ledger_fields[0].path = vec![1];
                c.ledger_fields.insert(
                    0,
                    LedgerField {
                        id: "padding".into(),
                        index: 0,
                        path: vec![0],
                        source: None,
                        declaration: LedgerFieldKind::Counter,
                    },
                );
                for r in &mut c.stateful_circuits {
                    for a in &mut r.actions {
                        if let StateAction::SetInsert { index, .. } = a {
                            *index = 1;
                        }
                    }
                    if let StateReturn::SetMember { index, .. } = &mut r.return_value {
                        *index = 1;
                    }
                }
            }
            "chunked" => {
                c.ledger_fields[0].path = vec![0, 1];
                c.ledger_fields.insert(
                    0,
                    LedgerField {
                        id: "padding0".into(),
                        index: 0,
                        path: vec![0, 0],
                        source: None,
                        declaration: LedgerFieldKind::Counter,
                    },
                );
                for i in 0..15 {
                    c.ledger_fields.push(LedgerField {
                        id: format!("padding{}", i + 1),
                        index: 1,
                        path: vec![1, i],
                        source: None,
                        declaration: LedgerFieldKind::Counter,
                    });
                }
            }
            "internal_insert" => {
                root(&mut c, "addName").internal = true;
                let mut wrapper = root(&mut c, "addName").clone();
                wrapper.name = "insertViaHelper".into();
                wrapper.internal = false;
                wrapper.actions = vec![StateAction::CircuitCall {
                    name: "addName".into(),
                    arguments: vec![parameter("name")],
                }];
                c.stateful_circuits.push(wrapper);
            }
            _ => unreachable!(),
        }
        if mode == "internal_insert" {
            let cx = Context::new(&c);
            assert!(cx.old(cx.circuits["addName"]).unwrap());
            assert!(cx.domain(cx.circuits["addName"]).unwrap().is_some());
            assert!(current(&c, "insertViaHelper").contains("record_insert"));
            // Internal insertion is reached through the public wrapper; it is
            // not misrepresented as an independently exported recorded API.
        } else {
            included(&c, "addName");
        }
        included(&c, "hasName");
    }
}
#[test]
fn boolean_interface_composes_helpers_lazy_queries_and_non_set_direct_returns() {
    for mode in ["expression", "helper_lazy", "direct_empty"] {
        let mut c = fixture();
        let r = root(&mut c, "hasName");
        match mode {
            "expression" => r.return_value = StateReturn::Expression { value: query() },
            "helper_lazy" => {
                r.actions = vec![StateAction::CircuitCall {
                    name: "addName".into(),
                    arguments: vec![parameter("name")],
                }];
                r.return_value = StateReturn::Expression {
                    value: Expr::If {
                        condition: Box::new(Expr::Boolean { value: true }),
                        then: Box::new(query()),
                        otherwise: Box::new(Expr::Boolean { value: false }),
                    },
                };
            }
            "direct_empty" => {
                r.return_value = StateReturn::SetIsEmpty {
                    field: "names".into(),
                    index: 0,
                }
            }
            _ => unreachable!(),
        }
        let cx = Context::new(&c);
        assert!(!cx.old(cx.circuits["hasName"]).unwrap(), "expansion {mode}");
        let text = current(&c, "hasName");
        if mode == "helper_lazy" {
            assert!(text.find("record_insert").unwrap() < text.find("record_member").unwrap());
            assert!(text.contains("if"));
        }
    }
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-maps-schema20-ir.json"
    ))
    .unwrap();
    assert!(current(&c, "direct_member").contains("record_member"));
    let cx = Context::new(&c);
    assert!(cx.domain(cx.circuits["direct_lookup"]).unwrap().is_none());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == "direct_lookup" && !r.recorded && !r.observed_call)
    );
}
