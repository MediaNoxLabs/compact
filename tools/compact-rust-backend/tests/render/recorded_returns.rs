// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Recorded return plans, effect order, and lexical ownership.

use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, ReturnPlan, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn effectful_recording_audits_both_branches_slots_and_lexical_scope() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../effectful-return-schema20-ir.json")).unwrap();
    let parse = |value| serde_json::from_value::<Contract>(value).unwrap();
    let accepted = render_with_capabilities(&parse(source.clone())).unwrap();
    assert!(accepted.capabilities.circuits[0].recorded);
    assert!(accepted.capabilities.circuits[0].observed_call);
    let branch = "/stateful_circuits/0/return_value/body/result/result";
    let mut wrong_slot = source.clone();
    wrong_slot
        .pointer_mut(&format!("{branch}/then/result/actions/0/index"))
        .map(|v| *v = serde_json::json!(1))
        .unwrap();
    assert!(render(&parse(wrong_slot)).is_err());
    let mut wrong_branch = source.clone();
    wrong_branch
        .pointer_mut(&format!("{branch}/otherwise/result/result/value"))
        .map(|v| *v = serde_json::json!({"kind":"boolean", "value":false}))
        .unwrap();
    assert!(render(&parse(wrong_branch)).is_err());
    let mut escaping = source.clone();
    escaping
        .pointer_mut(&format!("{branch}/otherwise/result/result/value"))
        .map(|v| *v = serde_json::json!({"kind":"parameter", "name":"selected"}))
        .unwrap();
    assert!(render(&parse(escaping)).is_err());
    let mut wrong_witness = source.clone();
    wrong_witness["witnesses"][0]["result"] = serde_json::json!({"kind":"boolean"});
    assert!(render(&parse(wrong_witness)).is_err());
    // Statically unselected branches are checked; Assert is a supported effect.
    let mut unsupported = source.clone();
    unsupported
        .pointer_mut(&format!("{branch}/condition"))
        .map(|v| *v = serde_json::json!({"kind":"boolean", "value":true}))
        .unwrap();
    unsupported.pointer_mut(&format!("{branch}/otherwise/result/actions")).unwrap().as_array_mut().unwrap().push(
        serde_json::json!({"kind":"assert", "condition":{"kind":"boolean", "value":true}, "message":"outside effectful recording profile"}));
    let report = render_with_capabilities(&parse(unsupported)).unwrap();
    // Default algebra admission composes Assert without a whole-body profile.
    assert!(report.capabilities.circuits[0].recorded);
    assert!(report.capabilities.circuits[0].observed_call);
    let mut missing_slot = source.clone();
    missing_slot["ledger_fields"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(render(&parse(missing_slot)).is_err());
    let mut wrong_parameter = source.clone();
    wrong_parameter["stateful_circuits"][0]["parameters"][0]["ty"] =
        serde_json::json!({"kind":"boolean"});
    assert!(render(&parse(wrong_parameter)).is_err());
    let mut wrong_result = source.clone();
    wrong_result["stateful_circuits"][0]["result"] = serde_json::json!({"kind":"boolean"});
    assert!(render(&parse(wrong_result)).is_err());
    let mut wrong_arity = source.clone();
    wrong_arity["witnesses"][0]["parameters"] =
        serde_json::json!([{"name":"extra", "ty":{"kind":"field"}}]);
    assert!(render(&parse(wrong_arity)).is_err());
    let mut action_local_escape = source.clone();
    action_local_escape
        .pointer_mut(&format!("{branch}/otherwise/result/result/value"))
        .map(|v| *v = serde_json::json!({"kind":"parameter", "name":"local_action_only"}))
        .unwrap();
    action_local_escape
        .pointer_mut(&format!(
            "{branch}/otherwise/result/actions/0/bindings/0/name"
        ))
        .map(|v| *v = serde_json::json!("local_action_only"))
        .unwrap();
    action_local_escape
        .pointer_mut(&format!(
            "{branch}/otherwise/result/actions/0/action/value/name"
        ))
        .map(|v| *v = serde_json::json!("local_action_only"))
        .unwrap();
    assert!(render(&parse(action_local_escape)).is_err());
    // Slot declaration changes must not silently retype a Field read.
    let mut wrong_type = source;
    wrong_type["ledger_fields"][0]["declaration"]["ty"] = serde_json::json!({"kind":"boolean"});
    assert!(render(&parse(wrong_type)).is_err());
}

#[test]
fn effectful_return_plan_owns_order_scope_and_typed_branch_results() {
    let before = Expr::Parameter {
        name: "before".into(),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "state".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "choose".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Effectful {
                body: ReturnPlan::Let {
                    bindings: vec![LocalBinding {
                        name: "before".into(),
                        ty: Type::Field,
                        value: Expr::CellRead {
                            field: "state".into(),
                            index: 0,
                        },
                    }],
                    result: Box::new(ReturnPlan::Sequence {
                        actions: vec![StateAction::CellWrite {
                            field: "state".into(),
                            index: 0,
                            value: Expr::FieldLiteral { value: "1".into() },
                        }],
                        result: Box::new(ReturnPlan::Conditional {
                            condition: Expr::Boolean { value: true },
                            then: Box::new(ReturnPlan::Let {
                                bindings: vec![LocalBinding {
                                    name: "before".into(),
                                    ty: Type::Field,
                                    value: Expr::FieldLiteral { value: "7".into() },
                                }],
                                result: Box::new(ReturnPlan::Sequence {
                                    actions: vec![StateAction::CellWrite {
                                        field: "state".into(),
                                        index: 0,
                                        value: Expr::FieldLiteral { value: "2".into() },
                                    }],
                                    result: Box::new(ReturnPlan::Value {
                                        value: before.clone(),
                                    }),
                                }),
                            }),
                            otherwise: Box::new(ReturnPlan::Value { value: before }),
                        }),
                    }),
                },
            },
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(
        rendered
            .source
            .contains("let (context, __compact_effectful_result)")
    );
    assert!(rendered.source.contains("__compact_return_local_0"));
    assert!(rendered.source.contains("__compact_return_local_1"));

    let mut duplicate = contract.clone();
    duplicate.stateful_circuits[0]
        .actions
        .push(StateAction::CellWrite {
            field: "state".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "3".into() },
        });
    assert_eq!(render(&duplicate), Err(RenderError::MalformedReturnPlan));
    let mut malformed = serde_json::to_value(&contract).unwrap();
    malformed["stateful_circuits"][0]["return_value"]["body"]["result"]["result"]
        .as_object_mut()
        .unwrap()
        .remove("otherwise");
    assert!(serde_json::from_value::<Contract>(malformed).is_err());

    fn branch(contract: &mut Contract) -> (&mut Expr, &mut Box<ReturnPlan>, &mut Box<ReturnPlan>) {
        let StateReturn::Effectful { body } = &mut contract.stateful_circuits[0].return_value
        else {
            unreachable!()
        };
        let ReturnPlan::Let { result, .. } = body else {
            unreachable!()
        };
        let ReturnPlan::Sequence { result, .. } = result.as_mut() else {
            unreachable!()
        };
        let ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } = result.as_mut()
        else {
            unreachable!()
        };
        (condition, then, otherwise)
    }
    *branch(&mut contract).0 = Expr::FieldLiteral { value: "1".into() };
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            ..
        })
    ));
    *branch(&mut contract).0 = Expr::Boolean { value: true };
    **branch(&mut contract).2 = ReturnPlan::Value {
        value: Expr::Boolean { value: false },
    };
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            ..
        })
    ));
    {
        let (_, then, otherwise) = branch(&mut contract);
        **then = ReturnPlan::Let {
            bindings: vec![LocalBinding {
                name: "branch_only".into(),
                ty: Type::Field,
                value: Expr::FieldLiteral { value: "7".into() },
            }],
            result: Box::new(ReturnPlan::Value {
                value: Expr::Parameter {
                    name: "branch_only".into(),
                },
            }),
        };
        **otherwise = ReturnPlan::Value {
            value: Expr::Parameter {
                name: "branch_only".into(),
            },
        };
    }
    assert!(
        matches!(render(&contract), Err(RenderError::UnknownParameter(name)) if name == "branch_only")
    );
    contract.schema_version = SCHEMA_VERSION - 1;
    assert!(matches!(
        render(&contract),
        Err(RenderError::SchemaVersion(_))
    ));
}

#[test]
fn discarded_field_binding_keeps_reads_but_rejects_wrong_slot_type_and_nonunit_body() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../unused-field-read-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    let mut pure_only = contract.clone();
    let StateReturn::Expression {
        value: Expr::Let { bindings, .. },
    } = &mut pure_only.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    bindings[0].value = Expr::Parameter { name: "a".into() };
    // A pure Unit composition can record an empty program. Preparation still
    // rejects EmptyTranscript; source-fixture tests qualify that separate gate.
    let pure_rendered = render_with_capabilities(&pure_only).unwrap();
    assert!(pure_rendered.capabilities.circuits[0].recorded);
    assert!(pure_rendered.capabilities.circuits[0].observed_call);

    let mut unsupported = contract.clone();
    let StateReturn::Expression {
        value: Expr::Let { bindings, .. },
    } = &mut unsupported.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    bindings.push(LocalBinding {
        name: "hash".into(),
        ty: Type::Bytes { length: 32 },
        value: Expr::PersistentHash {
            value: Box::new(Expr::Parameter { name: "b".into() }),
        },
    });
    // Aligned persistent hashing composes with discarded reads; malformed slot
    // authority and result types below remain independent rejection controls.
    let hashed = render_with_capabilities(&unsupported).unwrap();
    assert!(hashed.capabilities.circuits[0].recorded);
    assert!(hashed.capabilities.circuits[0].observed_call);
    let mut wrong_index = contract.clone();
    wrong_index.ledger_fields[0].index = 1;
    assert!(render(&wrong_index).is_err());
    let mut wrong_type = contract.clone();
    wrong_type.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean };
    assert!(render(&wrong_type).is_err());
    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut contract.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    **body = Expr::FieldLiteral { value: "7".into() };
    assert!(render(&contract).is_err());
}

#[test]
fn terminal_let_preserves_return_scope_but_siblings_and_branches_do_not_escape() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "stored".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "replace".into(),
            parameters: vec![],
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "previous".into(),
                    ty: Type::Field,
                    value: Expr::CellRead {
                        field: "stored".into(),
                        index: 0,
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "stored".into(),
                    index: 0,
                    value: Expr::FieldLiteral { value: "7".into() },
                }),
            }],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: Expr::Parameter {
                    name: "previous".into(),
                },
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("let result = __compact_action_local_0"));
    assert!(source.contains("crate::ledger_slots::stored"));
    assert!(source.contains(".write(context"));

    // Assertions before the final top-level Let do not shorten the lifetime
    // of its typed binding. This is the complete bboard take_down shape.
    contract.stateful_circuits[0].actions.insert(
        0,
        StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "before binding".into(),
        },
    );
    let source = render(&contract).unwrap();
    assert!(source.contains("let result = __compact_action_local_0"));

    // An earlier sibling Let has ended before the return; neither it nor a
    // nested Let may leak merely because it has the same binding name.
    contract.stateful_circuits[0]
        .actions
        .push(StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "after binding".into(),
        });
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("previous".into()))
    );
    contract.stateful_circuits[0].actions.pop();

    if let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[1] {
        bindings[0].ty = Type::Boolean;
    }
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    if let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[1] {
        bindings[0].ty = Type::Field;
    }

    let root_let = contract.stateful_circuits[0].actions.remove(1);
    contract.stateful_circuits[0].actions = vec![StateAction::Sequence {
        actions: vec![root_let.clone()],
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains("let result = __compact_action_local_0"));

    // A branch is not a return continuation, even when it is the final action.
    contract.stateful_circuits[0].actions = vec![StateAction::If {
        condition: Expr::Boolean { value: true },
        then: Box::new(root_let.clone()),
        otherwise: Box::new(root_let),
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("previous".into()))
    );

    // A nested binding may shadow the root name for its own action, but the
    // final return still denotes the outer value whose scope reaches it.
    contract.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "previous".into(),
            ty: Type::Field,
            value: Expr::CellRead {
                field: "stored".into(),
                index: 0,
            },
        }],
        action: Box::new(StateAction::Sequence {
            actions: vec![
                StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "previous".into(),
                        ty: Type::Boolean,
                        value: Expr::Boolean { value: true },
                    }],
                    action: Box::new(StateAction::Expression {
                        value: Expr::Parameter {
                            name: "previous".into(),
                        },
                    }),
                },
                StateAction::CellWrite {
                    field: "stored".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "previous".into(),
                    },
                },
            ],
        }),
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains("let __compact_action_local_1: bool = true"));
    assert!(source.contains("let result = __compact_action_local_0"));
    // Terminal nested scopes, unlike the earlier sibling above, own the
    // extracted return. Keep every binding and use the nearest shadow.
    let value = |name: &str| Expr::Parameter { name: name.into() };
    let binding = |name: &str, expression: Expr, action: StateAction| StateAction::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty: Type::Field,
            value: expression,
        }],
        action: Box::new(StateAction::Sequence {
            actions: vec![action],
        }),
    };
    let write = StateAction::CellWrite {
        field: "stored".into(),
        index: 0,
        value: value("previous"),
    };
    contract.stateful_circuits[0].actions = vec![binding(
        "previous",
        Expr::CellRead {
            field: "stored".into(),
            index: 0,
        },
        binding(
            "next",
            Expr::Add {
                left: Box::new(value("previous")),
                right: Box::new(Expr::FieldLiteral { value: "1".into() }),
            },
            binding("previous", value("next"), write),
        ),
    )];
    let source = render(&contract).unwrap();
    assert!(source.contains("let result = __compact_action_local_2"));
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: value("next"),
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("let result = __compact_action_local_1"));
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: value("missing"),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn terminal_lexical_source_records_extracted_returns_but_preserves_explicit_scopes() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../terminal-lexical-return-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let output = render_with_capabilities(&contract).unwrap();
    assert_eq!(output.capabilities.circuits.len(), 5);
    for capability in &output.capabilities.circuits {
        assert!(capability.recorded, "{}", capability.name);
        assert!(capability.observed_call, "{}", capability.name);
    }
    for name in ["two", "three", "echo", "observed", "nested"] {
        assert!(output.source.contains(&format!("pub fn {name}<")));
    }
    // Action-owned bindings inside an independent ReturnPlan::Sequence
    // cannot escape into its separate result continuation.
    let circuit = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "two")
        .unwrap();
    let actions = std::mem::take(&mut circuit.actions);
    let StateReturn::Expression { value } = circuit.return_value.clone() else {
        panic!()
    };
    circuit.return_value = StateReturn::Effectful {
        body: ReturnPlan::Sequence {
            actions,
            result: Box::new(ReturnPlan::Value { value }),
        },
    };
    let error = render(&contract).unwrap_err();
    assert!(format!("{error:?}").contains("UnknownParameter(\"after\")"));
}
