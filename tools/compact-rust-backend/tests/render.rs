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

use std::io::Write;
use std::process::{Command, Stdio};

use compact_rust_backend::ir::{
    Constructor, ConstructorStep, Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind,
    LocalBinding, NativeWitnessBuiltin, Parameter, PureCircuit, ReturnPlan, SCHEMA_VERSION,
    SourceLocation, StateAction, StateReturn, StatefulCircuit, StructField, Type, TypeAlias,
    WitnessDeclaration,
};
use compact_rust_backend::{
    RenderError, render, render_with_capabilities, render_with_proof_capabilities,
};

#[test]
fn effectful_recording_audits_both_branches_slots_and_lexical_scope() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("effectful-return-schema20-ir.json")).unwrap();
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
    // Even a statically unselected branch must fit the audited action domain.
    let mut unsupported = source.clone();
    unsupported
        .pointer_mut(&format!("{branch}/condition"))
        .map(|v| *v = serde_json::json!({"kind":"boolean", "value":true}))
        .unwrap();
    unsupported.pointer_mut(&format!("{branch}/otherwise/result/actions")).unwrap().as_array_mut().unwrap().push(
        serde_json::json!({"kind":"assert", "condition":{"kind":"boolean", "value":true}, "message":"outside effectful recording profile"}));
    let report = render_with_capabilities(&parse(unsupported)).unwrap();
    assert!(!report.capabilities.circuits[0].recorded);
    assert!(!report.capabilities.circuits[0].observed_call);
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
        serde_json::from_str(include_str!("unused-field-read-ir.json")).unwrap();
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
    assert!(
        !render_with_capabilities(&pure_only)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

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
    assert!(
        !render_with_capabilities(&unsupported)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
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
fn direct_plain_merkle_root_recording_requires_exact_digest_and_slot() {
    let digest = Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            depth: 3,
            ty: Type::Boolean,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        name: "known".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "root".into(),
            ty: digest.clone(),
        }],
        actions: vec![],
        result: Type::Boolean,
        return_value: StateReturn::MerkleCheckRoot {
            field: "tree".into(),
            index: 0,
            root: Expr::Parameter {
                name: "root".into(),
            },
        },
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);

    let mut computed = contract.clone();
    let StateReturn::MerkleCheckRoot { root, .. } = &mut computed.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    *root = Expr::Default { ty: digest.clone() };
    assert!(
        !render_with_capabilities(&computed)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut effectful = contract.clone();
    effectful.stateful_circuits[0]
        .actions
        .push(StateAction::MerkleInsert {
            field: "tree".into(),
            index: 0,
            value: Expr::Boolean { value: true },
        });
    assert!(
        !render_with_capabilities(&effectful)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut wrong_slot = contract.clone();
    let StateReturn::MerkleCheckRoot { index, .. } =
        &mut wrong_slot.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    *index = 1;
    assert!(render(&wrong_slot).is_err());

    let mut historic = contract.clone();
    historic.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        depth: 3,
        ty: Type::Boolean,
    };
    historic.stateful_circuits[0].return_value = StateReturn::HistoricMerkleCheckRoot {
        field: "tree".into(),
        index: 0,
        root: Expr::Parameter {
            name: "root".into(),
        },
    };
    let historic_rendered = render_with_capabilities(&historic).unwrap();
    assert!(historic_rendered.capabilities.circuits[0].recorded);
    assert!(historic_rendered.capabilities.circuits[0].observed_call);
    assert!(historic_rendered.source.contains("record_check_root("));

    let mut wrong_historic_slot = historic.clone();
    wrong_historic_slot.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        depth: 3,
        ty: Type::Boolean,
    };
    assert!(render_with_capabilities(&wrong_historic_slot).is_err());
    let mut wrong_historic_index = historic.clone();
    if let StateReturn::HistoricMerkleCheckRoot { index, .. } =
        &mut wrong_historic_index.stateful_circuits[0].return_value
    {
        *index = 1;
    }
    assert!(render_with_capabilities(&wrong_historic_index).is_err());
    let mut nested_historic = historic.clone();
    nested_historic.ledger_fields[0].path = vec![1];
    assert!(
        !render_with_capabilities(&nested_historic)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    let mut wrong_historic_digest = historic.clone();
    wrong_historic_digest.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert!(
        !render_with_capabilities(&wrong_historic_digest)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    let mut computed_historic = historic.clone();
    if let StateReturn::HistoricMerkleCheckRoot { root, .. } =
        &mut computed_historic.stateful_circuits[0].return_value
    {
        *root = Expr::Default { ty: digest.clone() };
    }
    assert!(
        !render_with_capabilities(&computed_historic)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    let mut effectful_historic = historic.clone();
    effectful_historic.stateful_circuits[0]
        .actions
        .push(StateAction::HistoricMerkleInsert {
            field: "tree".into(),
            index: 0,
            value: Expr::Boolean { value: true },
        });
    assert!(
        !render_with_capabilities(&effectful_historic)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    let rejected = render_with_capabilities(&contract);
    assert!(rejected.is_err() || !rejected.unwrap().capabilities.circuits[0].recorded);
}

#[test]
fn recorded_mixed_width_guard_requires_exact_public_widths_and_pure_operands() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "count".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Counter,
            },
            LedgerField {
                source: None,
                id: "stored".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Cell {
                    ty: Type::Unsigned { max: "255".into() },
                },
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "matching".into(),
            parameters: vec![
                Parameter {
                    name: "small".into(),
                    ty: Type::Unsigned { max: "255".into() },
                },
                Parameter {
                    name: "big".into(),
                    ty: Type::Unsigned {
                        max: "4294967295".into(),
                    },
                },
            ],
            actions: vec![
                StateAction::Assert {
                    condition: Expr::Equal {
                        left: Box::new(Expr::UnsignedCast {
                            max: "4294967295".into(),
                            value: Box::new(Expr::Parameter {
                                name: "small".into(),
                            }),
                        }),
                        right: Box::new(Expr::Parameter { name: "big".into() }),
                    },
                    message: "different".into(),
                },
                StateAction::CounterIncrement {
                    field: "count".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let recorded = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    };
    assert!(recorded(&contract));
    assert!(
        render_with_capabilities(&contract)
            .unwrap()
            .source
            .contains("__compact_recorded_widened_0")
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned {
        max: "65535".into(),
    };
    assert!(!recorded(&contract));
    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned { max: "255".into() };
    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = left.as_mut() else {
        unreachable!()
    };
    **value = Expr::CellRead {
        field: "stored".into(),
        index: 1,
    };
    assert!(!recorded(&contract));
}

#[test]
fn recorded_product_guard_requires_closed_pure_body_and_counter_one() {
    let uint32 = Type::Unsigned {
        max: "4294967295".into(),
    };
    let product = Type::Unsigned {
        max: "17179869180".into(),
    };
    let param = |name: &str| Expr::Parameter { name: name.into() };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "count".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "guard".into(),
            parameters: vec![
                Parameter {
                    name: "q".into(),
                    ty: uint32.clone(),
                },
                Parameter {
                    name: "y".into(),
                    ty: uint32.clone(),
                },
            ],
            result: Type::Unit,
            body: Expr::Sequence {
                steps: vec![Expr::Assert {
                    condition: Box::new(Expr::Let {
                        bindings: vec![LocalBinding {
                            name: "product".into(),
                            ty: product.clone(),
                            value: Expr::UnsignedMultiply {
                                max: "17179869180".into(),
                                left: Box::new(Expr::UnsignedCast {
                                    max: "17179869180".into(),
                                    value: Box::new(param("q")),
                                }),
                                right: Box::new(Expr::UnsignedLiteral {
                                    value: "4".into(),
                                    max: "17179869180".into(),
                                }),
                            },
                        }],
                        body: Box::new(Expr::Compare {
                            operator: compact_rust_backend::ir::ComparisonOperator::LessEqual,
                            left: Box::new(param("product")),
                            right: Box::new(Expr::UnsignedCast {
                                max: "17179869180".into(),
                                value: Box::new(param("y")),
                            }),
                        }),
                    }),
                    message: "product exceeds bound".into(),
                }],
                value: Box::new(Expr::Unit),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "record".into(),
            parameters: vec![
                Parameter {
                    name: "q".into(),
                    ty: uint32.clone(),
                },
                Parameter {
                    name: "y".into(),
                    ty: uint32.clone(),
                },
            ],
            actions: vec![
                StateAction::PureCall {
                    name: "guard".into(),
                    arguments: vec![
                        Expr::Coerce {
                            value: Box::new(param("q")),
                            ty: uint32.clone(),
                        },
                        Expr::Coerce {
                            value: Box::new(param("y")),
                            ty: uint32.clone(),
                        },
                    ],
                },
                StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "one".into(),
                        ty: Type::Unsigned {
                            max: "65535".into(),
                        },
                        value: Expr::UnsignedLiteral {
                            value: "1".into(),
                            max: "65535".into(),
                        },
                    }],
                    action: Box::new(StateAction::CounterIncrement {
                        field: "count".into(),
                        index: 0,
                        amount: CounterAmount::Parameter { name: "one".into() },
                    }),
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let recorded = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    };
    assert!(recorded(&contract));
    let pristine = contract.clone();

    if let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body {
        steps.push(Expr::Unit);
    }
    assert!(!recorded(&contract));
    contract = pristine.clone();
    if let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body
        && let Expr::Assert { condition, .. } = &mut steps[0]
        && let Expr::Let { bindings, .. } = condition.as_mut()
        && let Expr::UnsignedMultiply { right, .. } = &mut bindings[0].value
    {
        **right = Expr::UnsignedLiteral {
            value: "5".into(),
            max: "17179869180".into(),
        };
    }
    assert!(!recorded(&contract));
    contract = pristine.clone();
    if let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[1] {
        bindings[0].value = Expr::UnsignedLiteral {
            value: "2".into(),
            max: "65535".into(),
        };
    }
    assert!(!recorded(&contract));
    contract = pristine;
    if let StateAction::PureCall { arguments, .. } = &mut contract.stateful_circuits[0].actions[0] {
        arguments.swap(0, 1);
    }
    assert!(!recorded(&contract));
}

#[test]
fn recorded_pair_hash_bindings_require_closed_typed_field_literals() {
    let pair = || Expr::Tuple {
        elements: vec![
            Expr::FieldLiteral { value: "0".into() },
            Expr::FieldLiteral { value: "1".into() },
        ],
    };
    let binding = |name: &str, ty: Type, value: Expr, index: u8| StateAction::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        action: Box::new(StateAction::CellWrite {
            field: if index == 0 { "digest" } else { "field" }.into(),
            index,
            value: Expr::Parameter { name: name.into() },
        }),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                id: "digest".into(),
                index: 0,
                path: vec![],
                source: None,
                declaration: LedgerFieldKind::Cell {
                    ty: Type::Bytes { length: 32 },
                },
            },
            LedgerField {
                id: "field".into(),
                index: 1,
                path: vec![],
                source: None,
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: false,
                name: "persistent".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![binding(
                    "hash",
                    Type::Bytes { length: 32 },
                    Expr::PersistentHash {
                        value: Box::new(pair()),
                    },
                    0,
                )],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "transient".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![binding(
                    "hash",
                    Type::Field,
                    Expr::TransientHash {
                        value: Box::new(pair()),
                    },
                    1,
                )],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits.iter().all(|c| c.recorded));
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let persistent = recorded.split("pub fn persistent<Private>").nth(1).unwrap();
    let transient = recorded.split("pub fn transient<Private>").nth(1).unwrap();
    assert!(
        persistent.find("runtime::persistent_hash(").unwrap()
            < persistent.find(".record_write(").unwrap()
    );
    assert!(
        transient.find("runtime::transient_hash(").unwrap()
            < transient.find(".record_write(").unwrap()
    );

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::PersistentHash {
        value: Box::new(Expr::Tuple {
            elements: vec![
                Expr::CellRead {
                    field: "field".into(),
                    index: 1,
                },
                Expr::FieldLiteral { value: "1".into() },
            ],
        }),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[1].recorded);

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[1].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::TransientHash {
        value: Box::new(Expr::Tuple {
            elements: vec![Expr::FieldLiteral { value: "0".into() }],
        }),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits.iter().all(|c| !c.recorded));
}

#[test]
fn opaque_string_map_recording_requires_closed_field_lookup_sequence() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("opaque-string-map-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let statuses = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .map(|capability| (capability.name, capability.recorded))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), true), ("ensure".into(), true)]
    );

    let original = contract.stateful_circuits[0].actions[0].clone();
    contract.stateful_circuits[0].actions.push(original.clone());
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), false), ("ensure".into(), true)]
    );
    contract.stateful_circuits[0].actions.truncate(1);
    let StateAction::MapInsert { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::FieldLiteral { value: "7".into() };
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), false), ("ensure".into(), true)]
    );
    contract.stateful_circuits[0].actions[0] = original;

    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut contract.stateful_circuits[1].return_value
    else {
        unreachable!()
    };
    **body = Expr::FieldLiteral { value: "7".into() };
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), true), ("ensure".into(), false)]
    );
}

#[test]
fn asset_removal_recording_requires_scoped_opaque_key_and_exact_order() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("asset-removal-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "removeRecord")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn removeRecord<Private"));
    assert!(source.contains("record_remove(frame"));

    let mut swapped = contract.clone();
    let circuit = swapped
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "removeRecord")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    actions.swap(3, 4);
    assert!(!recorded(&swapped).0, "mutation order is part of the gate");

    let circuit = contract
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "removeRecord")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::MapRemove { key, .. } = &mut actions[3] else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "recordId".into(),
    };
    assert!(!recorded(&contract).0, "Map removal must use the bound key");
}

#[test]
fn custody_grant_recording_requires_typed_lookup_and_closed_pure_assertion() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("asset-grant-effective-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "assertGrantEffective")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn assertGrantEffective<Private"));
    assert!(source.contains("record_lookup(frame"));

    let mut wrong_key = contract.clone();
    let StateAction::Let { action, .. } = &mut wrong_key.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[1] else {
        unreachable!()
    };
    let Expr::MapLookup { key, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **key = Expr::Parameter {
        name: "grantId".into(),
    };
    assert!(!recorded(&wrong_key).0, "lookup must use the scoped key");

    let mut changed_guard = contract;
    let Expr::Sequence { steps, .. } = &mut changed_guard.circuits[0].body else {
        unreachable!()
    };
    steps.insert(
        0,
        Expr::Assert {
            condition: Box::new(Expr::NotEqual {
                left: Box::new(Expr::FieldLiteral { value: "1".into() }),
                right: Box::new(Expr::FieldLiteral { value: "0".into() }),
            }),
            message: "extra guard".into(),
        },
    );
    assert!(
        !recorded(&changed_guard).0,
        "extra pure work is outside the closed guard"
    );
}

#[test]
fn guarded_struct_map_read_requires_typed_lookup_and_closed_freshness_guard() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("asset-stored-record-fresh-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "assertStoredRecordFresh")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn assertStoredRecordFresh<Private"));
    assert!(source.contains("record_lookup(frame"));

    let mut wrong_key = contract.clone();
    let StateAction::Let { action, .. } = &mut wrong_key.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[1] else {
        unreachable!()
    };
    let Expr::MapLookup { key, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **key = Expr::Parameter {
        name: "recordId".into(),
    };
    assert!(!recorded(&wrong_key).0, "lookup must use the scoped key");

    let mut extra_pure_work = contract;
    let Expr::Sequence { steps, .. } = &mut extra_pure_work.circuits[0].body else {
        unreachable!()
    };
    steps.insert(
        0,
        Expr::Assert {
            condition: Box::new(Expr::NotEqual {
                left: Box::new(Expr::FieldLiteral { value: "1".into() }),
                right: Box::new(Expr::FieldLiteral { value: "0".into() }),
            }),
            message: "extra guard".into(),
        },
    );
    assert!(
        !recorded(&extra_pure_work).0,
        "pure body must stay in the audited subset"
    );
}

#[test]
fn opaque_string_set_recording_requires_closed_typed_operations() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("opaque-string-set-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let statuses = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .map(|capability| (capability.name, capability.recorded))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&contract),
        vec![("addName".into(), true), ("hasName".into(), true)]
    );

    let extra_action = contract.stateful_circuits[0].actions[0].clone();
    contract.stateful_circuits[0].actions.push(extra_action);
    assert_eq!(
        statuses(&contract),
        vec![("addName".into(), false), ("hasName".into(), true)]
    );

    contract.stateful_circuits[0].actions.truncate(1);
    let StateReturn::SetMember { value, .. } = &mut contract.stateful_circuits[1].return_value
    else {
        unreachable!()
    };
    *value = Expr::Default {
        ty: Type::OpaqueString,
    };
    assert_eq!(
        statuses(&contract),
        vec![("addName".into(), true), ("hasName".into(), false)]
    );
}

#[test]
fn welcome_organizer_recording_requires_the_closed_witness_hash_guard() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("welcome-organizer-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let statuses = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        rendered
            .capabilities
            .circuits
            .into_iter()
            .map(|capability| {
                (
                    capability.name,
                    capability.recorded,
                    capability.recording_unavailable.map(|gap| gap.path),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&contract),
        vec![
            ("add_participant".to_owned(), true, None),
            ("add_organizer".to_owned(), true, None),
            ("check_in".to_owned(), true, None),
        ]
    );

    let hash = contract
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    hash.body = Expr::Parameter { name: "sk".into() };
    assert_eq!(
        statuses(&contract),
        vec![
            (
                "add_participant".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            (
                "add_organizer".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            ("check_in".to_owned(), true, None),
        ]
    );

    let hash = contract
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    let original: Contract =
        serde_json::from_str(include_str!("welcome-organizer-schema13-ir.json")).unwrap();
    hash.body = original
        .circuits
        .iter()
        .find(|circuit| circuit.name == "public_key")
        .unwrap()
        .body
        .clone();
    let helper = contract
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "local_sk_or_error")
        .unwrap();
    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut helper.return_value
    else {
        unreachable!()
    };
    let Expr::Sequence { steps, .. } = body.as_mut() else {
        unreachable!()
    };
    steps.push(Expr::Assert {
        condition: Box::new(Expr::Boolean { value: true }),
        message: "extra assertion".into(),
    });
    assert_eq!(
        statuses(&contract),
        vec![
            (
                "add_participant".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            (
                "add_organizer".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            ("check_in".to_owned(), true, None),
        ]
    );
}

fn identity(result: Type, body: Expr) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result,
            body,
        }],
        stateful_circuits: vec![],
    }
}

#[test]
fn recorded_distinct_struct_constructor_calls_require_exact_projection_and_closed_body() {
    let fixture = || -> Contract {
        let mut contract: Contract = serde_json::from_str(include_str!(
            "../fixtures/recorded-struct-constructor-cells.json"
        ))
        .unwrap();
        contract.schema_version = SCHEMA_VERSION;
        contract
    };
    let contract = fixture();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits.iter().all(|c| c.recorded));
    assert!(rendered.source.contains("crate::types::RecCompact1"));
    assert!(rendered.source.contains("crate::types::Rec"));
    assert!(rendered.source.contains("pure_circuits::makeAlpha"));
    assert!(rendered.source.contains("pure_circuits::makeBeta"));

    let mut wrong_identity = fixture();
    let StateAction::Let { bindings, .. } = &mut wrong_identity.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Call { name, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    *name = "makeBeta".into();
    // The typed renderer rejects a cross-module result identity before it
    // can produce either native or recorded Rust.
    assert!(matches!(
        render_with_capabilities(&wrong_identity),
        Err(RenderError::Located { .. })
    ));

    let mut wrong_write = fixture();
    let StateAction::Let { action, .. } = &mut wrong_write.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { action, .. } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::CellWrite { value, .. } = action.as_mut() else {
        unreachable!()
    };
    *value = Expr::Parameter { name: "x".into() };
    assert!(
        !render_with_capabilities(&wrong_write)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut effectful_helper = fixture();
    let previous = effectful_helper.circuits[0].body.clone();
    effectful_helper.circuits[0].body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "side condition".into(),
        }],
        value: Box::new(previous),
    };
    assert!(
        !render_with_capabilities(&effectful_helper)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_nested_uint64_projection_rejects_other_widths_and_effects() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-nested-uint64-cell-counter.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_nested_uint64_")
    );

    let closed = contract.clone();
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[0] else {
        unreachable!()
    };
    bindings[0].ty = Type::Unsigned { max: "255".into() };
    let Expr::UnsignedCast { max, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    *max = "255".into();
    let LedgerFieldKind::Cell { ty } = &mut contract.ledger_fields[0].declaration else {
        unreachable!()
    };
    *ty = Type::Unsigned { max: "255".into() };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed.clone();
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[1] else {
        unreachable!()
    };
    bindings[0].value = Expr::UnsignedLiteral {
        value: "2".into(),
        max: "65535".into(),
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicFlag".into(),
        parameters: vec![],
        result: Type::Boolean,
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If { condition, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **condition = Expr::WitnessCall {
        name: "dynamicFlag".into(),
        arguments: vec![],
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_conditional_field_pair_rejects_unrecorded_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-closed-conditional-field-pair.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_conditional_pair_")
    );

    let closed = contract.clone();
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = action.as_mut() else {
        unreachable!()
    };
    let Expr::Vector { elements, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { condition, .. } = &mut elements[0] else {
        unreachable!()
    };
    **condition = Expr::CellRead {
        field: "flag".into(),
        index: 0,
    };
    assert!(render_with_capabilities(&contract).is_err());

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicField".into(),
        parameters: vec![],
        result: Type::Field,
    });
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = action.as_mut() else {
        unreachable!()
    };
    let Expr::Vector { elements, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { then, .. } = &mut elements[0] else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicField".into(),
            arguments: vec![],
        }),
        ty: Type::Field,
    };
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn recorded_annotated_uint8_rejects_unrecorded_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-annotated-uint8-conditional.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_annotated_uint8_")
    );
    assert!(rendered.source.contains("runtime::Field::from"));

    let closed = contract.clone();
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { condition, .. } = value.as_mut() else {
        unreachable!()
    };
    **condition = Expr::CellRead {
        field: "flag".into(),
        index: 0,
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicArm".into(),
        parameters: vec![],
        result: Type::Unsigned { max: "2".into() },
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicArm".into(),
            arguments: vec![],
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_closed_curve_argument_rejects_effectful_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-closed-curve-argument.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("runtime::hash_to_curve"));
    assert!(rendered.source.contains("runtime::jubjub_point_x"));

    let closed = contract.clone();
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::HashToCurve { value } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::FieldCast { value } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { condition, .. } = value.as_mut() else {
        unreachable!()
    };
    **condition = Expr::CellRead {
        field: "flag".into(),
        index: 0,
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicArm".into(),
        parameters: vec![],
        result: Type::Unsigned { max: "2".into() },
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::HashToCurve { value } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::FieldCast { value } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicArm".into(),
            arguments: vec![],
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn typed_field_helper_hash_then_cell_read_preserves_order_and_rejects_effects() {
    let pair = Type::Vector {
        length: 2,
        element: Box::new(Type::Field),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "source".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "target".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
        ],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "hashPair".into(),
            parameters: vec![Parameter {
                name: "pair".into(),
                ty: pair.clone(),
            }],
            result: Type::Field,
            body: Expr::TransientHash {
                value: Box::new(Expr::Parameter {
                    name: "pair".into(),
                }),
            },
        }],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "helper".into(),
                parameters: vec![Parameter {
                    name: "pair".into(),
                    ty: pair.clone(),
                }],
                result: Type::Field,
                return_value: StateReturn::Expression {
                    value: Expr::Add {
                        left: Box::new(Expr::Call {
                            name: "hashPair".into(),
                            arguments: vec![Expr::Coerce {
                                value: Box::new(Expr::Parameter {
                                    name: "pair".into(),
                                }),
                                ty: pair.clone(),
                            }],
                        }),
                        right: Box::new(Expr::CellRead {
                            field: "source".into(),
                            index: 0,
                        }),
                    },
                },
                actions: vec![],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "save".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "value".into(),
                        ty: Type::Field,
                        value: Expr::Call {
                            name: "helper".into(),
                            arguments: vec![Expr::Coerce {
                                value: Box::new(Expr::Tuple {
                                    elements: vec![
                                        Expr::FieldLiteral { value: "0".into() },
                                        Expr::FieldLiteral { value: "1".into() },
                                    ],
                                }),
                                ty: pair,
                            }],
                        },
                    }],
                    action: Box::new(StateAction::CellWrite {
                        field: "target".into(),
                        index: 1,
                        value: Expr::Parameter {
                            name: "value".into(),
                        },
                    }),
                }],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let recording = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let pure = recording.find("crate::pure_circuits::hashPair(").unwrap();
    let read = recording.find("record_read(frame)").unwrap();
    let write = recording.find("record_write(frame").unwrap();
    assert!(recording.contains("crate::ledger_slots::source"));
    assert!(recording.contains("crate::ledger_slots::target"));
    assert!(pure < read && read < write);

    contract.stateful_circuits[0]
        .actions
        .push(StateAction::CellWrite {
            field: "source".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "2".into() },
        });
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    contract.stateful_circuits[0].actions.clear();
    contract.circuits[0].body = Expr::FieldLiteral { value: "9".into() };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn opaque_set_check_in_records_only_typed_parameter_and_unit_witness() {
    let opaque = Type::OpaqueString;
    let parameter = Parameter {
        name: "participant".into(),
        ty: opaque.clone(),
    };
    let key = Expr::Parameter {
        name: "participant".into(),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "set_local_id".into(),
            parameters: vec![parameter.clone()],
            result: Type::Unit,
        }],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "eligible".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Set { ty: opaque.clone() },
            },
            LedgerField {
                source: None,
                id: "checked".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Set { ty: opaque.clone() },
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "check_in".into(),
            parameters: vec![parameter],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![
                StateAction::Assert {
                    condition: Expr::SetMember {
                        field: "eligible".into(),
                        index: 0,
                        value: Box::new(key.clone()),
                    },
                    message: "not eligible".into(),
                },
                StateAction::SetInsert {
                    field: "checked".into(),
                    index: 1,
                    value: key.clone(),
                },
                StateAction::Expression {
                    value: Expr::WitnessCall {
                        name: "set_local_id".into(),
                        arguments: vec![Expr::Coerce {
                            value: Box::new(key),
                            ty: opaque.clone(),
                        }],
                    },
                },
            ],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains("record_member"));
    assert!(rendered.source.contains("record_insert"));
    assert!(rendered.source.contains("try_witness_metered"));

    let mut incomplete = contract.clone();
    incomplete.stateful_circuits[0].actions.truncate(2);
    let rejected = render_with_capabilities(&incomplete).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[0]"
    );

    let StateAction::Expression {
        value: Expr::WitnessCall { arguments, .. },
    } = &mut contract.stateful_circuits[0].actions[2]
    else {
        unreachable!()
    };
    arguments[0] = Expr::Default { ty: opaque.clone() };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[2].value.arguments[0]"
    );

    contract.stateful_circuits[0].actions[2] = StateAction::SetRemove {
        field: "checked".into(),
        index: 1,
        value: Expr::Parameter {
            name: "participant".into(),
        },
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[2]"
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::OpaqueBytes;
    contract.ledger_fields[0].declaration = LedgerFieldKind::Set {
        ty: Type::OpaqueBytes,
    };
    contract.ledger_fields[1].declaration = LedgerFieldKind::Set {
        ty: Type::OpaqueBytes,
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[0]"
    );
}

#[test]
fn recorded_nested_uint4_accepts_only_closed_typed_arms() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../fixtures/recorded-nested-uint4.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("BoundedUint::<4>::new"));
    assert!(rendered.source.contains("__compact_recorded_nested_field_"));

    let closed = contract.clone();

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If { then, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::Coerce { value, .. } = then.as_mut() else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::UnsignedAdd {
            max: "2".into(),
            left: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "2".into(),
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "2".into(),
            }),
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicArm".into(),
        parameters: vec![],
        result: Type::Unsigned { max: "2".into() },
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If { then, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::Coerce { value, .. } = then.as_mut() else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicArm".into(),
            arguments: vec![],
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn closed_unsigned_ternary_comparison_records_only_matching_literal_arms() {
    let uint = Type::Unsigned { max: "255".into() };
    let narrow = Type::Unsigned { max: "1".into() };
    let arm = |value: &str| Expr::Coerce {
        value: Box::new(Expr::UnsignedLiteral {
            value: value.into(),
            max: "1".into(),
        }),
        ty: narrow.clone(),
    };
    let selected = Expr::UnsignedCast {
        max: "255".into(),
        value: Box::new(Expr::If {
            condition: Box::new(Expr::Parameter { name: "c".into() }),
            then: Box::new(arm("1")),
            otherwise: Box::new(arm("0")),
        }),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![
            Parameter {
                name: "c".into(),
                ty: Type::Boolean,
            },
            Parameter {
                name: "x".into(),
                ty: uint,
            },
            Parameter {
                name: "y".into(),
                ty: narrow.clone(),
            },
        ],
        actions: vec![StateAction::Assert {
            condition: Expr::Equal {
                left: Box::new(Expr::Parameter { name: "x".into() }),
                right: Box::new(selected),
            },
            message: "comparison".into(),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);

    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Equal { right, .. } = condition else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = right.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::Parameter { name: "y".into() }),
        ty: narrow,
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
}

#[test]
fn closed_ternary_struct_member_records_only_literal_field_values() {
    let box_ty = Type::Struct {
        name: "Box".into(),
        fields: vec![StructField {
            name: "f".into(),
            ty: Type::Field,
        }],
    };
    let uint = Type::Unsigned { max: "2".into() };
    let arm = |value: &str| Expr::Coerce {
        value: Box::new(Expr::UnsignedLiteral {
            value: value.into(),
            max: "2".into(),
        }),
        ty: uint.clone(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "out".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![
            Parameter {
                name: "c".into(),
                ty: Type::Boolean,
            },
            Parameter {
                name: "dynamic".into(),
                ty: uint.clone(),
            },
        ],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "box".into(),
                ty: box_ty.clone(),
                value: Expr::StructLiteral {
                    ty: box_ty,
                    fields: vec![Expr::FieldCast {
                        value: Box::new(Expr::If {
                            condition: Box::new(Expr::Parameter { name: "c".into() }),
                            then: Box::new(arm("1")),
                            otherwise: Box::new(arm("2")),
                        }),
                    }],
                },
            }],
            action: Box::new(StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "member".into(),
                    ty: Type::Field,
                    value: Expr::StructField {
                        value: Box::new(Expr::Parameter { name: "box".into() }),
                        field: "f".into(),
                        index: 0,
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "out".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "member".into(),
                    },
                }),
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("__compact_recorded_struct_"));

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::StructLiteral { fields, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::FieldCast { value } = &mut fields[0] else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::Parameter {
            name: "dynamic".into(),
        }),
        ty: uint,
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
}

#[test]
fn native_own_public_key_is_a_private_effect_without_a_user_witness() {
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "test1".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("own_coin_public_key()?"));
    assert!(source.contains("private_transcript_outputs"));
    assert!(source.contains("runtime::fab::AlignedValue::from"));
    assert!(source.contains("pub fn test1<Private>"));
    assert!(!source.contains("Witnesses::ownPublicKey"));
    assert!(!source.contains("pub fn test1<Private, W:"));
}

#[test]
fn native_private_output_flows_through_a_stateful_caller() {
    let helper = StatefulCircuit {
        source: None,
        internal: true,
        name: "helper".into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        }],
    };
    let caller = StatefulCircuit {
        source: None,
        internal: false,
        name: "caller".into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::CircuitCall {
            name: "helper".into(),
            arguments: vec![],
        }],
    };
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![helper, caller],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("call_step.private_transcript_outputs"));
    assert!(source.contains("pub fn caller<Private>"));
    assert!(!source.contains("pub fn caller<Private, W:"));
}

#[test]
fn native_public_key_expression_retains_value_and_private_effect() {
    let native = Expr::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    };
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: false,
                name: "key".into(),
                parameters: vec![],
                actions: vec![],
                result: NativeWitnessBuiltin::OwnPublicKey.result_type(),
                return_value: StateReturn::Expression {
                    value: native.clone(),
                },
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "key_bytes".into(),
                parameters: vec![],
                actions: vec![],
                result: Type::Bytes { length: 32 },
                return_value: StateReturn::Expression {
                    value: Expr::StructField {
                        value: Box::new(native),
                        field: "bytes".into(),
                        index: 0,
                    },
                },
            },
        ],
    };
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("context.own_coin_public_key()?").count(), 2);
    assert!(source.contains("pub struct ZswapCoinPublicKey"));
    assert!(source.contains("pub fn key<Private>"));
    assert!(source.contains("pub fn key_bytes<Private>"));
    assert!(!source.contains("pub fn key<Private, W:"));
    assert!(!source.contains("pub fn key_bytes<Private, W:"));
}

#[test]
fn pure_unit_circuits_keep_effects_in_statement_order() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let literal = render(&contract).unwrap();
    assert!(literal.contains("pub fn identity(value: runtime::Field) -> Result<(), runtime::CompactError> {\n        Ok(())"));

    contract.circuits[0].body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "first".into(),
        }],
        value: Box::new(Expr::Unit),
    };
    let assertions = render(&contract).unwrap();
    assert!(assertions.contains("if !(true)"));
    assert!(assertions.contains("AssertionFailed(\"first\".to_owned())"));
    assert!(!assertions.contains("Ok({"));

    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Unit,
        body: Expr::Unit,
    });
    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        }),
        otherwise: Box::new(Expr::Unit),
    };
    let conditional_call = render(&contract).unwrap();
    assert!(conditional_call.contains("if true"));
    assert!(conditional_call.contains("crate::pure_circuits::target(value)?;"));
    assert!(conditional_call.contains("Ok(())"));
    assert!(!conditional_call.contains("else {\n            ()"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Assert {
            condition: Box::new(Expr::Boolean { value: false }),
            message: "guarded".into(),
        }),
        otherwise: Box::new(Expr::Unit),
    };
    let guarded_assertion = render(&contract).unwrap();
    assert!(guarded_assertion.contains("if (true) && (!(false))"));
    assert!(!guarded_assertion.contains("if true {\n            if"));
}

#[test]
fn pure_fallible_tail_returns_its_result_without_an_extra_try() {
    let mut contract = identity(
        Type::Field,
        Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: contract.circuits[0].parameters.clone(),
        result: Type::Field,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    let called = render(&contract).unwrap();
    assert!(called.contains("crate::pure_circuits::target(value)\n"));
    assert!(!called.contains("Ok(crate::pure_circuits::target(value)?)"));
    assert!(called.contains("Ok(value)"));

    contract.circuits.pop();
    contract.circuits[0].parameters[0].ty = Type::Unsigned { max: "4".into() };
    contract.circuits[0].result = Type::Unsigned { max: "255".into() };
    contract.circuits[0].body = Expr::UnsignedCast {
        max: "255".into(),
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    let cast = render(&contract).unwrap();
    assert!(cast.contains("runtime::cast_unsigned::<4, 255>(value)"));
    assert!(!cast.contains("runtime::cast_unsigned::<4, 255>(value)?"));
}

#[test]
fn opposite_boolean_literal_branches_share_one_ast_normalization() {
    let mut contract = identity(
        Type::Boolean,
        Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: false }),
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Boolean;
    let positive = render(&contract).unwrap();
    assert!(positive.contains("Ok(value)"));
    assert!(!positive.contains("if value { true } else { false }"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Boolean { value: false }),
        otherwise: Box::new(Expr::Boolean { value: true }),
    };
    let negative = render(&contract).unwrap();
    assert!(negative.contains("Ok(!(value))"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let nonliteral = render(&contract).unwrap();
    assert!(nonliteral.contains("if value { value } else { false }"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Sequence {
            steps: vec![Expr::Assert {
                condition: Box::new(Expr::Boolean { value: true }),
                message: "keep assertion".into(),
            }],
            value: Box::new(Expr::Boolean { value: true }),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let effectful = render(&contract).unwrap();
    assert!(effectful.contains("keep assertion"));
    assert!(effectful.contains("if value"));

    contract.circuits[0].result = Type::Unit;
    contract.circuits[0].body = Expr::Assert {
        condition: Box::new(Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Boolean { value: false }),
            otherwise: Box::new(Expr::Boolean { value: true }),
        }),
        message: "one read".into(),
    };
    let assertion = render(&contract).unwrap();
    assert!(assertion.contains("if value {"));
    assert!(!assertion.contains("!(!"));
}

#[test]
fn identical_if_arms_keep_one_condition_and_one_arm() {
    let mut contract = identity(
        Type::Boolean,
        Expr::If {
            condition: Box::new(Expr::Call {
                name: "probe".into(),
                arguments: vec![Expr::Parameter {
                    name: "value".into(),
                }],
            }),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: true }),
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Boolean;
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "probe".into(),
        parameters: contract.circuits[0].parameters.clone(),
        result: Type::Boolean,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    let pure = render(&contract).unwrap();
    assert_eq!(
        pure.matches("crate::pure_circuits::probe(value)?").count(),
        1
    );
    assert!(!pure.contains("if crate::pure_circuits::probe(value)?"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Boolean { value: true }),
        otherwise: Box::new(Expr::Boolean { value: true }),
    };
    let effect_free = render(&contract).unwrap();
    assert!(effect_free.contains("Ok(true)"));
    assert!(!effect_free.contains("let _ = value;"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Call {
            name: "probe".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        }),
        then: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let different = render(&contract).unwrap();
    assert!(different.contains("if crate::pure_circuits::probe(value)?"));

    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "observed".into(),
        parameters: vec![],
        result: Type::Boolean,
    });
    contract.stateful_circuits.push(StatefulCircuit {
        source: None,
        internal: false,
        name: "record".into(),
        parameters: vec![],
        actions: vec![StateAction::Expression {
            value: Expr::If {
                condition: Box::new(Expr::WitnessCall {
                    name: "observed".into(),
                    arguments: vec![],
                }),
                then: Box::new(Expr::Boolean { value: true }),
                otherwise: Box::new(Expr::Boolean { value: true }),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    });
    let stateful = render(&contract).unwrap();
    assert_eq!(stateful.matches(".observed(").count(), 1);
    assert!(stateful.contains("private_transcript_outputs"));
}

#[test]
fn pure_block_conditions_are_lifted_before_if_and_assertions() {
    let block_condition = || Expr::Let {
        bindings: vec![LocalBinding {
            name: "flag".into(),
            ty: Type::Boolean,
            value: Expr::Parameter {
                name: "value".into(),
            },
        }],
        body: Box::new(Expr::Parameter {
            name: "flag".into(),
        }),
    };
    let mut contract = identity(
        Type::Boolean,
        Expr::If {
            condition: Box::new(block_condition()),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: false }),
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Boolean;
    let conditional = render(&contract).unwrap();
    assert!(conditional.contains("let __compact_condition: bool = {"));
    assert!(!conditional.contains("if {"));

    contract.circuits[0].result = Type::Unit;
    contract.circuits[0].body = Expr::Assert {
        condition: Box::new(Expr::If {
            condition: Box::new(block_condition()),
            then: Box::new(Expr::Boolean { value: false }),
            otherwise: Box::new(Expr::Boolean { value: true }),
        }),
        message: "lifted".into(),
    };
    let assertion = render(&contract).unwrap();
    assert!(assertion.contains("let __compact_condition: bool = {"));
    assert!(assertion.contains("AssertionFailed(\"lifted\".to_owned())"));
    assert!(!assertion.contains("if {"));

    contract.circuits[0].parameters[0].name = "__compact_condition".into();
    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Let {
            bindings: vec![LocalBinding {
                name: "flag".into(),
                ty: Type::Boolean,
                value: Expr::Parameter {
                    name: "__compact_condition".into(),
                },
            }],
            body: Box::new(Expr::Parameter {
                name: "flag".into(),
            }),
        }),
        then: Box::new(Expr::Unit),
        otherwise: Box::new(Expr::Unit),
    };
    let collision = render(&contract).unwrap();
    assert!(collision.contains("let __compact_condition_1: bool = {"));

    contract.circuits[0].result = Type::Boolean;
    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "__compact_condition".into(),
        }),
        then: Box::new(Expr::Parameter {
            name: "__compact_condition".into(),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let simple = render(&contract).unwrap();
    assert!(!simple.contains("let __compact_condition_1"));
}

#[test]
fn public_state_getters_follow_declared_types_and_escaped_names() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "type".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        },
        LedgerField {
            source: None,
            id: "flag".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "from".into(),
            index: 2,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
    ];
    let source = render(&contract).unwrap();
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn r#type("));
    assert!(source.contains("pub fn flag(&self)"));
    assert!(source.contains("crate::ledger_slots::r#type.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::flag.inspect(self.state)"));
    assert!(source.contains("pub fn from(&self)"));
    assert!(source.contains("runtime::public_state::PublicStateSource"));
    assert!(source.contains("source.public_state()"));
}

#[test]
fn ledger_validation_retains_compact_source_location() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let location = SourceLocation {
        file: "vault.compact".into(),
        line: 2,
        column: 8,
    };
    contract.ledger_fields.push(LedgerField {
        source: Some(location.clone()),
        id: "balance".into(),
        index: 3,
        path: vec![3, 0],
        declaration: LedgerFieldKind::Counter,
    });
    let error = render(&contract).unwrap_err();
    assert_eq!(
        error,
        RenderError::Located {
            location,
            error: Box::new(RenderError::InvalidLedgerPath(vec![3, 0])),
        }
    );
    assert_eq!(
        error.to_string(),
        "vault.compact line 2 char 8: invalid ledger field path [3, 0]"
    );
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn backend_cli_prints_source_diagnostic_instead_of_debug_structure() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields.push(LedgerField {
        source: Some(SourceLocation {
            file: "vault.compact".into(),
            line: 2,
            column: 8,
        }),
        id: "balance".into(),
        index: 3,
        path: vec![3, 0],
        declaration: LedgerFieldKind::Counter,
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_compact-rust-backend"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&contract).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "compact-rust-backend: vault.compact line 2 char 8: invalid ledger field path [3, 0]\n"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn declaration_errors_retain_their_compact_owner_location() {
    let location = SourceLocation {
        file: "owners.compact".into(),
        line: 7,
        column: 1,
    };
    let located = |error| RenderError::Located {
        location: location.clone(),
        error: Box::new(error),
    };

    let mut pure = identity(Type::Field, Expr::Boolean { value: true });
    pure.circuits[0].source = Some(location.clone());
    assert_eq!(
        render(&pure),
        Err(located(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        }))
    );

    let mut stateful = identity(Type::Unit, Expr::Unit);
    stateful.stateful_circuits.push(StatefulCircuit {
        source: Some(location.clone()),
        name: "write".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "missing".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "1".into() },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    });
    assert_eq!(
        render(&stateful),
        Err(located(RenderError::UnknownLedgerField("missing".into())))
    );

    let mut witnesses = identity(Type::Unit, Expr::Unit);
    for _ in 0..2 {
        witnesses.witnesses.push(WitnessDeclaration {
            source: Some(location.clone()),
            name: "answer".into(),
            parameters: vec![],
            result: Type::Field,
        });
    }
    assert_eq!(
        render(&witnesses),
        Err(located(RenderError::DuplicateWitness("answer".into())))
    );

    let mut constructor = identity(Type::Unit, Expr::Unit);
    constructor.constructor = Some(Constructor {
        source: Some(location.clone()),
        parameters: vec![],
        steps: vec![ConstructorStep::CellWrite {
            field: "missing".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "1".into() },
        }],
    });
    assert_eq!(
        render(&constructor),
        Err(located(RenderError::UnknownLedgerField("missing".into())))
    );

    let mut aliases = identity(Type::Unit, Expr::Unit);
    for _ in 0..2 {
        aliases.type_aliases.push(TypeAlias {
            source: Some(location.clone()),
            name: "Amount".into(),
            ty: Type::Field,
        });
    }
    assert_eq!(
        render(&aliases),
        Err(located(RenderError::ConflictingTypeAlias("Amount".into())))
    );
}

#[test]
fn generated_unit_enum_uses_checked_derive_without_handwritten_codecs() {
    let choice = Type::Enum {
        name: "Choice".into(),
        variants: vec!["yes".into(), "no".into()],
    };
    let mut contract = identity(
        choice.clone(),
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.circuits[0].parameters[0].ty = choice;
    let source = render(&contract).unwrap();
    assert!(source.contains("::midnight_compact_runtime::CompactCellValue"));
    assert!(source.contains("::midnight_compact_runtime::CompactEnum"));
    assert!(source.contains("pub enum Choice"));
    assert!(source.contains("RUST_RUNTIME_ABI == 50"));
    assert!(!source.contains("impl FieldRepr for Choice"));
    assert!(!source.contains("impl BinaryHashRepr for Choice"));
    assert!(!source.contains("impl FromFieldRepr for Choice"));
}

#[test]
fn witness_collection_and_merkle_getters_use_declared_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "observe".into(),
        parameters: vec![],
        result: Type::Unit,
    });
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "seen".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "values".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Map {
                key: Type::Boolean,
                value: Type::Boolean,
            },
        },
        LedgerField {
            source: None,
            id: "queue".into(),
            index: 2,
            path: vec![],
            declaration: LedgerFieldKind::List { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "plain".into(),
            index: 3,
            path: vec![],
            declaration: LedgerFieldKind::MerkleTree {
                depth: 3,
                ty: Type::Boolean,
            },
        },
        LedgerField {
            source: None,
            id: "historic".into(),
            index: 4,
            path: vec![],
            declaration: LedgerFieldKind::HistoricMerkleTree {
                depth: 4,
                ty: Type::Boolean,
            },
        },
    ];
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    for field in ["seen", "values", "queue", "plain", "historic"] {
        assert!(source.contains(&format!(
            "crate::ledger_slots::{field}.witness_view(self.meter)"
        )));
    }
    assert!(!source.contains("runtime::ledger::metered_"));
}

#[test]
fn native_merkle_calls_use_declared_typed_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let leaf = Type::Unsigned { max: "255".into() };
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "plain".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::MerkleTree {
                ty: leaf.clone(),
                depth: 3,
            },
        },
        LedgerField {
            source: None,
            id: "historic".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::HistoricMerkleTree { ty: leaf, depth: 4 },
        },
    ];
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: false,
            name: "plain_full".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Unsigned { max: "255".into() },
            }],
            actions: vec![StateAction::MerkleInsert {
                field: "plain".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Boolean,
            return_value: StateReturn::MerkleIsFull {
                field: "plain".into(),
                index: 0,
            },
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "reset_history".into(),
            parameters: vec![],
            actions: vec![StateAction::HistoricMerkleResetHistory {
                field: "historic".into(),
                index: 1,
            }],
            result: Type::Boolean,
            return_value: StateReturn::HistoricMerkleIsFull {
                field: "historic".into(),
                index: 1,
            },
        },
    ];
    let source = render(&contract).unwrap();
    assert!(source.contains(
        "pub const plain: runtime::slots::MerkleSlot<runtime::BoundedUint<255>, 3u8, false>"
    ));
    assert!(source.contains("pub const historic: runtime::slots::MerkleSlot<"));
    assert!(source.contains("4u8,\n        true,"));
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn plain("));
    assert!(source.contains("pub fn historic("));
    assert!(source.contains("runtime::slots::PlainMerkleStateView<"));
    assert!(source.contains("runtime::slots::HistoricMerkleStateView<"));
    assert!(source.contains("crate::ledger_slots::plain.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::historic.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::plain.insert(context, __compact_param_0)?"));
    assert!(source.contains("crate::ledger_slots::plain.is_full(context)?"));
    assert!(source.contains("crate::ledger_slots::historic.reset_history(context)?"));
    assert!(source.contains("crate::ledger_slots::historic.is_full(context)?"));
    assert!(!source.contains("context.merkle_insert(0,"));
}

#[test]
fn historic_reset_history_records_only_exact_historic_slot() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::HistoricMerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "forget".into(),
        parameters: vec![],
        actions: vec![StateAction::HistoricMerkleResetHistory {
            field: "tree".into(),
            index: 0,
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let supported = render_with_capabilities(&contract).unwrap();
    assert!(supported.capabilities.circuits[0].recorded);
    assert!(
        supported
            .source
            .contains("tree.record_reset_history(frame)")
    );

    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(
        !render_with_capabilities(&contract)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    if let StateAction::HistoricMerkleResetHistory { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 1;
    }
    assert!(
        !render_with_capabilities(&contract)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    if let StateAction::HistoricMerkleResetHistory { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 0;
    }
    contract.ledger_fields[0].path = vec![1];
    assert!(
        !render_with_capabilities(&contract)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
}

#[test]
fn historic_reset_to_default_records_only_exact_historic_slot() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::HistoricMerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "reset".into(),
        parameters: vec![],
        actions: vec![StateAction::HistoricMerkleResetToDefault {
            field: "tree".into(),
            index: 0,
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let supported = render_with_capabilities(&contract).unwrap();
    assert!(supported.capabilities.circuits[0].recorded);
    assert!(
        supported
            .source
            .contains("tree.record_reset_to_default(frame)")
    );

    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(
        !render_with_capabilities(&contract)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    if let StateAction::HistoricMerkleResetToDefault { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 1;
    }
    assert!(
        !render_with_capabilities(&contract)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
    if let StateAction::HistoricMerkleResetToDefault { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 0;
    }
    contract.ledger_fields[0].path = vec![1];
    assert!(
        !render_with_capabilities(&contract)
            .map(|rendered| rendered.capabilities.circuits[0].recorded)
            .unwrap_or(false)
    );
}

#[test]
fn historic_merkle_indexed_hash_records_only_matching_bytes32_and_uint64_sources() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::HistoricMerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "place_hash".into(),
        parameters: vec![
            Parameter {
                name: "hash".into(),
                ty: Type::Bytes { length: 32 },
            },
            Parameter {
                name: "position".into(),
                ty: Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
            },
        ],
        actions: vec![StateAction::HistoricMerkleInsertHashIndex {
            field: "tree".into(),
            index: 0,
            hash: Expr::Parameter {
                name: "hash".into(),
            },
            position: Expr::Parameter {
                name: "position".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_hash_index(frame,"));

    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 31 };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.stateful_circuits[0].parameters[1].ty = Type::Unsigned { max: "255".into() };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[1].ty = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    let StateAction::HistoricMerkleInsertHashIndex { position, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
}

#[test]
fn historic_merkle_hash_append_records_only_exact_historic_bytes32_source() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::HistoricMerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "append_hash".into(),
        parameters: vec![Parameter {
            name: "hash".into(),
            ty: Type::Bytes { length: 32 },
        }],
        actions: vec![StateAction::HistoricMerkleInsertHash {
            field: "tree".into(),
            index: 0,
            hash: Expr::Parameter {
                name: "hash".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_hash(frame,"));

    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 31 };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    let StateAction::HistoricMerkleInsertHash { hash, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *hash = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "hash".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "hash".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
}

#[test]
fn plain_merkle_hash_recording_requires_declared_slot_and_exact_bytes32_source() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "append_hash".into(),
        parameters: vec![Parameter {
            name: "hash".into(),
            ty: Type::Bytes { length: 32 },
        }],
        actions: vec![StateAction::MerkleInsertHash {
            field: "tree".into(),
            index: 0,
            hash: Expr::Parameter {
                name: "hash".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_hash(frame,"));

    if let StateAction::MerkleInsertHash { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 1;
    }
    assert!(render_with_capabilities(&contract).is_err());

    if let StateAction::MerkleInsertHash { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 0;
    }
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 31 };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    let StateAction::MerkleInsertHash { hash, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *hash = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "hash".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "hash".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
}

#[test]
fn plain_merkle_indexed_hash_recording_requires_exact_hash_and_uint64_position() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "place_hash".into(),
        parameters: vec![
            Parameter {
                name: "hash".into(),
                ty: Type::Bytes { length: 32 },
            },
            Parameter {
                name: "position".into(),
                ty: Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
            },
        ],
        actions: vec![StateAction::MerkleInsertHashIndex {
            field: "tree".into(),
            index: 0,
            hash: Expr::Parameter {
                name: "hash".into(),
            },
            position: Expr::Parameter {
                name: "position".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_hash_index(frame,"));

    contract.ledger_fields[0].path = vec![0, 0];
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].path.clear();
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 31 };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.stateful_circuits[0].parameters[1].ty = Type::Unsigned { max: "255".into() };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[1].ty = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    let StateAction::MerkleInsertHashIndex { position, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
}

#[test]
fn indexed_merkle_recording_requires_typed_leaf_and_position_sources() {
    let leaf = Type::Unsigned { max: "255".into() };
    let position = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "plain".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: leaf.clone(),
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: false,
            name: "place".into(),
            parameters: vec![
                Parameter {
                    name: "value".into(),
                    ty: leaf,
                },
                Parameter {
                    name: "position".into(),
                    ty: position.clone(),
                },
            ],
            actions: vec![StateAction::MerkleInsertIndex {
                field: "plain".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
                position: Expr::Parameter {
                    name: "position".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "default_at".into(),
            parameters: vec![Parameter {
                name: "position".into(),
                ty: position,
            }],
            actions: vec![StateAction::MerkleInsertIndexDefault {
                field: "plain".into(),
                index: 0,
                position: Expr::Parameter {
                    name: "position".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
    ];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    assert!(rendered.source.contains(".record_insert_index("));
    assert!(rendered.source.contains(".record_insert_index_default("));

    let StateAction::MerkleInsertIndex { position, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::MerkleInsertIndex"
    );
    assert!(rejected.capabilities.circuits[1].recorded);

    let StateAction::MerkleInsertIndex {
        value, position, ..
    } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::Parameter {
        name: "position".into(),
    };
    *value = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert!(rejected.capabilities.circuits[1].recorded);
}

#[test]
fn indexed_merkle_recording_accepts_only_typed_literal_position_binding() {
    let position = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            depth: 3,
            ty: Type::Unsigned { max: "255".into() },
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        name: "replace".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Unsigned { max: "255".into() },
        }],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "position".into(),
                ty: position.clone(),
                value: Expr::UnsignedLiteral {
                    value: "0".into(),
                    max: u64::MAX.to_string(),
                },
            }],
            action: Box::new(StateAction::MerkleInsertIndex {
                field: "tree".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
                position: Expr::Parameter {
                    name: "position".into(),
                },
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_index("));

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::UnsignedLiteral {
            value: "0".into(),
            max: u64::MAX.to_string(),
        }),
        otherwise: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: u64::MAX.to_string(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert!(!rejected.capabilities.circuits[0].observed_call);
}

#[test]
fn direct_merkle_fullness_reads_record_only_matching_declared_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "plain".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::MerkleTree {
                ty: Type::Boolean,
                depth: 3,
            },
        },
        LedgerField {
            source: None,
            id: "historic".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::HistoricMerkleTree {
                ty: Type::Boolean,
                depth: 3,
            },
        },
    ];
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: false,
            name: "plain_full".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::MerkleIsFull {
                field: "plain".into(),
                index: 0,
            },
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "historic_full".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::HistoricMerkleIsFull {
                field: "historic".into(),
                index: 1,
            },
        },
    ];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.source.matches("record_is_full(frame)?").count(), 2);
    let fullness = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|circuit| circuit.name.ends_with("_full"))
        .collect::<Vec<_>>();
    assert_eq!(fullness.len(), 2);
    for circuit in fullness {
        assert!(circuit.recorded && circuit.observed_call);
    }

    contract.stateful_circuits[0].return_value = StateReturn::HistoricMerkleIsFull {
        field: "plain".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("plain".into()))
    );
    contract.stateful_circuits[0].return_value = StateReturn::MerkleIsFull {
        field: "plain".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("plain".into()))
    );
}

#[test]
fn exported_alias_is_typed_and_reexported() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    let alias = TypeAlias {
        source: None,
        name: "Tag".into(),
        ty: Type::Bytes { length: 8 },
    };
    contract.type_aliases.push(alias.clone());
    let source = render(&contract).unwrap();
    assert!(source.contains("pub type Tag = ::midnight_compact_runtime::FixedBytes<8>;"));
    assert!(source.contains("pub use types::Tag;"));
    contract.type_aliases.push(alias);
    assert_eq!(
        render(&contract),
        Err(RenderError::ConflictingTypeAlias("Tag".into()))
    );
}

#[test]
fn compact_dollar_names_and_rust_reserved_names_render_as_identifiers() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "final".into(),
        },
    );
    contract.circuits[0].name = "vote$commit".into();
    contract.circuits[0].parameters[0].name = "final".into();
    let source = render(&contract).unwrap();
    assert!(source.contains("pub fn vote_commit"), "{source}");
    assert!(source.contains("r#final: runtime::Field"), "{source}");
}

#[test]
fn struct_field_projection_checks_declared_name_and_position() {
    let mut contract = identity(
        Type::Field,
        Expr::StructField {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            field: "amount".into(),
            index: 0,
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    assert!(!render(&contract).unwrap().contains(".amount.clone()"));
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    contract.circuits[0].result = vector.clone();
    contract.circuits[0].parameters[0].ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: vector,
        }],
    };
    assert!(render(&contract).unwrap().contains(".amount).clone()"));
    let Expr::StructField { index, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    *index = 1;
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidStructField("amount".into()))
    );
}

#[test]
fn struct_literal_uses_shorthand_only_for_an_uncloned_field_name() {
    let field = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = identity(
        field.clone(),
        Expr::StructLiteral {
            ty: field,
            fields: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    let source = render(&contract).unwrap();
    assert!(
        source.contains("crate::types::Record { value }"),
        "{source}"
    );
    assert!(!source.contains("value: value"));

    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let record = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: vector.clone(),
        }],
    };
    contract.circuits[0].parameters[0].ty = vector;
    contract.circuits[0].result = record.clone();
    contract.circuits[0].body = Expr::StructLiteral {
        ty: record,
        fields: vec![Expr::Parameter {
            name: "value".into(),
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("value: value.clone()"), "{source}");
}

#[test]
fn struct_literal_checks_field_value_types() {
    let ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = identity(
        ty.clone(),
        Expr::StructLiteral {
            ty,
            fields: vec![Expr::FieldLiteral { value: "7".into() }],
        },
    );
    assert!(render(&contract).is_ok());
    let Expr::StructLiteral { fields, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    fields[0] = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn repeated_vector_parameters_keep_value_semantics() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let mut contract = identity(
        Type::Tuple {
            elements: vec![vector.clone(), vector.clone()],
        },
        Expr::Tuple {
            elements: vec![
                Expr::Parameter {
                    name: "value".into(),
                },
                Expr::Parameter {
                    name: "value".into(),
                },
            ],
        },
    );
    contract.circuits[0].parameters[0].ty = vector;
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("value.clone()").count(), 2);
}

#[test]
fn observed_call_input_clones_non_copy_values_but_not_copy_values() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: vector.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "write".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty: vector,
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::ListPushFront {
            field: "stored".into(),
            index: 0,
            value: Expr::Parameter {
                name: "value".into(),
            },
        }],
    }];
    let source = render(&contract).unwrap();
    assert!(
        source.contains("AlignedValue::from((value).clone())"),
        "{source}"
    );

    contract.ledger_fields[0].declaration = LedgerFieldKind::List { ty: Type::Field };
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    let source = render(&contract).unwrap();
    assert!(source.contains("AlignedValue::from(value)"), "{source}");
    assert!(!source.contains("AlignedValue::from((value).clone())"));
}

#[test]
fn vector_coercion_rejects_length_changes() {
    let target = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let mut contract = identity(
        target.clone(),
        Expr::Coerce {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            ty: target,
        },
    );
    let source = Type::Vector {
        element: Box::new(Type::Unsigned { max: "255".into() }),
        length: 2,
    };
    contract.circuits[0].parameters[0].ty = source.clone();
    assert!(render(&contract).unwrap().contains("into_array()"));
    let Expr::Coerce { ty, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    *ty = Type::Vector {
        element: Box::new(Type::Field),
        length: 3,
    };
    let expected = ty.clone();
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected,
            actual: source,
        })
    );
}

#[test]
fn tuple_coercion_widens_uints_but_rejects_narrowing() {
    let source = Type::Tuple {
        elements: vec![
            Type::Unsigned { max: "255".into() },
            Type::Unsigned { max: "255".into() },
        ],
    };
    let target = Type::Tuple {
        elements: vec![
            Type::Field,
            Type::Unsigned {
                max: "65535".into(),
            },
        ],
    };
    let mut contract = identity(
        target.clone(),
        Expr::Coerce {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            ty: target,
        },
    );
    contract.circuits[0].parameters[0].ty = source;
    assert!(
        render(&contract)
            .unwrap()
            .contains("cast_unsigned::<255, 65535>")
    );
    let Expr::Coerce { ty, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    *ty = Type::Tuple {
        elements: vec![Type::Field, Type::Unsigned { max: "1".into() }],
    };
    contract.circuits[0].result = ty.clone();
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "1".into() },
            actual: Type::Unsigned { max: "255".into() },
        })
    );
}

#[test]
fn constructor_cell_parameters_are_typed_and_validated() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        steps: vec![ConstructorStep::CellWrite {
            field: "stored".into(),
            index: 0,
            value: Expr::Parameter {
                name: "seed".into(),
            },
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_constructor_param_0: runtime::Field"));
    assert!(source.contains("let __compact_constructor_value_0 = __compact_constructor_param_0;"));
    assert!(!source.contains("__compact_constructor_value_0.clone()"));
    assert!(
        source.contains(
            "Result<runtime::context::ConstructorResult<Private>, runtime::CompactError>"
        )
    );

    let constructor = contract.constructor.as_mut().unwrap();
    let ConstructorStep::CellWrite { index, .. } = &mut constructor.steps[0] else {
        panic!("expected Cell write")
    };
    *index = 1;
    assert_eq!(render(&contract), Err(RenderError::InvalidLedgerIndex(1)));
    let ConstructorStep::CellWrite { index, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        panic!("expected Cell write")
    };
    *index = 0;
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Field;
    let duplicate = contract.constructor.as_ref().unwrap().steps[0].clone();
    contract.constructor.as_mut().unwrap().steps.push(duplicate);
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("context.write_cell(0,").count(), 2);
}

#[test]
fn constructor_counter_steps_use_typed_vm_calls() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "amount".into(),
            ty: Type::Unsigned {
                max: "65535".into(),
            },
        }],
        steps: vec![
            ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            },
            ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 3 },
            },
            ConstructorStep::CounterReset {
                field: "count".into(),
                index: 0,
            },
            ConstructorStep::CounterDecrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 1 },
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("increment_counter(0, __compact_constructor_param_0.value() as u16)?"));
    assert!(source.contains("context.increment_counter(0, 3u16)?"));
    assert!(source.contains("context.write_cell(0, 0_u64)?"));
    assert!(source.contains("context.decrement_counter(0, 1u16)?"));
    assert!(source.contains("context.into_constructor_result()"));
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Field,
        })
    );
}

#[test]
fn constructor_for_each_checks_element_type_and_loop_binding() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![ConstructorStep::ForEach {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
            },
            values: vec![Expr::UnsignedLiteral {
                value: "2".into(),
                max: "65535".into(),
            }],
            steps: vec![ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "item".into(),
                },
            }],
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("for __compact_constructor_item_0 in __compact_constructor_values_0"));
    assert!(source.contains("increment_counter(0, __compact_constructor_item_0.value() as u16)?"));
    let ConstructorStep::ForEach { binding, steps, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    binding.name = "_".into();
    let ConstructorStep::CounterIncrement { amount, .. } = &mut steps[0] else {
        unreachable!()
    };
    *amount = CounterAmount::Parameter { name: "_".into() };
    assert!(
        render(&contract)
            .unwrap()
            .contains("__compact_constructor_item_0")
    );
    let ConstructorStep::ForEach { values, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    values[0] = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_vector_parameter_loop_checks_shape_and_conditional_order() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "initial".into(),
            ty: Type::Vector {
                element: Box::new(Type::Boolean),
                length: 2,
            },
        }],
        steps: vec![ConstructorStep::ForEachVector {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Boolean,
            },
            source: Expr::Parameter {
                name: "initial".into(),
            },
            length: 2,
            steps: vec![ConstructorStep::If {
                condition: Expr::Parameter {
                    name: "item".into(),
                },
                then_steps: vec![ConstructorStep::CounterIncrement {
                    field: "count".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                }],
                otherwise_steps: vec![],
            }],
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_constructor_param_0"));
    assert!(source.contains(".iter()"));
    assert!(!source.contains(".cloned()"));
    assert!(!source.contains("into_array()"));
    assert!(source.contains("for __compact_constructor_item_0 in"));
    assert!(source.contains("if __compact_constructor_item_0"));
    assert!(source.contains("context.increment_counter(0, 1u16)?"));

    {
        let ConstructorStep::ForEachVector { length, .. } =
            &mut contract.constructor.as_mut().unwrap().steps[0]
        else {
            unreachable!()
        };
        *length = 3;
    }
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
    let ConstructorStep::ForEachVector { length, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *length = 2;
    let ConstructorStep::ForEachVector { steps, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    let ConstructorStep::If { condition, .. } = &mut steps[0] else {
        unreachable!()
    };
    *condition = Expr::SetMember {
        field: "count".into(),
        index: 0,
        value: Box::new(Expr::Boolean { value: true }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidConstructorInitializer("if".into()))
    );
}

#[test]
fn constructor_set_steps_validate_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "seen".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Boolean },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::SetInsert {
                field: "seen".into(),
                index: 0,
                value: Expr::Boolean { value: true },
            },
            ConstructorStep::SetRemove {
                field: "seen".into(),
                index: 0,
                value: Expr::Boolean { value: false },
            },
            ConstructorStep::SetReset {
                field: "seen".into(),
                index: 0,
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("context.insert_set(0, true)?"));
    assert!(source.contains("context.remove_set(0, false)?"));
    assert!(source.contains("context.reset_set(0)?"));
    let ConstructorStep::SetInsert { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::FieldLiteral { value: "1".into() };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
}

#[test]
fn constructor_list_steps_validate_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: Type::Field },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "3".into() },
            },
            ConstructorStep::ListPopFront {
                field: "items".into(),
                index: 0,
            },
            ConstructorStep::ListReset {
                field: "items".into(),
                index: 0,
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("push_front_list(0,"));
    assert!(source.contains("context.pop_front_list(0)?"));
    assert!(source.contains("context.reset_list(0)?"));
    let ConstructorStep::ListPushFront { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_map_steps_validate_keys_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "table".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Map {
            key: Type::Boolean,
            value: Type::Field,
        },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::MapInsert {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: true },
                value: Expr::FieldLiteral { value: "7".into() },
            },
            ConstructorStep::MapRemove {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: false },
            },
            ConstructorStep::MapReset {
                field: "table".into(),
                index: 0,
            },
            ConstructorStep::MapInsertDefault {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: false },
            },
        ],
    });
    let source = render(&contract).unwrap();
    for method in ["insert_map(0,", "remove_map(0,", "reset_map(0)?"] {
        assert!(source.contains(method), "missing {method}");
    }
    let ConstructorStep::MapInsert { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn nested_set_query_in_cell_write_checks_field_and_item_types() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "seen".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Field },
        },
    ];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check".into(),
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "flag".into(),
            index: 0,
            value: Expr::SetMember {
                field: "seen".into(),
                index: 1,
                value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::seen"));
    assert!(source.contains(".member(context,"));
    assert!(source.contains(".record_member(frame,"), "{source}");
    assert!(source.contains("total_cost += __compact_query_0.gas_cost"));
    let effectful_value = {
        let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0]
        else {
            unreachable!()
        };
        let Expr::SetMember { value: item, .. } = value else {
            unreachable!()
        };
        **item = Expr::Boolean { value: true };
        value.clone()
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.circuits[0].body = effectful_value;
    assert_eq!(render(&contract), Err(RenderError::EffectfulExpression));
}

#[test]
fn native_collection_emptiness_uses_typed_scalar_and_nested_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "entries".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Field },
        },
    ];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check".into(),
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "flag".into(),
            index: 0,
            value: Expr::SetIsEmpty {
                field: "entries".into(),
                index: 1,
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let set_source = render(&contract).unwrap();
    assert!(set_source.contains("crate::ledger_slots::entries.is_empty(context)?"));
    assert!(!set_source.contains("context.is_empty_set(1)?"));

    contract.ledger_fields[1].declaration = LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::Field,
    };
    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::MapIsEmpty {
        field: "entries".into(),
        index: 1,
    };
    let map_source = render(&contract).unwrap();
    assert!(map_source.contains("crate::ledger_slots::entries.is_empty(context)?"));
    assert!(!map_source.contains("context.is_empty_map(1)?"));

    contract.ledger_fields[1].declaration = LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    };
    let nested_source = render(&contract).unwrap();
    assert!(nested_source.contains("crate::ledger_slots::entries.is_empty(context)?"));
    assert!(nested_source.contains("runtime::slots::MapNode<runtime::Field, runtime::Field>"));
    assert!(nested_source.contains("pub const entries:"));
    assert!(!nested_source.contains("context.is_empty_map(1)?"));
}

#[test]
fn nested_set_size_assertion_records_one_ordered_query() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "entries".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check_size".into(),
        parameters: vec![],
        actions: vec![
            StateAction::SetReset {
                field: "entries".into(),
                index: 0,
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::SetSize {
                        field: "entries".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::UnsignedLiteral {
                        value: "0".into(),
                        max: u64::MAX.to_string(),
                    }),
                },
                message: "empty after reset".into(),
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render(&contract).unwrap();
    assert!(rendered.contains("crate::ledger_slots::entries.size(context)?"));
    assert!(rendered.contains(".record_size(frame)?"));
    assert!(rendered.contains("pub fn check_size<Private>"));
    assert!(rendered.contains("pub mod recorded"));
    let reset = rendered.find("record_reset(frame)?").unwrap();
    let size = rendered.find("record_size(frame)?").unwrap();
    assert!(reset < size);

    contract.stateful_circuits[0].actions[1] = StateAction::Assert {
        condition: Expr::Equal {
            left: Box::new(Expr::SetSize {
                field: "entries".into(),
                index: 1,
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "0".into(),
                max: u64::MAX.to_string(),
            }),
        },
        message: "wrong path".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("entries".into()))
    );
}

#[test]
fn recorded_set_assertions_preserve_boolean_queries_and_pure_field_bindings() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "entries".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check_queries".into(),
        parameters: vec![],
        actions: vec![
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::SetIsEmpty {
                        field: "entries".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::Boolean { value: true }),
                },
                message: "initially empty".into(),
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::Let {
                        bindings: vec![LocalBinding {
                            name: "key".into(),
                            ty: Type::Field,
                            value: Expr::FieldLiteral { value: "0".into() },
                        }],
                        body: Box::new(Expr::SetMember {
                            field: "entries".into(),
                            index: 0,
                            value: Box::new(Expr::Parameter { name: "key".into() }),
                        }),
                    }),
                    right: Box::new(Expr::Boolean { value: false }),
                },
                message: "no member".into(),
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render(&contract).unwrap();
    assert!(rendered.contains("record_is_empty(frame)?"));
    assert!(rendered.contains("record_member(frame,"));
    assert!(rendered.contains("__compact_recorded_static_"));
    assert!(rendered.find("record_is_empty(frame)?") < rendered.find("record_member(frame,"));
}

#[test]
fn enum_set_locals_record_typed_keys_in_source_order() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    let names = Type::Enum {
        name: "Names".into(),
        variants: vec!["bill".into(), "sally".into()],
    };
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "c".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: names.clone() },
    }];
    let key = Expr::Parameter { name: "one".into() };
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "one".into(),
                ty: names.clone(),
                value: Expr::EnumVariant {
                    ty: names.clone(),
                    variant: "bill".into(),
                },
            }],
            action: Box::new(StateAction::Sequence {
                actions: vec![
                    StateAction::SetInsert {
                        field: "c".into(),
                        index: 0,
                        value: key.clone(),
                    },
                    StateAction::Assert {
                        condition: Expr::Equal {
                            left: Box::new(Expr::SetMember {
                                field: "c".into(),
                                index: 0,
                                value: Box::new(key.clone()),
                            }),
                            right: Box::new(Expr::Boolean { value: true }),
                        },
                        message: "member".into(),
                    },
                    StateAction::SetRemove {
                        field: "c".into(),
                        index: 0,
                        value: key,
                    },
                ],
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let bound = body.find("__compact_recorded_enum_").unwrap();
    let inserted = body.find("record_insert(frame,").unwrap();
    let observed = body.find("record_member(frame,").unwrap();
    let removed = body.find("record_remove(frame,").unwrap();
    assert!(bound < inserted && inserted < observed && observed < removed);

    // Other native enum bindings keep an explicit recording gap until their
    // source shape has its own replay and proof coverage.
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!();
    };
    bindings[0].value = Expr::Default { ty: names };
    let defaulted = render_with_capabilities(&contract).unwrap();
    assert!(!defaulted.capabilities.circuits[0].recorded);
    assert_eq!(
        defaulted.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::Let",
    );
}

#[test]
fn closed_literal_vector_call_records_set_key_but_other_pure_calls_remain_unavailable() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    contract.circuits = vec![PureCircuit {
        source: None,
        internal: true,
        name: "getVector".into(),
        parameters: vec![],
        result: vector.clone(),
        body: Expr::Vector {
            element: Type::Field,
            elements: vec![
                Expr::FieldLiteral { value: "2".into() },
                Expr::FieldLiteral { value: "12".into() },
            ],
        },
    }];
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "c".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: vector.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "one".into(),
                ty: vector.clone(),
                value: Expr::Call {
                    name: "getVector".into(),
                    arguments: vec![],
                },
            }],
            action: Box::new(StateAction::Sequence {
                actions: vec![
                    StateAction::SetInsert {
                        field: "c".into(),
                        index: 0,
                        value: Expr::Parameter { name: "one".into() },
                    },
                    StateAction::Assert {
                        condition: Expr::Equal {
                            left: Box::new(Expr::SetMember {
                                field: "c".into(),
                                index: 0,
                                value: Box::new(Expr::Parameter { name: "one".into() }),
                            }),
                            right: Box::new(Expr::Boolean { value: true }),
                        },
                        message: "member".into(),
                    },
                ],
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let pure = body.find("crate::pure_circuits::getVector()?").unwrap();
    let inserted = body.find("record_insert(frame,").unwrap();
    let observed = body.find("record_member(frame,").unwrap();
    assert!(pure < inserted && inserted < observed);

    contract.circuits[0].body = Expr::Default { ty: vector.clone() };
    let defaulted = render_with_capabilities(&contract).unwrap();
    assert!(!defaulted.capabilities.circuits[0].recorded);
    assert_eq!(
        defaulted.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::Let"
    );

    contract.circuits[0].body = Expr::Vector {
        element: Type::Field,
        elements: vec![
            Expr::FieldLiteral { value: "2".into() },
            Expr::FieldLiteral { value: "12".into() },
        ],
    };
    contract.circuits[0].parameters = vec![Parameter {
        name: "unused".into(),
        ty: Type::Field,
    }];
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!();
    };
    bindings[0].value = Expr::Call {
        name: "getVector".into(),
        arguments: vec![Expr::FieldLiteral { value: "7".into() }],
    };
    let argument_bearing = render_with_capabilities(&contract).unwrap();
    assert!(!argument_bearing.capabilities.circuits[0].recorded);
}

#[test]
fn direct_boolean_witness_assertion_records_before_cell_write() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.witnesses = vec![WitnessDeclaration {
        source: None,
        name: "echo".into(),
        parameters: vec![Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        }],
        result: Type::Boolean,
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "checked_write".into(),
        parameters: vec![
            Parameter {
                name: "flag".into(),
                ty: Type::Boolean,
            },
            Parameter {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
        actions: vec![
            StateAction::Assert {
                condition: Expr::WitnessCall {
                    name: "echo".into(),
                    arguments: vec![Expr::Parameter {
                        name: "flag".into(),
                    }],
                },
                message: "write denied".into(),
            },
            StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let witness = recorded
        .find(".try_witness_metered(|context, meter|")
        .unwrap();
    let assertion = recorded.find("AssertionFailed(\"write denied\"").unwrap();
    let write = recorded.find(".record_write(frame,").unwrap();
    assert!(witness < assertion && assertion < write);
    assert!(recorded.contains("let __compact_recorded_arg_0: bool = __compact_param_0;"));
    assert!(recorded.contains("pub fn checked_write_call<'observed, Private>"));
}

#[test]
fn vector_expression_preserves_element_type() {
    let mut contract = identity(
        Type::Vector {
            element: Box::new(Type::Field),
            length: 2,
        },
        Expr::Vector {
            element: Type::Field,
            elements: vec![
                Expr::FieldLiteral { value: "3".into() },
                Expr::FieldLiteral { value: "5".into() },
            ],
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::FixedVector::new")
    );
    contract.circuits[0].body = Expr::Vector {
        element: Type::Field,
        elements: vec![
            Expr::Boolean { value: true },
            Expr::FieldLiteral { value: "5".into() },
        ],
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
}

#[test]
fn byte_literals_keep_their_exact_length() {
    let mut contract = identity(
        Type::Bytes { length: 3 },
        Expr::BytesLiteral {
            bytes: vec![1, 2, 0],
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::FixedBytes::new([1u8, 2u8, 0u8])")
    );
    contract.circuits[0].result = Type::Bytes { length: 2 };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 2 },
            actual: Type::Bytes { length: 3 },
        })
    );
}

#[test]
fn field_cast_requires_an_unsigned_operand() {
    let contract = identity(
        Type::Field,
        Expr::FieldCast {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::ExpectedUnsigned(Type::Field))
    );
}

#[test]
fn explicit_field_to_bytes32_checks_operand_and_records_only_typed_counter_read() {
    let mut pure = identity(
        Type::Bytes { length: 32 },
        Expr::FieldToBytes32 {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    let source = render(&pure).unwrap();
    assert!(source.contains("as_le_bytes()"));
    assert!(source.contains("runtime::FixedBytes"));
    pure.circuits[0].body = Expr::FieldToBytes32 {
        value: Box::new(Expr::Boolean { value: true }),
    };
    assert_eq!(
        render(&pure),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );

    let mut stateful = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "instance".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "snapshot".into(),
            internal: false,
            parameters: vec![],
            actions: vec![],
            result: Type::Bytes { length: 32 },
            return_value: StateReturn::Expression {
                value: Expr::FieldToBytes32 {
                    value: Box::new(Expr::FieldCast {
                        value: Box::new(Expr::CounterRead {
                            field: "instance".into(),
                            index: 0,
                        }),
                    }),
                },
            },
        }],
    };
    let rendered = render_with_capabilities(&stateful).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains("record_read(frame)?"));

    stateful.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert_eq!(
        render(&stateful),
        Err(RenderError::UnknownLedgerField("instance".into()))
    );
}

#[test]
fn emits_a_pure_field_circuit_as_parseable_rust() {
    let contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub use midnight_compact_runtime as runtime;"));
    assert!(source.contains("pub fn identity("));
    assert!(source.contains("value: runtime::Field"));
    assert!(source.contains("Ok(value)"));
}

#[test]
fn cell_and_counter_declarations_emit_typed_slots() {
    let mut cell: Contract = serde_json::from_str(include_str!("../fixtures/cell_boolean.json"))
        .expect("Cell fixture parses");
    cell.schema_version = SCHEMA_VERSION;
    let source = render(&cell).unwrap();
    assert!(source.contains("pub mod ledger_slots"));
    assert!(source.contains("pub const flag: runtime::slots::CellSlot<bool>"));
    assert!(source.contains("&[0u8]"));

    let mut counter: Contract = serde_json::from_str(include_str!("../fixtures/counter.json"))
        .expect("Counter fixture parses");
    counter.schema_version = SCHEMA_VERSION;
    let source = render(&counter).unwrap();
    assert!(source.contains("pub const round: runtime::slots::CounterSlot"));
    assert!(source.contains("&[0u8]"));
}

#[test]
fn qualified_set_coin_insert_requires_exact_typed_slot_and_operands() {
    let bytes = Type::Bytes { length: 32 };
    let uint128 = Type::Unsigned {
        max: u128::MAX.to_string(),
    };
    let uint64 = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let coin_fields = vec![
        StructField {
            name: "nonce".into(),
            ty: bytes.clone(),
        },
        StructField {
            name: "color".into(),
            ty: bytes.clone(),
        },
        StructField {
            name: "value".into(),
            ty: uint128,
        },
    ];
    let coin = Type::Struct {
        name: "ShieldedCoinInfo".into(),
        fields: coin_fields.clone(),
    };
    let mut qualified_fields = coin_fields;
    qualified_fields.push(StructField {
        name: "mt_index".into(),
        ty: uint64,
    });
    let qualified = Type::Struct {
        name: "QualifiedShieldedCoinInfo".into(),
        fields: qualified_fields,
    };
    let recipient = Type::Struct {
        name: "Either".into(),
        fields: vec![
            StructField {
                name: "is_left".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "left".into(),
                ty: Type::Struct {
                    name: "ZswapCoinPublicKey".into(),
                    fields: vec![StructField {
                        name: "bytes".into(),
                        ty: bytes.clone(),
                    }],
                },
            },
            StructField {
                name: "right".into(),
                ty: Type::Struct {
                    name: "ContractAddress".into(),
                    fields: vec![StructField {
                        name: "bytes".into(),
                        ty: bytes,
                    }],
                },
            },
        ],
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "coins".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: qualified },
    });
    contract.stateful_circuits.push(StatefulCircuit {
        source: None,
        name: "insert_coin".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "coin".into(),
                ty: coin.clone(),
            },
            Parameter {
                name: "recipient".into(),
                ty: recipient.clone(),
            },
        ],
        actions: vec![StateAction::SetInsertCoin {
            field: "coins".into(),
            index: 0,
            coin: Expr::Parameter {
                name: "coin".into(),
            },
            recipient: Expr::Parameter {
                name: "recipient".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains(".insert_coin("));
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);

    let mut wrong_coin = contract.clone();
    wrong_coin.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert!(matches!(
        render(&wrong_coin),
        Err(RenderError::TypeMismatch { .. })
    ));
    let mut wrong_recipient = contract.clone();
    wrong_recipient.stateful_circuits[0].parameters[1].ty = Type::Field;
    assert!(matches!(
        render(&wrong_recipient),
        Err(RenderError::TypeMismatch { .. })
    ));
    let mut wrong_slot = contract.clone();
    wrong_slot.ledger_fields[0].declaration = LedgerFieldKind::Set { ty: coin };
    assert!(render(&wrong_slot).is_err());
    let mut wrong_index = contract.clone();
    wrong_index.stateful_circuits[0].actions[0] = StateAction::SetInsertCoin {
        field: "coins".into(),
        index: 1,
        coin: Expr::Parameter {
            name: "coin".into(),
        },
        recipient: Expr::Parameter {
            name: "recipient".into(),
        },
    };
    assert!(render(&wrong_index).is_err());
    let mut unknown_binding = contract.clone();
    if let StateAction::SetInsertCoin { coin, .. } =
        &mut unknown_binding.stateful_circuits[0].actions[0]
    {
        *coin = Expr::Parameter {
            name: "escaped".into(),
        };
    }
    assert!(matches!(
        render(&unknown_binding),
        Err(RenderError::UnknownParameter(_))
    ));
    contract.schema_version = 13;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(13)));
}

#[test]
fn single_element_tuple_is_a_tuple_in_type_and_expression() {
    let contract = identity(
        Type::Tuple {
            elements: vec![Type::Field],
        },
        Expr::Tuple {
            elements: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    let source = render(&contract).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let module = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "pure_circuits" => Some(module),
            _ => None,
        })
        .expect("expected pure circuits module");
    let function = module
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) => Some(function),
            _ => None,
        })
        .expect("expected function");
    let syn::ReturnType::Type(_, result) = &function.sig.output else {
        panic!("expected return type")
    };
    let syn::Type::Path(result) = result.as_ref() else {
        panic!("expected Result")
    };
    let syn::PathArguments::AngleBracketed(arguments) =
        &result.path.segments.last().unwrap().arguments
    else {
        panic!("expected type arguments")
    };
    let syn::GenericArgument::Type(syn::Type::Tuple(tuple)) = &arguments.args[0] else {
        panic!("expected tuple")
    };
    assert_eq!(tuple.elems.len(), 1);
    assert!(tuple.elems.trailing_punct());
    assert!(source.contains("Ok((value,))"));
}

#[test]
fn rejects_bad_schema_and_unknown_references() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.schema_version = 2;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(2)));
    contract.schema_version = 4;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(4)));
    contract.schema_version = 6;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(6)));
    contract.schema_version = 7;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(7)));
    contract.schema_version = 8;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(8)));
    contract.schema_version = 9;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(9)));
    contract.schema_version = 10;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(10)));
    contract.schema_version = 11;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(11)));
    contract.schema_version = 12;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(12)));
    contract.schema_version = 13;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(13)));
    contract.schema_version = SCHEMA_VERSION;
    contract.circuits[0].body = Expr::Parameter {
        name: "missing".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn rejects_wrong_native_hash_opening_and_conversion_types() {
    let field = Expr::Parameter {
        name: "value".into(),
    };
    let contract = identity(
        Type::Bytes { length: 32 },
        Expr::PersistentCommit {
            value: Box::new(field.clone()),
            opening: Box::new(field.clone()),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );

    let contract = identity(
        Type::Field,
        Expr::DegradeToTransient {
            value: Box::new(field),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_field_as_jubjub_point_coordinate_input() {
    let contract = identity(
        Type::Field,
        Expr::JubjubPointX {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::JubjubPoint,
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_type_mismatch_and_invalid_identifier() {
    let mut contract = identity(
        Type::Boolean,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
    contract.circuits[0].name = "bad-name".into();
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
}

#[test]
fn unsigned_add_and_cast_reject_field_inputs() {
    let mut contract = identity(
        Type::Unsigned { max: "511".into() },
        Expr::UnsignedAdd {
            max: "511".into(),
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "255".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
    contract.circuits[0].body = Expr::UnsignedCast {
        max: "511".into(),
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
}

#[test]
fn unsigned_literal_checks_the_declared_maximum() {
    let mut contract = identity(
        Type::Unsigned { max: "255".into() },
        Expr::UnsignedLiteral {
            value: "255".into(),
            max: "255".into(),
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("BoundedUint::<255>::new(255u128)")
    );
    contract.circuits[0].body = Expr::UnsignedLiteral {
        value: "256".into(),
        max: "255".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidUnsignedLiteral {
            value: "256".into(),
            max: "255".into(),
        })
    );
}

#[test]
fn field_literal_requires_canonical_ledger_field_value() {
    let mut contract = identity(Type::Field, Expr::FieldLiteral { value: "42".into() });
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from(42u128)")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "042".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral("042".into()))
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "340282366920938463463374607431768211456".into(),
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from_le_bytes")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "115792089237316195423570985008687907853269984665640564039457584007913129639935"
            .into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral(
            "115792089237316195423570985008687907853269984665640564039457584007913129639935".into()
        ))
    );
}

#[test]
fn pure_call_checks_target_arity_and_argument_types() {
    let mut contract = identity(
        Type::Field,
        Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownCircuit("target".into()))
    );
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: vec![Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "flag".into(),
        },
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.circuits[1].parameters[0].ty = Type::Field;
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.clear();
    }
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 0,
        })
    );
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.push(Expr::Parameter {
            name: "value".into(),
        });
    }
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::pure_circuits::target(value)")
    );
}

#[test]
fn local_binding_checks_declared_type_and_scope() {
    let mut contract = identity(
        Type::Field,
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "saved".into(),
                ty: Type::Boolean,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            body: Box::new(Expr::Parameter {
                name: "saved".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::Let { bindings, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    bindings[0].ty = Type::Field;
    let source = render(&contract).unwrap();
    assert!(source.contains("let __compact_local_saved: runtime::Field = value;"));
    assert!(source.contains("Ok({"));
}

#[test]
fn conditional_requires_boolean_condition_and_equal_branch_types() {
    let mut contract = identity(
        Type::Field,
        Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            otherwise: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::If {
        condition,
        otherwise,
        ..
    } = &mut contract.circuits[0].body
    else {
        unreachable!()
    };
    **condition = Expr::Boolean { value: true };
    **otherwise = Expr::Boolean { value: false };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn refuses_to_add_a_boolean_to_a_field() {
    let contract = identity(
        Type::Field,
        Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::Boolean { value: true }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn rejects_noncanonical_or_unsupported_unsigned_maxima() {
    for max in [
        "08",
        "-1",
        "452312848583266388373324160190187140051835877600158453279131187530910662656",
    ] {
        let contract = Contract {
            schema_version: SCHEMA_VERSION,
            type_aliases: vec![],
            constructor: None,
            witnesses: vec![],
            ledger_fields: vec![],
            circuits: vec![PureCircuit {
                source: None,
                internal: false,
                name: "id_u".into(),
                parameters: vec![Parameter {
                    name: "value".into(),
                    ty: Type::Unsigned { max: max.into() },
                }],
                result: Type::Unsigned { max: max.into() },
                body: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            stateful_circuits: vec![],
        };
        assert_eq!(
            render(&contract),
            Err(RenderError::InvalidUnsignedMaximum(max.into()))
        );
    }
}

#[test]
fn unknown_json_fields_are_rejected() {
    let json = r#"{"schema_version":4,"ledger_fields":[],"circuits":[],"stateful_circuits":[],"rust_source":"panic!()"}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());

    let json = r#"{"schema_version":4,"ledger_fields":[{"id":"round","index":0,"declaration":{"kind":"counter"},"rust_source":"panic!()"}],"circuits":[],"stateful_circuits":[]}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());
}

#[test]
fn witness_calls_require_a_declared_witness_and_matching_signature() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: vec![],
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "secret".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Field,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "read_secret".into(),
            parameters: vec![Parameter {
                name: "flag".into(),
                ty: Type::Boolean,
            }],
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![Expr::Parameter {
                        name: "flag".into(),
                    }],
                },
            },
        }],
    };
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub trait Witnesses<Private>"));
    assert!(source.contains("#[runtime::compact_witness_bridge]"));
    assert!(source.contains("private_transcript_outputs"));
    assert!(source.contains("pub struct Contract<W>"));
    assert!(source.contains("W: TryWitnesses<Private>"));
    assert!(source.contains("crate::ledger_contract::read_secret("));
    assert!(source.contains("&self.witnesses"));

    contract.witnesses.clear();
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownWitness("secret".into()))
    );
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![],
        result: Type::Field,
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "secret".into(),
            expected: 0,
            actual: 1,
        })
    );
}

#[test]
fn witness_cell_and_counter_getters_use_declared_composite_slots() {
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "observe".into(),
            parameters: vec![],
            result: Type::Unit,
        }],
        ledger_fields: (0..16)
            .map(|position| LedgerField {
                source: None,
                id: match position {
                    0 => "flag".into(),
                    1 => "round".into(),
                    _ => format!("filler_{position}"),
                },
                index: u8::from(position != 0),
                path: if position == 0 {
                    vec![0, 0]
                } else {
                    vec![1, position - 1]
                },
                declaration: if position == 1 {
                    LedgerFieldKind::Counter
                } else {
                    LedgerFieldKind::Cell { ty: Type::Boolean }
                },
            })
            .collect(),
        circuits: vec![],
        stateful_circuits: vec![],
    };

    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    let compact = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    assert!(compact.contains(
        "pubconstflag:runtime::slots::CellSlot<bool>=runtime::slots::CellSlot::new(&[0u8,0u8]"
    ));
    assert!(compact.contains(
        "pubconstround:runtime::slots::CounterSlot=runtime::slots::CounterSlot::new(&[1u8,0u8]"
    ));
    assert!(source.contains("crate::ledger_slots::flag.witness_read(self.meter)"));
    assert!(source.contains("crate::ledger_slots::round.witness_read(self.meter)?"));
    assert!(!source.contains("self.meter.read_cell::<"));
}

#[test]
fn witnessed_field_cell_and_nested_call_use_native_frame() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.witnesses = vec![WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
    }];
    let seed = Expr::Parameter {
        name: "seed".into(),
    };
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: true,
            name: "inner".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "value".into(),
                    ty: Type::Field,
                    value: Expr::WitnessCall {
                        name: "secret".into(),
                        arguments: vec![seed.clone()],
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "cell".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "value".into(),
                    },
                }),
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "outer".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::CircuitCall {
                name: "inner".into(),
                arguments: vec![seed],
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
    ];
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub fn recording(&self) -> recorded::BorrowedContract<'_, W>"));
    assert!(!source.contains("::core::convert::From<&'a super::Contract<W>>"));
    assert_eq!(source.matches("CircuitFrame::new(context)").count(), 2);
    assert!(source.contains("meter: &'a runtime::context::WitnessReadMeter<'a>"));
    assert!(source.contains("crate::ledger_slots::cell.witness_read(self.meter)"));
    assert!(source.contains(".try_witness_metered(|context, meter|"));
    assert!(source.contains("#[runtime::compact_witness_bridge]"));
    assert!(!source.contains("pub trait TryWitnesses<Private>"));
    assert!(source.contains("ledger_slots::cell.write(context"));
    assert!(source.contains(".apply(|context|"));
    assert!(source.contains("self::inner(context, witnesses"));
    assert!(source.contains("Ok(frame.finish(()))"));

    // `$` is valid Compact syntax; `r#` cases exercise the typed IR only.
    for name in ["recording", "r#recording"] {
        let mut collision = contract.clone();
        collision.stateful_circuits[1].name = name.into();
        let rendered = render(&collision).unwrap();
        assert!(!rendered.contains("pub fn recording(&self)"), "{name}");
        assert!(rendered.contains(&format!("pub fn {name}<Private>(")));
        assert!(rendered.contains("pub recording: recorded::Contract"));
        assert!(rendered.contains("::core::convert::From<&'a super::Contract<W>>"));
        assert!(rendered.contains("witnesses: &contract.witnesses"));

        // A private-IR circuit named `from` (a Compact keyword) cannot prevent
        // fully qualified trait construction of the witnessed handle.
        let mut from = collision.stateful_circuits[1].clone();
        from.name = "from".into();
        collision.stateful_circuits.push(from);
        let rendered = render(&collision).unwrap();
        syn::parse_file(&rendered).unwrap();
        assert!(rendered.contains("pub fn from<Private>("));
        assert!(rendered.contains("::core::convert::From<&'a super::Contract<W>>"));
    }
    for name in ["outer_call", "outer$call", "r#outer_call"] {
        let mut collision = contract.clone();
        let mut exported = collision.stateful_circuits[1].clone();
        exported.name = name.into();
        collision.stateful_circuits.push(exported);
        let rendered = render_with_capabilities(&collision).unwrap();
        assert!(!rendered.source.contains("pub fn outer_call<'observed"));
        assert!(!rendered.capabilities.circuits[0].observed_call);
        assert_eq!(
            rendered.capabilities.circuits[0]
                .observed_call_unavailable
                .as_ref()
                .unwrap()
                .code
                .as_str(),
            "name_collision"
        );
        assert!(rendered.capabilities.circuits[1].observed_call);
    }

    let mut two_parameter_witness = contract.clone();
    two_parameter_witness.stateful_circuits[1]
        .parameters
        .push(Parameter {
            name: "extra".into(),
            ty: Type::Boolean,
        });
    let witnessed_source = render(&two_parameter_witness).unwrap();
    syn::parse_file(&witnessed_source).unwrap();
    assert!(witnessed_source.contains("pub fn outer_call<'observed, Private>("));
    assert!(witnessed_source.contains("let input = runtime::fab::AlignedValue::from(("));
    assert!(witnessed_source.contains("W: super::TryWitnesses<Private>"));

    let mut second_caller = contract.stateful_circuits[1].clone();
    second_caller.name = "outer_again".into();
    contract.stateful_circuits.push(second_caller);
    let shared = render(&contract).unwrap();
    syn::parse_file(&shared).unwrap();
    assert_eq!(
        shared.matches("fn __compact_recorded_body_inner<").count(),
        1
    );
    assert_eq!(shared.matches("__compact_recorded_body_inner").count(), 3);

    let mut name_collision = contract.stateful_circuits[1].clone();
    name_collision.name = "__compact_recorded_body_inner".into();
    contract.stateful_circuits.push(name_collision);
    let generated = syn::parse_file(&render(&contract).unwrap()).unwrap();
    let ledger = generated
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => Some(module),
            _ => None,
        })
        .unwrap();
    let recorded = ledger
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "recorded" => Some(module),
            _ => None,
        })
        .unwrap();
    let mut function_names = std::collections::HashSet::new();
    for item in &recorded.content.as_ref().unwrap().1 {
        if let syn::Item::Fn(function) = item {
            assert!(function_names.insert(function.sig.ident.to_string()));
        }
    }
}

#[test]
fn recorded_field_returning_helper_is_shared_across_callers() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.witnesses = vec![WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![],
        result: Type::Field,
    }];
    let caller = |name: &str| StatefulCircuit {
        source: None,
        internal: false,
        name: name.into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "next".into(),
                ty: Type::Field,
                value: Expr::Call {
                    name: "innerValue".into(),
                    arguments: vec![],
                },
            }],
            action: Box::new(StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "next".into(),
                },
            }),
        }],
    };
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: true,
            name: "innerValue".into(),
            parameters: vec![],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![],
                },
            },
            actions: vec![],
        },
        caller("outerA"),
        caller("outerB"),
    ];
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_eq!(
        source
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        source.matches("__compact_recorded_body_innerValue").count(),
        3
    );
    assert!(source.contains("pub fn outerA<Private"));
    assert!(source.contains("pub fn outerB<Private"));

    contract.stateful_circuits[0].parameters = vec![
        Parameter {
            name: "first".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "second".into(),
            ty: Type::Field,
        },
    ];
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Add {
            left: Box::new(Expr::WitnessCall {
                name: "secret".into(),
                arguments: vec![],
            }),
            right: Box::new(Expr::Add {
                left: Box::new(Expr::Parameter {
                    name: "first".into(),
                }),
                right: Box::new(Expr::Parameter {
                    name: "second".into(),
                }),
            }),
        },
    };
    for circuit in &mut contract.stateful_circuits[1..] {
        let StateAction::Let { bindings, .. } = &mut circuit.actions[0] else {
            unreachable!()
        };
        bindings[0].value = Expr::Call {
            name: "innerValue".into(),
            arguments: vec![
                Expr::FieldLiteral { value: "1".into() },
                Expr::FieldLiteral { value: "2".into() },
            ],
        };
    }
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_eq!(
        source
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        source.matches("__compact_recorded_body_innerValue").count(),
        3
    );
    assert!(source.contains("__compact_param_0: runtime::Field"));
    assert!(source.contains("__compact_param_1: runtime::Field"));

    // The compiler lowers `value = innerValue(1, 2) + 3` to a Field
    // action binding with an expression-nested call. Reuse the same body.
    let mut nested = contract.stateful_circuits[1].clone();
    nested.name = "outerNested".into();
    let StateAction::Let { bindings, .. } = &mut nested.actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::Add {
        left: Box::new(Expr::Call {
            name: "innerValue".into(),
            arguments: vec![
                Expr::FieldLiteral { value: "1".into() },
                Expr::FieldLiteral { value: "2".into() },
            ],
        }),
        right: Box::new(Expr::FieldLiteral { value: "3".into() }),
    };
    contract.stateful_circuits.push(nested);
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_eq!(
        source
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        source.matches("__compact_recorded_body_innerValue").count(),
        4
    );
    assert!(source.contains("pub fn outerNested<Private"));

    contract
        .stateful_circuits
        .retain(|circuit| circuit.internal || circuit.name == "outerNested");
    let nested_only = render(&contract).unwrap();
    assert_eq!(
        nested_only
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        nested_only
            .matches("__compact_recorded_body_innerValue")
            .count(),
        2
    );

    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Multiply {
            left: Box::new(Expr::Parameter {
                name: "first".into(),
            }),
            right: Box::new(Expr::Parameter {
                name: "second".into(),
            }),
        },
    };
    let multiplied = render(&contract).unwrap();
    assert!(multiplied.contains("pub mod recorded"));
    assert!(multiplied.contains("__compact_recorded_product_"));

    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Add {
            left: Box::new(Expr::Call {
                name: "innerValue".into(),
                arguments: vec![
                    Expr::Parameter {
                        name: "first".into(),
                    },
                    Expr::Parameter {
                        name: "second".into(),
                    },
                ],
            }),
            right: Box::new(Expr::FieldLiteral { value: "1".into() }),
        },
    };
    assert!(matches!(
        render(&contract),
        Err(RenderError::UnsupportedStatefulCall(name)) if name == "innerValue"
    ));
}

#[test]
fn state_action_must_reference_the_declared_ledger_field_and_index() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "increment".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CounterIncrement {
                field: "round".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 1 },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::round.increment(context, 1)?"));
    assert!(source.contains("pub mod recorded"));
    assert!(source.contains("pub fn recording(&self) -> &recorded::Contract"));
    assert!(source.contains("&self.recording"));
    assert!(source.contains("crate::ledger_slots::round.record_increment(frame, 1u16)?"));
    assert!(source.contains("Ok(frame.finish(()))"));
    assert!(source.contains("pub fn increment_call<'observed, Private>("));
    assert!(source.contains("runtime::transaction::RecordedCall::new("));
    assert!(source.contains("\"increment\","));

    for name in ["recording", "r#recording"] {
        let mut recording_circuit = contract.clone();
        recording_circuit.stateful_circuits[0].name = name.into();
        let source = render(&recording_circuit).unwrap();
        assert!(source.contains(&format!("pub fn {name}<Private>(")));
        assert!(!source.contains("pub fn recording(&self) -> &recorded::Contract"));
    }

    contract.stateful_circuits[0].actions[0] = StateAction::CounterDecrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.decrement(context, 1)?")
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.record_decrement(frame, 1u16)?")
    );
    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 0,
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.reset(context)?")
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.record_reset(frame)?")
    );
    assert!(render(&contract).unwrap().contains("pub fn increment_call"));

    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean };
    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::Counter;

    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "missing".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("missing".into()))
    );

    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 1,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );

    contract.ledger_fields[0].index = 1;
    assert_eq!(render(&contract), Err(RenderError::InvalidLedgerIndex(1)));
}

#[test]
fn nested_call_exposes_a_trace_only_when_every_step_is_supported() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "inner".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::CounterReset {
                    field: "round".into(),
                    index: 0,
                }],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "outer".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::CircuitCall {
                    name: "inner".into(),
                    arguments: vec![],
                }],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("pub mod recorded"));
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    assert_eq!(rendered.capabilities.circuits[0].name, "outer");
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);

    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].result = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    contract.stateful_circuits[0].return_value = StateReturn::CounterRead {
        field: "round".into(),
        index: 0,
    };
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));
}

#[test]
fn closed_pure_field_call_records_let_value_without_admitting_hashes() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![
            PureCircuit {
                source: None,
                internal: false,
                name: "identityField".into(),
                parameters: vec![Parameter {
                    name: "input".into(),
                    ty: Type::Field,
                }],
                result: Type::Field,
                body: Expr::Parameter {
                    name: "input".into(),
                },
            },
            PureCircuit {
                source: None,
                internal: false,
                name: "literalField".into(),
                parameters: vec![],
                result: Type::Field,
                body: Expr::Call {
                    name: "identityField".into(),
                    arguments: vec![Expr::FieldLiteral {
                        value: "819310549611346726241370945440405716213240158234039660170669895299022906775".into(),
                    }],
                },
            },
        ],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "writeLiteral".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "computed".into(),
                    ty: Type::Field,
                    value: Expr::Call {
                        name: "literalField".into(),
                        arguments: vec![],
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "value".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "computed".into(),
                    },
                }),
            }],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered
        .source
        .contains("let __compact_recorded_pure_field_0: runtime::Field = crate::pure_circuits::literalField()?;"));
    assert!(rendered.source.contains("record_write(frame"));

    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Call {
            name: "literalField".into(),
            arguments: vec![],
        },
    };
    let returned = render_with_capabilities(&contract).unwrap();
    assert!(returned.capabilities.circuits[0].recorded);
    assert!(returned
        .source
        .contains("let __compact_recorded_pure_return_1: runtime::Field = crate::pure_circuits::literalField()?;"));

    contract.stateful_circuits[0].actions = vec![StateAction::CellWrite {
        field: "value".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "7".into() },
    }];

    contract.circuits[1].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "5".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateReturn::Expression"
    );
}

#[test]
fn closed_field_pair_hash_call_records_typed_bridge_without_admitting_other_hashes() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let tuple = Type::Tuple {
        elements: vec![Type::Field, Type::Field],
    };
    let pair = Expr::Tuple {
        elements: vec![
            Expr::FieldLiteral { value: "3".into() },
            Expr::FieldLiteral { value: "4".into() },
        ],
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![
            PureCircuit {
                source: None,
                internal: false,
                name: "hashVector".into(),
                parameters: vec![Parameter {
                    name: "input".into(),
                    ty: vector.clone(),
                }],
                result: Type::Field,
                body: Expr::TransientHash {
                    value: Box::new(Expr::Parameter {
                        name: "input".into(),
                    }),
                },
            },
            PureCircuit {
                source: None,
                internal: false,
                name: "bridge".into(),
                parameters: vec![],
                result: Type::Field,
                body: Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "tuple".into(),
                        ty: tuple.clone(),
                        value: pair,
                    }],
                    body: Box::new(Expr::Call {
                        name: "hashVector".into(),
                        arguments: vec![Expr::Coerce {
                            value: Box::new(Expr::Parameter {
                                name: "tuple".into(),
                            }),
                            ty: vector.clone(),
                        }],
                    }),
                },
            },
        ],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "writeHash".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "computed".into(),
                    ty: Type::Field,
                    value: Expr::Call {
                        name: "bridge".into(),
                        arguments: vec![],
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "value".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "computed".into(),
                    },
                }),
            }],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = rendered.source.split("pub mod recorded").nth(1).unwrap();
    assert!(
        body.find("pure_circuits::bridge()?").unwrap() < body.find("record_write(frame").unwrap()
    );

    contract.circuits[0].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "3".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "Expr::Call"
    );
}

#[test]
fn typed_pair_hash_call_records_shared_unit_helper_and_conditional_caller() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let argument = Expr::Vector {
        element: Type::Field,
        elements: vec![
            Expr::FieldLiteral { value: "0".into() },
            Expr::FieldLiteral { value: "1".into() },
        ],
    };
    let call = StateAction::CircuitCall {
        name: "storeVector".into(),
        arguments: vec![argument],
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "value".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "flag".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
            },
        ],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "hashVector".into(),
            parameters: vec![Parameter {
                name: "input".into(),
                ty: vector.clone(),
            }],
            result: Type::Field,
            body: Expr::TransientHash {
                value: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
            },
        }],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "storeVector".into(),
                parameters: vec![Parameter {
                    name: "v".into(),
                    ty: vector,
                }],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "hash".into(),
                        ty: Type::Field,
                        value: Expr::Call {
                            name: "hashVector".into(),
                            arguments: vec![Expr::Parameter { name: "v".into() }],
                        },
                    }],
                    action: Box::new(StateAction::CellWrite {
                        field: "value".into(),
                        index: 0,
                        value: Expr::Parameter {
                            name: "hash".into(),
                        },
                    }),
                }],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "bare".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![call.clone()],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "conditional".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::If {
                    condition: Expr::CellRead {
                        field: "flag".into(),
                        index: 1,
                    },
                    then: Box::new(call),
                    otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
                }],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 2);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    assert!(
        rendered
            .source
            .contains("fn __compact_recorded_body_storeVector")
    );
    assert!(rendered.source.contains("pure_circuits::hashVector("));
    assert!(rendered.source.contains("__compact_recorded_pair_arg_"));

    contract.circuits[0].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "7".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(rejected.capabilities.circuits.iter().all(|c| !c.recorded));
    assert!(rejected.capabilities.circuits.iter().all(|c| {
        c.recording_unavailable
            .as_ref()
            .is_some_and(|gap| gap.ir_node == "Expr::Call")
    }));
}

#[test]
fn unsupported_pure_call_and_supported_field_arithmetic_have_exact_capabilities() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "square".into(),
            parameters: vec![Parameter {
                name: "input".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
            body: Expr::Multiply {
                left: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
                right: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: Some(SourceLocation {
                file: "pure-call.compact".into(),
                line: 4,
                column: 1,
            }),
            internal: false,
            name: "write".into(),
            parameters: vec![
                Parameter {
                    name: "left".into(),
                    ty: Type::Field,
                },
                Parameter {
                    name: "right".into(),
                    ty: Type::Field,
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "value".into(),
                index: 0,
                value: Expr::Call {
                    name: "square".into(),
                    arguments: vec![Expr::Parameter {
                        name: "left".into(),
                    }],
                },
            }],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.source.contains("pub mod recorded"));
    assert!(!rendered.source.contains("pub fn write_call"));
    let report = serde_json::to_value(&rendered.capabilities).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["circuits"][0]["name"], "write");
    assert_eq!(report["circuits"][0]["source"]["file"], "pure-call.compact");
    assert_eq!(report["circuits"][0]["source"]["line"], 4);
    assert_eq!(report["circuits"][0]["recorded"], false);
    assert_eq!(report["circuits"][0]["observed_call"], false);
    assert_eq!(
        report["circuits"][0]["recording_unavailable"]["code"],
        "unsupported_expression"
    );
    assert_eq!(
        report["circuits"][0]["recording_unavailable"]["path"],
        "actions[0].value"
    );
    assert_eq!(
        report["circuits"][0]["observed_call_unavailable"]["code"],
        "recording_unavailable"
    );
    let published = render_with_proof_capabilities(
        &contract,
        &serde_json::json!({"circuits": [
            {"name": "square", "proof": false},
            {"name": "write", "proof": true}
        ]}),
    )
    .unwrap();
    assert_eq!(published.source, rendered.source);
    let published_report = serde_json::to_value(&published.capabilities).unwrap();
    assert_eq!(published_report["schema_version"], 3);
    assert_eq!(published_report["circuits"][0]["proof_required"], true);
    assert_eq!(
        published_report["circuits"][0]["recording_status"],
        "unavailable"
    );
    assert!(matches!(
        render_with_proof_capabilities(&contract, &serde_json::json!({"circuits": []})),
        Err(RenderError::ProofApplicability(_))
    ));

    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let left = Box::new(Expr::Parameter {
        name: "left".into(),
    });
    let right = Box::new(Expr::Parameter {
        name: "right".into(),
    });
    *value = Expr::Subtract {
        left: left.clone(),
        right: right.clone(),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("pub mod recorded"));
    assert!(rendered.source.contains("__compact_recorded_difference_0"));
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::Multiply { left, right };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("__compact_recorded_product_0"));
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
}

#[test]
fn standalone_unit_witness_is_recorded_only_with_its_exact_signature() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "private_increment".into(),
            parameters: vec![],
            result: Type::Unit,
        }],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "increment".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![
                StateAction::CounterIncrement {
                    field: "round".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                },
                StateAction::Expression {
                    value: Expr::WitnessCall {
                        name: "private_increment".into(),
                        arguments: vec![],
                    },
                },
            ],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let counter = rendered
        .source
        .find("record_increment(frame, 1u16)")
        .unwrap();
    let witness = rendered.source.find(".try_witness_metered").unwrap();
    assert!(counter < witness);
    assert!(rendered.source.contains(".private_increment("));

    contract.witnesses[0].result = Type::Boolean;
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[1]"
    );
    contract.witnesses[0].result = Type::Unit;
    contract.stateful_circuits[0].actions[1] = StateAction::Expression {
        value: Expr::Boolean { value: true },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn standalone_field_witness_records_typed_argument_but_rejects_nested_effects() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "secret".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
        }],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "run".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![
                StateAction::Expression {
                    value: Expr::WitnessCall {
                        name: "secret".into(),
                        arguments: vec![Expr::FieldLiteral { value: "1".into() }],
                    },
                },
                StateAction::CounterIncrement {
                    field: "round".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                },
            ],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains("__compact_recorded_witness_arg_0"));
    assert!(rendered.source.contains(".try_witness_metered"));

    let StateAction::Expression {
        value: Expr::WitnessCall { arguments, .. },
    } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    arguments[0] = Expr::WitnessCall {
        name: "secret".into(),
        arguments: vec![Expr::FieldLiteral { value: "2".into() }],
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    let gap = rejected.capabilities.circuits[0]
        .recording_unavailable
        .as_ref()
        .unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_expression");
    assert_eq!(gap.path, "actions[0].value.arguments[0]");
}

#[test]
fn boolean_pair_hash_cell_assertion_records_read_before_counter_write() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "value".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "asserts".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Counter,
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "different".into(),
                parameters: vec![Parameter {
                    name: "v".into(),
                    ty: vector,
                }],
                result: Type::Boolean,
                return_value: StateReturn::Expression {
                    value: Expr::NotEqual {
                        left: Box::new(Expr::TransientHash {
                            value: Box::new(Expr::Parameter { name: "v".into() }),
                        }),
                        right: Box::new(Expr::CellRead {
                            field: "value".into(),
                            index: 0,
                        }),
                    },
                },
                actions: vec![],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "check".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![
                    StateAction::Assert {
                        condition: Expr::Call {
                            name: "different".into(),
                            arguments: vec![Expr::Vector {
                                element: Type::Field,
                                elements: vec![
                                    Expr::FieldLiteral { value: "0".into() },
                                    Expr::FieldLiteral { value: "1".into() },
                                ],
                            }],
                        },
                        message: "different".into(),
                    },
                    StateAction::CounterIncrement {
                        field: "asserts".into(),
                        index: 1,
                        amount: CounterAmount::Literal { value: 1 },
                    },
                ],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let source = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let hash = source.find("runtime::transient_hash(").unwrap();
    let read = source.find("record_read(frame)").unwrap();
    let write = source.find("record_increment(frame").unwrap();
    assert!(hash < read && read < write);

    let StateReturn::Expression { value } = &mut contract.stateful_circuits[0].return_value else {
        unreachable!();
    };
    let Expr::NotEqual { left, .. } = value else {
        unreachable!()
    };
    **left = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "7".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::Assert"
    );
}

#[test]
fn recording_gaps_follow_the_first_definite_ir_failure() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "run".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Sequence {
                actions: vec![
                    StateAction::CounterReset {
                        field: "round".into(),
                        index: 0,
                    },
                    StateAction::Expression {
                        value: Expr::Boolean { value: true },
                    },
                    StateAction::Expression {
                        value: Expr::Boolean { value: false },
                    },
                ],
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_action");
    assert_eq!(gap.ir_node, "StateAction::Expression");
    assert_eq!(gap.path, "actions[0].actions[1]");

    contract.stateful_circuits[0].actions.clear();
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert_eq!(
        report.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .code
            .as_str(),
        "no_recorded_effect"
    );

    contract.stateful_circuits[0].result = Type::Boolean;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Boolean { value: true },
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_return");
    assert_eq!(gap.ir_node, "StateReturn::Expression");
    assert_eq!(gap.path, "return_value");
}

#[test]
fn conditional_recording_rejects_an_unsupported_unselected_branch() {
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "run".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(StateAction::CounterReset {
                    field: "round".into(),
                    index: 0,
                }),
                otherwise: Box::new(StateAction::Expression {
                    value: Expr::Boolean { value: false },
                }),
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let capability = &report.circuits[0];
    assert!(!capability.recorded);
    assert!(!capability.observed_call);
    let gap = capability.recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_action");
    assert_eq!(gap.ir_node, "StateAction::Expression");
    assert_eq!(gap.path, "actions[0].otherwise");
}

#[test]
fn recording_gaps_include_unsupported_type_and_called_callee_path() {
    let unsupported_cell_type = Type::OpaqueString;
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "small".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell {
                ty: unsupported_cell_type.clone(),
            },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "set".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "small".into(),
                index: 0,
                value: Expr::Default {
                    ty: unsupported_cell_type,
                },
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_type");
    assert_eq!(gap.ir_node, "StateAction::CellWrite");
    assert_eq!(gap.path, "actions[0].value");

    contract.ledger_fields[0].declaration = LedgerFieldKind::Counter;
    contract.stateful_circuits[0].name = "inner".into();
    contract.stateful_circuits[0].internal = true;
    contract.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::Boolean { value: true },
    }];
    let mut outer = contract.stateful_circuits[0].clone();
    outer.name = "outer".into();
    outer.internal = false;
    outer.actions = vec![StateAction::CircuitCall {
        name: "inner".into(),
        arguments: vec![],
    }];
    contract.stateful_circuits.push(outer);
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_action");
    assert_eq!(gap.path, "callee[inner].actions[0]");
    assert_eq!(gap.ir_node, "StateAction::Expression");
    for circuit in report.circuits {
        assert_eq!(circuit.recorded, circuit.recording_unavailable.is_none());
        assert_eq!(
            circuit.observed_call,
            circuit.observed_call_unavailable.is_none()
        );
    }
}

#[test]
fn stateful_parameters_are_checked_before_cell_writes() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "set_flag".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "flag".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("__compact_param_0: bool"));
    assert!(source.contains("crate::ledger_slots::flag.write(context, __compact_param_0)?"));

    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    let duplicate = contract.stateful_circuits[0].parameters[0].clone();
    contract.stateful_circuits[0].parameters.push(duplicate);
    assert_eq!(
        render(&contract),
        Err(RenderError::DuplicateParameter("value".into()))
    );
}

#[test]
fn counter_parameter_requires_uint16_and_a_known_name() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "increment_by".into(),
            parameters: vec![Parameter {
                name: "amount".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CounterIncrement {
                field: "round".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_param_0.value() as u16"));
    assert!(source.contains("pub fn increment_by_call<'observed, Private>("));
    assert!(source.contains("let input = runtime::fab::AlignedValue::from(amount);"));
    assert_eq!(source.matches("amount:").count(), 3);

    let mut two_parameters = contract.clone();
    two_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "unused".into(),
            ty: Type::Boolean,
        });
    let two_parameter_source = render(&two_parameters).unwrap();
    assert!(two_parameter_source.contains("pub mod recorded"));
    assert!(two_parameter_source.contains("pub fn increment_by_call<'observed"));
    assert!(two_parameter_source.contains("let input = runtime::fab::AlignedValue::from(("));
    assert!(two_parameter_source.contains("amount, unused"));

    let mut three_parameters = two_parameters.clone();
    three_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "another_unused".into(),
            ty: Type::Field,
        });
    let three_parameter_source = render(&three_parameters).unwrap();
    assert!(three_parameter_source.contains("pub fn increment_by_call<'observed"));
    assert!(three_parameter_source.contains("runtime::fab::AlignedValue::concat"));
    assert!(three_parameter_source.contains("AlignedValue::from(another_unused)"));

    let mut four_parameters = three_parameters.clone();
    four_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "unused_3".into(),
            ty: Type::Field,
        });
    let four_parameter_source = render(&four_parameters).unwrap();
    assert!(!four_parameter_source.contains("clippy::too_many_arguments"));

    let mut five_parameters = four_parameters.clone();
    five_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "unused_4".into(),
            ty: Type::Field,
        });
    let five_parameter_source = render(&five_parameters).unwrap();
    assert!(five_parameter_source.contains("clippy::too_many_arguments"));

    let mut twelve_parameters = five_parameters.clone();
    for index in 5..12 {
        twelve_parameters.stateful_circuits[0]
            .parameters
            .push(Parameter {
                name: format!("unused_{index}"),
                ty: Type::Field,
            });
    }
    let twelve_parameter_source = render(&twelve_parameters).unwrap();
    syn::parse_file(&twelve_parameter_source).unwrap();
    assert!(twelve_parameter_source.contains("pub fn increment_by_call<'observed"));
    assert!(twelve_parameter_source.contains("AlignedValue::from(unused_11)"));
    assert!(
        twelve_parameter_source
            .matches("clippy::too_many_arguments")
            .count()
            >= 4
    );

    // A Rust raw spelling is accepted only through the typed IR.
    for collision_name in [
        "increment_by_call",
        "increment_by$call",
        "r#increment_by_call",
    ] {
        let mut collision = two_parameters.clone();
        let mut exported = collision.stateful_circuits[0].clone();
        exported.name = collision_name.into();
        exported.parameters.clear();
        exported.actions[0] = StateAction::CounterIncrement {
            field: "round".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        };
        collision.stateful_circuits.push(exported);
        let rendered = render_with_capabilities(&collision).unwrap();
        assert!(
            !rendered
                .source
                .contains("pub fn increment_by_call<'observed")
        );
        assert_eq!(
            rendered
                .capabilities
                .circuits
                .iter()
                .map(|circuit| (
                    circuit.name.as_str(),
                    circuit.recorded,
                    circuit.observed_call
                ))
                .collect::<Vec<_>>(),
            [("increment_by", true, false), (collision_name, true, true)]
        );
        let gap = rendered.capabilities.circuits[0]
            .observed_call_unavailable
            .as_ref()
            .unwrap();
        assert_eq!(gap.code.as_str(), "name_collision");
        assert_eq!(gap.path, "name");
    }

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned { max: "255".into() };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Unsigned { max: "255".into() },
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned {
        max: "65535".into(),
    };
    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Parameter {
            name: "missing".into(),
        },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn ledger_read_return_must_match_the_declared_cell() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "read_flag".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::CellRead {
                field: "flag".into(),
                index: 0,
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::flag.read(context)?"));
    assert!(source.contains("let result = read_step.result"));

    contract.stateful_circuits[0].result = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].result = Type::Boolean;
    contract.stateful_circuits[0].return_value = StateReturn::CellRead {
        field: "flag".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("flag".into()))
    );
}

#[test]
fn counter_read_returns_uint64() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "read_round".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Unsigned {
                max: u64::MAX.to_string(),
            },
            return_value: StateReturn::CounterRead {
                field: "round".into(),
                index: 0,
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::round.read(context)?"));
    assert!(source.contains("crate::ledger_slots::round"));
    assert!(source.contains(".record_read(frame)?"));
    assert!(source.contains("pub struct Contract<W>"));
    assert!(source.contains("crate::ledger_contract::read_round(context"));

    contract.stateful_circuits[0].result = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Boolean,
        })
    );
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
fn set_actions_require_the_declared_element_type() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "seen".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "add".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            actions: vec![StateAction::SetInsert {
                field: "seen".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("pub const seen: runtime::slots::SetSlot<bool>"));
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn seen("));
    assert!(source.contains("runtime::ledger::SetView<'a, bool, D>"));
    assert!(source.contains("crate::ledger_slots::seen.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::seen.insert(context, __compact_param_0)?"));
    assert!(
        source.contains(".record_insert(frame, __compact_param_0)?"),
        "{source}"
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    contract.stateful_circuits[0].actions = vec![StateAction::SetReset {
        field: "seen".into(),
        index: 0,
    }];
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::seen.reset(context)?")
    );
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::SetSize {
        field: "seen".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
    contract.stateful_circuits[0].result = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::seen.size(context)?"));
    assert!(source.contains(".record_size(frame)?"), "{source}");

    // The native slot handles this type, but the recorder must not expose a
    // method until its complete typed expression path is supported.
    contract.ledger_fields[0].declaration = LedgerFieldKind::Set {
        ty: Type::Bytes { length: 32 },
    };
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::SetInsert {
        field: "seen".into(),
        index: 0,
        value: Expr::Parameter {
            name: "value".into(),
        },
    }];
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));
}

#[test]
fn map_insert_and_lookup_require_key_and_value_types() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "table".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Map {
                key: Type::Boolean,
                value: Type::Field,
            },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "put".into(),
            parameters: vec![
                Parameter {
                    name: "key".into(),
                    ty: Type::Boolean,
                },
                Parameter {
                    name: "value".into(),
                    ty: Type::Field,
                },
            ],
            actions: vec![StateAction::MapInsert {
                field: "table".into(),
                index: 0,
                key: Expr::Parameter { name: "key".into() },
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("MapSlot<bool, runtime::Field>"), "{source}");
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn table("));
    assert!(source.contains("runtime::ledger::MapView<'a, bool, runtime::Field, D>"));
    assert!(source.contains("crate::ledger_slots::table.inspect(self.state)"));
    assert!(
        source.contains(".insert(context, __compact_param_0, __compact_param_1)?"),
        "{source}"
    );
    assert!(source.contains(".record_insert(frame,"), "{source}");
    contract.stateful_circuits[0].parameters[1].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].parameters[1].ty = Type::Field;
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::MapLookup {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    };
    let source = render(&contract).unwrap();
    assert!(source.contains(".lookup(context, __compact_param_0)?"));
    assert!(source.contains(".record_lookup(frame,"), "{source}");
    contract.stateful_circuits[0].result = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::MapInsertDefault {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains(".insert_default(context, __compact_param_0)?"));
    assert!(source.contains(".record_insert_default(frame,"), "{source}");
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    contract.stateful_circuits[0].actions = vec![StateAction::MapReset {
        field: "table".into(),
        index: 0,
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains(".reset(context)?"));
    assert!(source.contains(".record_reset(frame)?"), "{source}");
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::MapSize {
        field: "table".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::Map {
        key: Type::Boolean,
        value: Type::Bytes { length: 32 },
    };
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::MapInsertDefault {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    }];
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));

    // A nested ledger Map has a structural slot, but its value still has no
    // CellValue codec for scalar insert and lookup operations.
    contract.stateful_circuits.clear();
    contract.ledger_fields[0].declaration = LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Unsigned {
                max: u64::MAX.to_string(),
            }),
        },
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("constructor_map()"));
    assert!(source.contains("MapSlot<"));
    assert!(source.contains("runtime::slots::MapNode<"));
    assert!(source.contains("runtime::BoundedUint<18446744073709551615>"));
    assert!(!source.contains("pub struct PublicStateView<'a, D:"));
}

#[test]
fn nested_list_queries_record_in_order_and_reject_wrong_shape() {
    let maybe_field = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    let head = Expr::ListHead {
        field: "items".into(),
        index: 0,
        ty: maybe_field.clone(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![
            StateAction::Assert {
                condition: Expr::ListIsEmpty {
                    field: "items".into(),
                    index: 0,
                },
                message: "empty".into(),
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::ListLength {
                        field: "items".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::UnsignedLiteral {
                        value: "0".into(),
                        max: u64::MAX.to_string(),
                    }),
                },
                message: "length".into(),
            },
            StateAction::Assert {
                condition: Expr::NotEqual {
                    left: Box::new(Expr::StructField {
                        value: Box::new(head.clone()),
                        field: "is_some".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::Boolean { value: true }),
                },
                message: "head absent".into(),
            },
            StateAction::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "7".into() },
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::StructField {
                        value: Box::new(head),
                        field: "value".into(),
                        index: 1,
                    }),
                    right: Box::new(Expr::FieldLiteral { value: "7".into() }),
                },
                message: "head value".into(),
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let empty = recorded.find("record_is_empty(frame)").unwrap();
    let length = recorded.find("record_length(frame)").unwrap();
    let head = recorded.find("record_head::<crate::types::Maybe").unwrap();
    let push = recorded.find("record_push_front(frame,").unwrap();
    assert!(empty < length && length < head && head < push);

    {
        let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[1]
        else {
            unreachable!()
        };
        let Expr::Equal { left, .. } = condition else {
            unreachable!()
        };
        **left = Expr::ListLength {
            field: "items".into(),
            index: 1,
        };
    }
    assert!(matches!(
        render(&contract),
        Err(RenderError::UnknownLedgerField(_))
    ));

    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[1]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    **left = Expr::ListLength {
        field: "items".into(),
        index: 0,
    };
    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[4]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = left.as_mut() else {
        unreachable!()
    };
    let Expr::ListHead { ty, .. } = value.as_mut() else {
        unreachable!()
    };
    *ty = Type::Boolean;
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
}

#[test]
fn enum_list_head_value_records_only_with_matching_typed_head() {
    let names = Type::Enum {
        name: "Names".into(),
        variants: vec!["bill".into(), "sally".into()],
    };
    let maybe_names = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: names.clone(),
            },
        ],
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: names.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![StateAction::Assert {
            condition: Expr::Equal {
                left: Box::new(Expr::StructField {
                    value: Box::new(Expr::ListHead {
                        field: "items".into(),
                        index: 0,
                        ty: maybe_names.clone(),
                    }),
                    field: "value".into(),
                    index: 1,
                }),
                right: Box::new(Expr::EnumVariant {
                    ty: names.clone(),
                    variant: "bill".into(),
                }),
            },
            message: "head value".into(),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("record_head::<crate::types::Maybe")
    );

    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = left.as_mut() else {
        unreachable!()
    };
    let Expr::ListHead { ty, .. } = value.as_mut() else {
        unreachable!()
    };
    *ty = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
}

#[test]
fn field_vector_list_head_value_records_without_admitting_other_vector_elements() {
    fn contract_for(element: Type) -> Contract {
        let vector = Type::Vector {
            element: Box::new(element),
            length: 4,
        };
        let maybe = Type::Struct {
            name: "Maybe".into(),
            fields: vec![
                StructField {
                    name: "is_some".into(),
                    ty: Type::Boolean,
                },
                StructField {
                    name: "value".into(),
                    ty: vector.clone(),
                },
            ],
        };
        let mut contract = identity(Type::Unit, Expr::Unit);
        contract.ledger_fields = vec![LedgerField {
            source: None,
            id: "items".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::List { ty: vector.clone() },
        }];
        contract.stateful_circuits = vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "test".into(),
            parameters: vec![],
            actions: vec![StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::StructField {
                        value: Box::new(Expr::ListHead {
                            field: "items".into(),
                            index: 0,
                            ty: maybe,
                        }),
                        field: "value".into(),
                        index: 1,
                    }),
                    right: Box::new(Expr::Default { ty: vector }),
                },
                message: "head vector".into(),
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }];
        contract
    }

    let field = render_with_capabilities(&contract_for(Type::Field)).unwrap();
    assert!(field.capabilities.circuits[0].recorded);
    assert!(field.source.contains("record_head::<crate::types::Maybe"));

    let boolean = render_with_capabilities(&contract_for(Type::Boolean)).unwrap();
    assert!(!boolean.capabilities.circuits[0].recorded);
    assert!(!boolean.capabilities.circuits[0].observed_call);
}

#[test]
fn literal_bytes_list_head_records_only_with_exact_length_and_excludes_opaque() {
    fn contract_for(element: Type, value: Expr) -> Contract {
        let maybe = Type::Struct {
            name: "Maybe".into(),
            fields: vec![
                StructField {
                    name: "is_some".into(),
                    ty: Type::Boolean,
                },
                StructField {
                    name: "value".into(),
                    ty: element.clone(),
                },
            ],
        };
        let mut contract = identity(Type::Unit, Expr::Unit);
        contract.ledger_fields = vec![LedgerField {
            source: None,
            id: "items".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::List {
                ty: element.clone(),
            },
        }];
        contract.stateful_circuits = vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "test".into(),
            parameters: vec![],
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "expected".into(),
                    ty: element.clone(),
                    value,
                }],
                action: Box::new(StateAction::Assert {
                    condition: Expr::Equal {
                        left: Box::new(Expr::StructField {
                            value: Box::new(Expr::ListHead {
                                field: "items".into(),
                                index: 0,
                                ty: maybe,
                            }),
                            field: "value".into(),
                            index: 1,
                        }),
                        right: Box::new(Expr::Parameter {
                            name: "expected".into(),
                        }),
                    },
                    message: "head bytes".into(),
                }),
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }];
        contract
    }

    let bytes = Type::Bytes { length: 4 };
    let literal = Expr::BytesLiteral {
        bytes: vec![1, 2, 3, 4],
    };
    let positive = render_with_capabilities(&contract_for(bytes.clone(), literal)).unwrap();
    assert!(positive.capabilities.circuits[0].recorded);
    assert!(positive.source.contains("__compact_recorded_bytes_"));
    assert!(
        positive
            .source
            .contains("record_head::<crate::types::Maybe")
    );

    let wrong_length = contract_for(
        bytes,
        Expr::BytesLiteral {
            bytes: vec![1, 2, 3],
        },
    );
    assert!(matches!(
        render(&wrong_length),
        Err(RenderError::TypeMismatch { .. })
    ));

    let opaque = Type::OpaqueString;
    let unsupported =
        render_with_capabilities(&contract_for(opaque.clone(), Expr::Default { ty: opaque }))
            .unwrap();
    assert!(!unsupported.capabilities.circuits[0].recorded);
    assert!(!unsupported.capabilities.circuits[0].observed_call);
}

#[test]
fn list_push_front_and_length_validate_declared_types() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "items".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::List { ty: Type::Field },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "prepend".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("ListSlot<runtime::Field>"), "{source}");
    assert!(
        source.contains("pub struct PublicStateView<'a, D:"),
        "{source}"
    );
    assert!(source.contains("pub fn items("), "{source}");
    assert!(
        source.contains("crate::ledger_slots::items.inspect(self.state)"),
        "{source}"
    );
    assert!(
        source.contains("runtime::ledger::ListView<'a, runtime::Field, D>"),
        "{source}"
    );
    let mut list_only = contract.clone();
    list_only.stateful_circuits.clear();
    let list_only_source = render(&list_only).unwrap();
    assert!(list_only_source.contains("pub struct PublicStateView<'a, D:"));
    assert!(list_only_source.contains("pub fn items("));
    assert!(
        source.contains(".push_front(context, __compact_param_0)?"),
        "{source}"
    );
    assert!(source.contains(".record_push_front(frame,"), "{source}");
    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::ListLength {
        field: "items".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
    let maybe_field = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    contract.stateful_circuits[0].return_value = StateReturn::ListHead {
        field: "items".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: maybe_field.clone(),
            actual: Type::Unit,
        })
    );
    contract.stateful_circuits[0].result = maybe_field;
    let source = render(&contract).unwrap();
    assert!(
        source.contains(".head::<crate::types::Maybe, _, _>(context)?"),
        "{source}"
    );
    assert!(
        source.contains(".record_head::<crate::types::Maybe, _, _>(frame)?"),
        "{source}"
    );
    let mut second_instantiation = contract.stateful_circuits[0].result.clone();
    let Type::Struct { name, .. } = &mut second_instantiation else {
        unreachable!();
    };
    *name = "MaybeCompact1".into();
    contract.stateful_circuits[0].result = second_instantiation.clone();
    let source = render(&contract).unwrap();
    assert!(
        source.contains(".head::<crate::types::MaybeCompact1, _, _>(context)?"),
        "{source}"
    );
    assert!(
        source.contains(".record_head::<crate::types::MaybeCompact1, _, _>(frame)?"),
        "{source}"
    );
    for invalid in ["MaybeCompact", "MaybeCompactX", "MaybeOther"] {
        let mut actual = second_instantiation.clone();
        let Type::Struct { name, .. } = &mut actual else {
            unreachable!();
        };
        *name = invalid.into();
        contract.stateful_circuits[0].result = actual.clone();
        assert_eq!(
            render(&contract),
            Err(RenderError::TypeMismatch {
                expected: Type::Struct {
                    name: "Maybe".into(),
                    fields: match &second_instantiation {
                        Type::Struct { fields, .. } => fields.clone(),
                        _ => unreachable!(),
                    },
                },
                actual,
            })
        );
    }
    let mut wrong_value = second_instantiation.clone();
    let Type::Struct { fields, .. } = &mut wrong_value else {
        unreachable!();
    };
    fields[1].ty = Type::Boolean;
    contract.stateful_circuits[0].result = wrong_value.clone();
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: second_instantiation,
            actual: wrong_value,
        })
    );
}

#[test]
fn struct_definitions_are_shared_by_name_and_must_match() {
    let pair = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: pair.clone(),
            }],
            result: pair.clone(),
            body: Expr::Parameter {
                name: "value".into(),
            },
        }],
        stateful_circuits: vec![],
    };
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("pub struct Pair").count(), 1);

    contract.circuits[0].result = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "active".into(),
            ty: Type::Boolean,
        }],
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::ConflictingStruct("Pair".into()))
    );
}

#[test]
fn stateful_call_checks_target_and_arguments() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: vec![],
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: false,
                name: "target".into(),
                parameters: vec![Parameter {
                    name: "value".into(),
                    ty: Type::Field,
                }],
                actions: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "caller".into(),
                parameters: vec![Parameter {
                    name: "seed".into(),
                    ty: Type::Field,
                }],
                actions: vec![StateAction::CircuitCall {
                    name: "target".into(),
                    arguments: vec![Expr::Parameter {
                        name: "seed".into(),
                    }],
                }],
                result: Type::Unit,
                return_value: StateReturn::Unit,
            },
        ],
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("let call_step = target(context, __compact_call_argument_0)?")
    );
    contract.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "target".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 0
        })
    );
    contract.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "missing".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownCircuit("missing".into()))
    );
    contract.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "caller".into(),
        arguments: vec![Expr::Parameter {
            name: "value".into(),
        }],
    }];
    contract.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "target".into(),
        arguments: vec![Expr::Parameter {
            name: "seed".into(),
        }],
    }];
    assert!(matches!(
        render(&contract),
        Err(RenderError::UnsupportedStatefulCall(_))
    ));
    let source = SourceLocation {
        file: "recursive.compact".into(),
        line: 4,
        column: 1,
    };
    contract.stateful_circuits[0].source = Some(source.clone());
    assert!(matches!(
        render(&contract),
        Err(RenderError::Located { location, error })
            if location == source && matches!(*error, RenderError::UnsupportedStatefulCall(_))
    ));
}

#[test]
fn pure_call_action_checks_arguments_and_discards_result() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "caller".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::PureCall {
            name: "identity".into(),
            arguments: vec![Expr::Parameter {
                name: "seed".into(),
            }],
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::pure_circuits::identity(__compact_param_0)?;")
    );
    contract.stateful_circuits[0].actions = vec![StateAction::PureCall {
        name: "identity".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "identity".into(),
            expected: 1,
            actual: 0
        })
    );
}

#[test]
fn recorded_pure_assert_call_rejects_extra_pure_steps() {
    let assertion = Expr::Assert {
        condition: Box::new(Expr::NotEqual {
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::FieldLiteral { value: "0".into() }),
        }),
        message: "positive".into(),
    };
    let mut contract = identity(
        Type::Unit,
        Expr::Sequence {
            steps: vec![assertion.clone()],
            value: Box::new(Expr::Unit),
        },
    );
    contract.circuits[0].name = "require_positive".into();
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "save".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::PureCall {
            name: "require_positive".into(),
            arguments: vec![Expr::Coerce {
                value: Box::new(Expr::Parameter {
                    name: "seed".into(),
                }),
                ty: Type::Field,
            }],
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("pure_circuits::require_positive"));

    let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    steps.push(assertion);
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
}

#[test]
fn recorded_struct_pure_guard_requires_closed_typed_call_and_continuation() {
    let policy = Type::Struct {
        name: "Policy".into(),
        fields: vec![StructField {
            name: "enabled".into(),
            ty: Type::Boolean,
        }],
    };
    let record = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let uint64 = Type::Unsigned {
        max: "18446744073709551615".into(),
    };
    let parameters = vec![
        Parameter {
            name: "policy".into(),
            ty: policy,
        },
        Parameter {
            name: "record".into(),
            ty: record,
        },
        Parameter {
            name: "time".into(),
            ty: uint64,
        },
    ];
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "written".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    });
    contract.circuits[0] = PureCircuit {
        source: None,
        internal: false,
        name: "guard".into(),
        parameters: parameters.clone(),
        result: Type::Unit,
        body: Expr::Sequence {
            steps: vec![
                Expr::Assert {
                    condition: Box::new(Expr::Boolean { value: true }),
                    message: "first".into(),
                },
                Expr::If {
                    condition: Box::new(Expr::Boolean { value: true }),
                    then: Box::new(Expr::Assert {
                        condition: Box::new(Expr::Boolean { value: true }),
                        message: "second".into(),
                    }),
                    otherwise: Box::new(Expr::Unit),
                },
            ],
            value: Box::new(Expr::Unit),
        },
    };
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: true,
            name: "write".into(),
            parameters: vec![],
            actions: vec![StateAction::CellWrite {
                field: "written".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "1".into() },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "caller".into(),
            parameters: parameters.clone(),
            actions: vec![
                StateAction::PureCall {
                    name: "guard".into(),
                    arguments: parameters
                        .iter()
                        .map(|parameter| Expr::Coerce {
                            value: Box::new(Expr::Parameter {
                                name: parameter.name.clone(),
                            }),
                            ty: parameter.ty.clone(),
                        })
                        .collect(),
                },
                StateAction::CircuitCall {
                    name: "write".into(),
                    arguments: vec![],
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
    ];
    let recorded = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .find(|circuit| circuit.name == "caller")
            .unwrap()
            .recorded
    };
    assert!(recorded(&contract));

    let closed = contract.clone();
    contract.stateful_circuits[1]
        .actions
        .push(StateAction::CellWrite {
            field: "written".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "2".into() },
        });
    assert!(!recorded(&contract));
    contract = closed.clone();
    if let StateAction::PureCall { arguments, .. } = &mut contract.stateful_circuits[1].actions[0] {
        arguments[0] = Expr::Default {
            ty: parameters[0].ty.clone(),
        };
    }
    assert!(!recorded(&contract));
    contract = closed.clone();
    if let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body {
        steps.push(Expr::Unit);
    }
    assert!(!recorded(&contract));
    contract = closed.clone();
    contract.stateful_circuits[0].parameters.push(Parameter {
        name: "extra".into(),
        ty: Type::Field,
    });
    if let StateAction::CircuitCall { arguments, .. } =
        &mut contract.stateful_circuits[1].actions[1]
    {
        arguments.push(Expr::FieldLiteral { value: "3".into() });
    }
    assert!(!recorded(&contract));

    // The same guard may precede precisely one compiler-emitted Uint<16>
    // literal-one Let/Counter increment. Different amounts and extra actions
    // cannot be admitted by this shortcut.
    contract = closed;
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "accepted".into(),
        index: 1,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    });
    contract.stateful_circuits[1].actions[1] = StateAction::Let {
        bindings: vec![LocalBinding {
            name: "one".into(),
            ty: Type::Unsigned {
                max: "65535".into(),
            },
            value: Expr::UnsignedLiteral {
                value: "1".into(),
                max: "65535".into(),
            },
        }],
        action: Box::new(StateAction::CounterIncrement {
            field: "accepted".into(),
            index: 1,
            amount: CounterAmount::Parameter { name: "one".into() },
        }),
    };
    let counter = contract.clone();
    assert!(recorded(&counter));
    if let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[1].actions[1] {
        bindings[0].value = Expr::UnsignedLiteral {
            value: "2".into(),
            max: "65535".into(),
        };
    }
    assert!(!recorded(&contract));
    contract = counter.clone();
    contract.stateful_circuits[1]
        .actions
        .push(StateAction::CounterReset {
            field: "accepted".into(),
            index: 1,
        });
    assert!(!recorded(&contract));
    contract = counter;
    if let StateAction::PureCall { arguments, .. } = &mut contract.stateful_circuits[1].actions[0] {
        arguments[1] = Expr::Default {
            ty: parameters[1].ty.clone(),
        };
    }
    assert!(!recorded(&contract));
}

#[test]
fn conditional_assertion_folds_only_closed_same_type_unsigned_equality() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "fieldCell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "checked_write".into(),
        parameters: vec![Parameter {
            name: "choose".into(),
            ty: Type::Boolean,
        }],
        actions: vec![
            StateAction::Assert {
                condition: Expr::If {
                    condition: Box::new(Expr::Parameter {
                        name: "choose".into(),
                    }),
                    then: Box::new(Expr::Equal {
                        left: Box::new(Expr::UnsignedLiteral {
                            value: "1".into(),
                            max: "2".into(),
                        }),
                        right: Box::new(Expr::UnsignedLiteral {
                            value: "2".into(),
                            max: "2".into(),
                        }),
                    }),
                    otherwise: Box::new(Expr::Boolean { value: true }),
                },
                message: "closed equality denied".into(),
            },
            StateAction::CellWrite {
                field: "fieldCell".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "1".into() },
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_conditional_bool_0: bool = !(__compact_param_0)")
    );
    assert!(rendered.source.contains("closed equality denied"));
    assert!(rendered.source.contains("record_write(frame"));

    // An observation inside an arm cannot be evaluated ahead of the If.
    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::If { then, .. } = condition else {
        unreachable!()
    };
    **then = Expr::Equal {
        left: Box::new(Expr::CellRead {
            field: "fieldCell".into(),
            index: 0,
        }),
        right: Box::new(Expr::FieldLiteral { value: "1".into() }),
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert!(!report.circuits[0].recorded);
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.ir_node, "StateAction::Assert");
    assert_eq!(gap.path, "actions[0]");
}

#[test]
fn plain_merkle_reset_recording_requires_exact_plain_slot() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "reset".into(),
        parameters: vec![],
        actions: vec![StateAction::MerkleResetToDefault {
            field: "tree".into(),
            index: 0,
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_reset_to_default(frame)"));
    contract.stateful_circuits[0].actions[0] = StateAction::MerkleResetToDefault {
        field: "tree".into(),
        index: 1,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].actions[0] = StateAction::MerkleResetToDefault {
        field: "tree".into(),
        index: 0,
    };
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].actions[0] = StateAction::HistoricMerkleResetToDefault {
        field: "tree".into(),
        index: 0,
    };
    assert!(
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn identity_vector_map_recording_requires_closed_literals_and_exact_shape() {
    let element = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let vector = Type::Vector {
        element: Box::new(element.clone()),
        length: 3,
    };
    let literal = || Expr::UnsignedLiteral {
        value: "0".into(),
        max: u64::MAX.to_string(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "values".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: vector.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "write_map".into(),
        parameters: vec![],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "mapped".into(),
                ty: vector,
                value: Expr::VectorMap {
                    parameter: Parameter {
                        name: "item".into(),
                        ty: element.clone(),
                    },
                    source: Box::new(Expr::Tuple {
                        elements: vec![literal(), literal(), literal()],
                    }),
                    body: Box::new(Expr::Parameter {
                        name: "item".into(),
                    }),
                    result: element,
                    length: 3,
                },
            }],
            action: Box::new(StateAction::CellWrite {
                field: "values".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "mapped".into(),
                },
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let mut nonidentity = contract.clone();
    let StateAction::Let { bindings, .. } = &mut nonidentity.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::VectorMap { body, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **body = literal();
    assert!(
        !render_with_capabilities(&nonidentity)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::VectorMap { length, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    *length = 2;
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn persistent_hash_helper_recording_preserves_formals_and_rejects_effects() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("inline-type-scope-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    for circuit in &rendered.capabilities.circuits {
        assert!(
            circuit.recorded && circuit.observed_call,
            "{}",
            circuit.name
        );
    }
    let mut effectful = contract.clone();
    effectful
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "scalarHelper")
        .unwrap()
        .actions
        .push(StateAction::CellWrite {
            field: "hashCell".into(),
            index: 1,
            value: Expr::BytesLiteral { bytes: vec![0; 32] },
        });
    let rendered = render_with_capabilities(&effectful).unwrap();
    assert!(
        !rendered
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == "checkScalarScope")
            .unwrap()
            .recorded
    );
    let mut wrong_width = contract.clone();
    wrong_width
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "aggHelper")
        .unwrap()
        .parameters[0]
        .ty = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    assert!(render_with_capabilities(&wrong_width).is_err());
    let mut wrong_index = contract;
    let helper = wrong_index
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "scalarHelper")
        .unwrap();
    let StateReturn::Expression {
        value: Expr::Equal { right, .. },
    } = &mut helper.return_value
    else {
        unreachable!()
    };
    let Expr::CellRead { index, .. } = right.as_mut() else {
        unreachable!()
    };
    *index = 0;
    assert!(render_with_capabilities(&wrong_index).is_err());
}

#[test]
fn typed_opaque_struct_map_write_requires_scoped_value_and_ordered_mutation() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("asset-custody-grant-write-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "setCustodyGrant")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn setCustodyGrant<Private"));
    assert!(source.contains("crate::ledger_slots::custodyGrants\n                .record_insert("));

    fn actions(contract: &mut Contract) -> &mut Vec<StateAction> {
        let circuit = contract
            .stateful_circuits
            .iter_mut()
            .find(|circuit| circuit.name == "setCustodyGrant")
            .unwrap();
        let [StateAction::Let { action, .. }] = circuit.actions.as_mut_slice() else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            unreachable!()
        };
        actions
    }
    let mut unscoped_value = contract.clone();
    let StateAction::MapInsert { value, .. } = &mut actions(&mut unscoped_value)[3] else {
        unreachable!()
    };
    *value = Expr::Parameter {
        name: "grant".into(),
    };
    assert!(
        !recorded(&unscoped_value).0,
        "insert must use the bound struct value"
    );

    let mut wrong_key = contract.clone();
    let StateAction::If { then, .. } = &mut actions(&mut wrong_key)[2] else {
        unreachable!()
    };
    let StateAction::Sequence { actions: update } = then.as_mut() else {
        unreachable!()
    };
    let StateAction::MapRemove { key, .. } = &mut update[1] else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "grantId".into(),
    };
    assert!(!recorded(&wrong_key).0, "update must remove the bound key");

    let mut reordered = contract;
    actions(&mut reordered).swap(3, 4);
    assert!(
        !recorded(&reordered).0,
        "Map insert must precede write continuation"
    );
}

#[test]
fn authorized_optional_cell_write_is_structural_and_fails_closed() {
    let source = include_str!("election-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .find(|circuit| circuit.name == "set_topic")
            .unwrap()
            .recorded
    };
    assert!(available(&contract));
    // No circuit, witness, formal, struct, field or enum spelling is an allowlist.
    let renamed = source
        .replace("set_topic", "configure")
        .replace("private$secret_key", "load_credential")
        .replace("public_key", "derive_key")
        .replace("authority", "controller")
        .replace("PublicState", "Phase")
        .replace("setup", "draft")
        .replace("Maybe", "OptionalText")
        .replace("is_some", "present")
        .replace("\"sk\"", "\"credential\"")
        .replace("\"apk\"", "\"digest\"");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(
        render_with_capabilities(&renamed)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == "configure")
            .unwrap()
            .recorded
    );
    let mut wrong_hash = contract.clone();
    let hash = wrong_hash
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    let Expr::PersistentHash { value } = &mut hash.body else {
        unreachable!()
    };
    let Expr::Tuple { elements } = value.as_mut() else {
        unreachable!()
    };
    elements[1] = Expr::BytesLiteral { bytes: vec![0; 32] };
    assert!(!available(&wrong_hash));
    let actions = |contract: &mut Contract| {
        let circuit = contract
            .stateful_circuits
            .iter_mut()
            .find(|circuit| circuit.name == "set_topic")
            .unwrap();
        let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            unreachable!()
        };
        actions.swap(0, 1);
    };
    let mut reordered = contract.clone();
    actions(&mut reordered);
    assert!(!available(&reordered));
    let mut wrong_index = contract;
    let field = wrong_index
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "authority")
        .unwrap();
    field.index = 8;
    assert!(render_with_capabilities(&wrong_index).is_err());
}

#[test]
fn typed_asset_map_write_requires_class_guard_and_insert_only_count() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("asset-record-write-schema13-ir.json")).unwrap();
    let recorded = |value: &serde_json::Value| {
        let mut contract: Contract = serde_json::from_value(value.clone()).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        let Ok(rendered) = render_with_capabilities(&contract) else {
            return (false, String::new());
        };
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "setRecord")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&original);
    assert!(available);
    assert!(source.contains("pub fn setRecord<Private"));
    assert!(source.contains("record_increment(frame, 1_u16)"));
    assert!(source.contains("assertRecordClassKnown("));
    assert!(source.contains("__compact_recorded_write_value.clone()"));

    // The checked guard follows the declared enum/member, not the oracle's
    // spellings for either one. Mutation labels remain a bounded domain.
    fn rename_class(value: &mut serde_json::Value) {
        if let Some(object) = value.as_object_mut() {
            let kind = object
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let name = object
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            if kind.as_deref() == Some("enum") && name.as_deref() == Some("AssetClass") {
                object.get_mut("variants").unwrap()[0] = serde_json::json!("Pending");
            }
            if kind.as_deref() == Some("enum_variant")
                && object.get("ty").and_then(|ty| ty.get("name"))
                    == Some(&serde_json::json!("AssetClass"))
            {
                object.insert("variant".into(), serde_json::json!("Pending"));
            }
            if kind.as_deref() == Some("struct") && name.as_deref() == Some("AssetRecord") {
                for field in object.get_mut("fields").unwrap().as_array_mut().unwrap() {
                    if field["name"] == "kind" {
                        field["name"] = serde_json::json!("category");
                    }
                }
            }
            if kind.as_deref() == Some("struct_field")
                && object.get("field") == Some(&serde_json::json!("kind"))
            {
                object.insert("field".into(), serde_json::json!("category"));
            }
            for child in object.values_mut() {
                rename_class(child);
            }
        } else if let Some(items) = value.as_array_mut() {
            for child in items {
                rename_class(child);
            }
        }
    }
    let mut renamed = original.clone();
    rename_class(&mut renamed);
    assert!(recorded(&renamed).0);

    fn actions(contract: &mut serde_json::Value) -> &mut Vec<serde_json::Value> {
        contract["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|circuit| circuit["name"] == "setRecord")
            .unwrap()["actions"][0]["action"]["action"]["action"]["actions"]
            .as_array_mut()
            .unwrap()
    }
    let mut wrong_class_argument = original.clone();
    actions(&mut wrong_class_argument)[2]["arguments"][0]["value"]["name"] =
        serde_json::json!("record");
    assert!(!recorded(&wrong_class_argument).0);

    let mut reordered = original.clone();
    actions(&mut reordered).swap(2, 3);
    assert!(!recorded(&reordered).0);

    let mut extra_class_step = original.clone();
    extra_class_step["circuits"][0]["body"]["steps"]
        .as_array_mut()
        .unwrap()
        .push(original["circuits"][0]["body"]["steps"][0].clone());
    assert!(!recorded(&extra_class_step).0);

    let mut wrong_counter_amount = original.clone();
    actions(&mut wrong_counter_amount)[3]["otherwise"]["then"]["actions"][1]["bindings"][0]["value"]
        ["value"] = serde_json::json!("2");
    assert!(!recorded(&wrong_counter_amount).0);

    let mut wrong_counter_slot = original.clone();
    actions(&mut wrong_counter_slot)[3]["otherwise"]["then"]["actions"][1]["action"]["field"] =
        serde_json::json!("open");
    assert!(!recorded(&wrong_counter_slot).0);

    let mut wrong_exists_map = original.clone();
    let helper = wrong_exists_map["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|circuit| circuit["name"] == "recordExists")
        .unwrap();
    helper["return_value"]["value"]["otherwise"]["field"] = serde_json::json!("records");
    assert!(!recorded(&wrong_exists_map).0);
}

#[test]
fn authorized_enum_advance_requires_closed_successor_and_ordered_optional_read() {
    let source = include_str!("election-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract, name: &str| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == name)
            .unwrap()
            .recorded
    };
    assert!(available(&contract, "advance"));
    let renamed = source
        .replace("advance", "progress")
        .replace("successor", "next_phase")
        .replace("private$secret_key", "load_credential")
        .replace("public_key", "derive_key")
        .replace("authority", "controller")
        .replace("topic", "subject")
        .replace("PublicState", "Phase")
        .replace("setup", "draft")
        .replace("Maybe", "OptionalText")
        .replace("is_some", "present")
        .replace("\"sk\"", "\"credential\"")
        .replace("\"apk\"", "\"digest\"");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "progress"));
    let mut guarded_successor = contract.clone();
    let helper = guarded_successor
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "successor")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    assert!(!available(&guarded_successor, "advance"));
    let mut reordered = contract.clone();
    let circuit = reordered
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "advance")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Let { action, .. } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    actions.swap(0, 1);
    assert!(!available(&reordered, "advance"));
    let mut wrong_index = contract;
    let field = wrong_index
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "topic")
        .unwrap();
    field.index = 8;
    assert!(render_with_capabilities(&wrong_index).is_err());
}

#[test]
fn witness_admitted_merkle_insert_requires_path_shape_argument_and_order() {
    let source = include_str!("election-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract, name: &str| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == name)
            .unwrap()
            .recorded
    };
    assert!(available(&contract, "add_voter"));
    let renamed = source
        .replace("add_voter", "admit_member")
        .replace("eligible_voters", "registry")
        .replace("private$secret_key", "load_credential")
        .replace("public_key", "derive_key")
        .replace("authority", "controller")
        .replace("PublicState", "Phase")
        .replace("setup", "draft")
        .replace("Maybe", "OptionalValue")
        .replace("is_some", "present");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "admit_member"));
    let mut wrong_argument = contract.clone();
    let circuit = wrong_argument
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "add_voter")
        .unwrap();
    let StateAction::Sequence { actions } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Assert {
        condition: Expr::If { condition, .. },
        ..
    } = &mut actions[0]
    else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = condition.as_mut() else {
        unreachable!()
    };
    let Expr::WitnessCall { arguments, .. } = value.as_mut() else {
        unreachable!()
    };
    arguments[0] = Expr::Coerce {
        value: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        ty: Type::Bytes { length: 32 },
    };
    assert!(!available(&wrong_argument, "add_voter"));
    let mut reordered = contract.clone();
    let circuit = reordered
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "add_voter")
        .unwrap();
    let StateAction::Sequence { actions } = &mut circuit.actions[0] else {
        unreachable!()
    };
    actions.swap(0, 1);
    assert!(!available(&reordered, "add_voter"));
    let mut wrong_depth = contract;
    let field = wrong_depth
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "eligible_voters")
        .unwrap();
    let LedgerFieldKind::MerkleTree { depth, .. } = &mut field.declaration else {
        unreachable!()
    };
    *depth = 9;
    // Other source path consumers may reject the contract before recording.
    if let Ok(rendered) = render_with_capabilities(&wrong_depth) {
        assert!(
            !rendered
                .capabilities
                .circuits
                .iter()
                .find(|circuit| circuit.name == "add_voter")
                .unwrap()
                .recorded
        );
    }
}

#[test]
fn composite_witness_and_pure_struct_hash_recording_remains_typed_and_closed() {
    let source = include_str!("zerocash-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract, name: &str| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == name)
            .unwrap()
            .recorded
    };
    assert!(available(&contract, "zerocash_mint"));
    assert!(available(&contract, "spend"));
    let renamed = source
        .replace("zerocash_mint", "create_item")
        .replace("commitment_from_coin_info", "derive_item")
        .replace("context$new_coin_info", "load_item")
        .replace("private$add_coin", "save_item")
        .replace("coin_info", "ItemData");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "create_item"));
    // A valid native branch is deliberately outside the closed hash expression subset.
    let mut branch = contract.clone();
    let helper = branch
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "commitment_from_coin_info")
        .unwrap();
    helper.body = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(helper.body.clone()),
        otherwise: Box::new(helper.body.clone()),
    };
    assert!(!available(&branch, "zerocash_mint"));
    let mut wrong_witness = contract.clone();
    wrong_witness
        .witnesses
        .iter_mut()
        .find(|witness| witness.name == "context$new_coin_info")
        .unwrap()
        .result = Type::Field;
    assert!(render_with_capabilities(&wrong_witness).is_err());
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "commitments")
        .unwrap()
        .index = 7;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}

#[test]
fn composite_recording_does_not_erase_local_argument_widening() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("struct-helper-widening-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    };
    assert!(!available(&contract));
    // Exercise the same conversion through a struct-returning witness.
    let helper = contract
        .circuits
        .iter()
        .find(|circuit| circuit.name == "wrap")
        .unwrap()
        .clone();
    contract.witnesses.push(WitnessDeclaration {
        name: helper.name.clone(),
        parameters: helper.parameters.clone(),
        result: helper.result.clone(),
        source: helper.source.clone(),
    });
    fn replace(action: &mut StateAction) {
        match action {
            StateAction::Let { bindings, action } => {
                for binding in bindings {
                    if let Expr::Call { name, arguments } = &binding.value
                        && name == "wrap"
                    {
                        binding.value = Expr::WitnessCall {
                            name: name.clone(),
                            arguments: arguments.clone(),
                        };
                    }
                }
                replace(action);
            }
            StateAction::Sequence { actions } => {
                for action in actions {
                    replace(action);
                }
            }
            _ => {}
        }
    }
    contract.circuits.retain(|circuit| circuit.name != "wrap");
    for action in &mut contract.stateful_circuits[0].actions {
        replace(action);
    }
    assert!(!available(&contract));
}

#[test]
fn guarded_opaque_set_mutation_requires_two_maps_and_opposite_typed_branches() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("asset-watch-write-schema13-ir.json")).unwrap();
    let recorded = |value: &serde_json::Value| {
        let mut contract: Contract = serde_json::from_value(value.clone()).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        let Ok(rendered) = render_with_capabilities(&contract) else {
            return (false, String::new());
        };
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "setWatch")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&original);
    assert!(available);
    assert!(source.contains("pub fn setWatch<Private"));
    assert!(source.contains("watchList"));
    assert!(source.contains("record_insert("));
    assert!(source.contains("record_remove("));

    fn actions(contract: &mut serde_json::Value) -> &mut Vec<serde_json::Value> {
        contract["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|circuit| circuit["name"] == "setWatch")
            .unwrap()["actions"][0]["action"]["action"]["actions"]
            .as_array_mut()
            .unwrap()
    }
    let mut wrong_key = original.clone();
    actions(&mut wrong_key)[2]["condition"]["arguments"][0]["value"]["name"] =
        serde_json::json!("recordId");
    assert!(!recorded(&wrong_key).0);

    let mut wrong_enum = original.clone();
    actions(&mut wrong_enum)[1]["condition"]["otherwise"]["right"]["variant"] =
        serde_json::json!("Add");
    assert!(!recorded(&wrong_enum).0);

    let mut reordered = original.clone();
    actions(&mut reordered).swap(2, 3);
    assert!(!recorded(&reordered).0);

    let mut wrong_set = original.clone();
    actions(&mut wrong_set)[3]["then"]["actions"][1]["field"] = serde_json::json!("retiredKeys");
    assert!(!recorded(&wrong_set).0);

    let mut wrong_drop = original.clone();
    let absent_condition = actions(&mut wrong_drop)[3]["then"]["actions"][0]["condition"].clone();
    actions(&mut wrong_drop)[3]["otherwise"]["then"]["actions"][0]["condition"] = absent_condition;
    assert!(!recorded(&wrong_drop).0);

    let mut duplicate_map = original.clone();
    let helper = duplicate_map["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|circuit| circuit["name"] == "recordExists")
        .unwrap();
    helper["return_value"]["value"]["otherwise"]["field"] = serde_json::json!("records");
    assert!(!recorded(&duplicate_map).0);
}

#[test]
fn typed_membership_plan_checks_helper_closure_and_requires_its_domain() {
    let source = include_str!("election-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract, name: &str| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == name)
            .unwrap()
            .recorded
    };
    assert!(available(&contract, "vote$commit"));
    assert!(available(&contract, "vote$reveal"));
    let renamed = source
        .replace("vote$commit", "submit_ballot")
        .replace("vote$reveal", "open_ballot")
        .replace("tally_yes", "accepted")
        .replace("tally_no", "rejected")
        .replace("private$vote$record", "store_ballot")
        .replace("commitment_nullifier", "unique_vote")
        .replace("public_key", "voter_key")
        .replace("ballot_repr", "encode_vote")
        .replace("commit_with_sk", "seal_vote")
        .replace("eligible_voters", "membership")
        .replace("committed_votes", "ballots");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "submit_ballot"));
    assert!(available(&renamed, "open_ballot"));
    let mut effectful = contract.clone();
    let helper = effectful
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "ballot_repr")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra guard".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    assert!(!available(&effectful, "vote$commit"));
    assert!(!available(&effectful, "vote$reveal"));
    let mut recursive = contract.clone();
    let helper = recursive
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "merkleTreePathEntryRoot")
        .unwrap();
    helper.body = Expr::Call {
        name: helper.name.clone(),
        arguments: helper
            .parameters
            .iter()
            .map(|parameter| Expr::Parameter {
                name: parameter.name.clone(),
            })
            .collect(),
    };
    // Recursive pure helpers are refused instead of publishing capabilities.
    let expected = RenderError::Located {
        location: helper.source.clone().unwrap(),
        error: Box::new(RenderError::RecursivePureCall(helper.name.clone())),
    };
    assert_eq!(
        render_with_capabilities(&recursive).err().unwrap(),
        expected
    );
    assert_eq!(render(&recursive).unwrap_err(), expected);
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "eligible_voters")
        .unwrap()
        .index = 8;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}

#[test]
fn typed_reveal_plan_rejects_unproven_counter_mutations_and_wrong_slots() {
    fn available(value: &serde_json::Value) -> bool {
        let mut contract: Contract = serde_json::from_value(value.clone()).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "vote$reveal")
            .unwrap()
            .recorded
    }
    fn replace_kind(value: &mut serde_json::Value, from: &str, to: &str) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|v| v.as_str()) == Some(from) {
                    map.insert("kind".into(), serde_json::Value::String(to.into()));
                }
                for value in map.values_mut() {
                    replace_kind(value, from, to);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    replace_kind(value, from, to);
                }
            }
            _ => {}
        }
    }
    let source: serde_json::Value =
        serde_json::from_str(include_str!("election-schema13-ir.json")).unwrap();
    assert!(available(&source));
    let mut decrement = source.clone();
    let reveal = decrement["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "vote$reveal")
        .unwrap();
    replace_kind(
        &mut reveal["actions"],
        "counter_increment",
        "counter_decrement",
    );
    assert!(!available(&decrement));
    let mut wrong_slot: Contract = serde_json::from_value(source).unwrap();
    wrong_slot.schema_version = SCHEMA_VERSION;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "tally_yes")
        .unwrap()
        .index = 4;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}

#[test]
fn typed_cell_lifecycle_retains_only_final_root_scope_and_audits_helpers() {
    let source = include_str!("bboard-schema13-ir.json");
    let load = |source: &str| {
        let mut contract: Contract = serde_json::from_str(source).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        contract
    };
    let available = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .map(|c| c.recorded)
            .collect::<Vec<_>>()
    };
    let contract = load(source);
    assert_eq!(available(&contract), [true, true]);
    let renamed = load(
        &source
            .replace("post", "publish")
            .replace("take_down", "remove_message")
            .replace("former_msg", "saved_content")
            .replace("public_key", "derive_owner")
            .replace("instance", "generation"),
    );
    assert_eq!(available(&renamed), [true, true]);
    let mut effectful = contract.clone();
    let helper = effectful
        .circuits
        .iter_mut()
        .find(|c| c.name == "public_key")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra effect".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    assert_eq!(available(&effectful), [false, false]);
    let mut nested = contract.clone();
    let action = nested.stateful_circuits[1].actions.pop().unwrap();
    nested.stateful_circuits[1]
        .actions
        .push(StateAction::Sequence {
            actions: vec![action],
        });
    // The shared continuation adapter preserves the same terminal lexical
    // scope as native lowering, without changing this profile's effects.
    assert_eq!(available(&nested), [true, true]);
    let mut earlier = contract.clone();
    earlier.stateful_circuits[1]
        .actions
        .push(StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "after scoped binding".into(),
        });
    let error = render(&earlier).unwrap_err();
    assert!(format!("{error:?}").contains("former_msg"), "{error:?}");
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "instance")
        .unwrap()
        .index = 3;
    assert!(render(&wrong_slot).is_err());
}

#[test]
fn typed_historic_spend_audits_path_helpers_and_slot_provenance() {
    let source = include_str!("zerocash-schema13-ir.json");
    let load = |source: &str| {
        let mut c: Contract = serde_json::from_str(source).unwrap();
        c.schema_version = SCHEMA_VERSION;
        c
    };
    let available = |c: &Contract, name: &str| {
        render_with_capabilities(c)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .recorded
    };
    let contract = load(source);
    assert!(available(&contract, "spend"));
    let renamed = load(
        &source
            .replace("spend", "transfer")
            .replace("ciphertexts", "envelopes")
            .replace("commitments", "notes")
            .replace("nullifiers", "used_notes")
            .replace("context$encrypt", "seal"),
    );
    assert!(available(&renamed, "transfer"));
    let mut effectful = contract.clone();
    let helper = effectful
        .circuits
        .iter_mut()
        .find(|c| c.name == "merkleTreePathEntryRoot")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra guard".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    assert!(!available(&effectful, "spend"));
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "commitments")
        .unwrap()
        .index = 2;
    assert!(render(&wrong_slot).is_err());
    let mut json: serde_json::Value = serde_json::from_str(source).unwrap();
    json["schema_version"] = serde_json::json!(SCHEMA_VERSION);
    fn replace(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|v| v.as_str()) == Some("set_insert") {
                    map.insert("kind".into(), serde_json::json!("set_remove"));
                }
                for value in map.values_mut() {
                    replace(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    replace(value);
                }
            }
            _ => {}
        }
    }
    replace(&mut json);
    let unsupported: Contract = serde_json::from_value(json).unwrap();
    assert!(!available(&unsupported, "spend"));
}

#[test]
fn audited_schnorr_local_helper_requires_transitive_public_purity_and_typed_sources() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("schnorr-attestation-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let reported = render_with_capabilities(&contract).unwrap();
    for name in ["verifyAttestation", "acceptAttestation"] {
        assert!(
            reported
                .capabilities
                .circuits
                .iter()
                .find(|c| c.name == name)
                .unwrap()
                .recorded
        );
    }
    assert!(reported.source.contains(".call_local(|context|"));

    let mut effectful = contract.clone();
    let helper = effectful
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerify")
        .unwrap();
    helper.actions.push(StateAction::CounterIncrement {
        field: "acceptedCount".into(),
        index: 1,
        amount: CounterAmount::Literal { value: 1 },
    });
    assert!(
        !render_with_capabilities(&effectful)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "verifyAttestation")
            .unwrap()
            .recorded
    );

    let mut read_opening = contract.clone();
    read_opening.ledger_fields.push(LedgerField {
        source: None,
        id: "hidden".into(),
        index: 3,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    });
    let helper = read_opening
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerify")
        .unwrap();
    helper.actions.push(StateAction::Expression {
        value: Expr::TransientCommit {
            value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            opening: Box::new(Expr::CellRead {
                field: "hidden".into(),
                index: 3,
            }),
        },
    });
    assert!(
        !render_with_capabilities(&read_opening)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "verifyAttestation")
            .unwrap()
            .recorded
    );

    let mut wrong_key = contract.clone();
    let key = wrong_key
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "attestorKey")
        .unwrap();
    key.declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert!(render_with_capabilities(&wrong_key).is_err());

    let mut missing_guard = contract.clone();
    missing_guard
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "verifyAttestation")
        .unwrap()
        .actions
        .remove(0);
    // The legacy audited-local profile still refuses this shape (its module test).
    // Unit composition admits the actual key read and audited local verifier,
    // without inventing the removed source guard.
    assert!(
        render_with_capabilities(&missing_guard)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "verifyAttestation")
            .unwrap()
            .recorded
    );

    let mut unknown = contract.clone();
    let helper = unknown
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerifyDigest")
        .unwrap();
    let StateAction::CircuitCall { name, .. } = &mut helper.actions[0] else {
        unreachable!()
    };
    *name = "unknownHelper".into();
    assert!(render_with_capabilities(&unknown).is_err());

    let mut recursive = contract.clone();
    recursive
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerify")
        .unwrap()
        .actions
        .push(StateAction::CircuitCall {
            name: "schnorrVerifyDigest".into(),
            arguments: vec![
                Expr::Parameter { name: "msg".into() },
                Expr::Parameter {
                    name: "signature".into(),
                },
                Expr::Parameter { name: "pk".into() },
            ],
        });
    assert!(render_with_capabilities(&recursive).is_err());
}

#[test]
fn counter_less_than_requires_typed_counter_slots_and_thresholds() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("counter-less-than-schema15-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 4);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|circuit| circuit.recorded && circuit.observed_call)
    );
    assert!(rendered.source.contains(".record_less_than("));
    let first = contract
        .stateful_circuits
        .iter()
        .position(|circuit| circuit.name == "compare")
        .unwrap();
    let mutate = |value: Expr| {
        let mut wrong = contract.clone();
        wrong.stateful_circuits[first].return_value = StateReturn::Expression { value };
        assert!(render_with_capabilities(&wrong).is_err());
    };
    mutate(Expr::CounterLessThan {
        field: "low".into(),
        index: 0,
        threshold: Box::new(Expr::Boolean { value: false }),
    });
    mutate(Expr::CounterLessThan {
        field: "low".into(),
        index: 1,
        threshold: Box::new(Expr::Parameter {
            name: "threshold".into(),
        }),
    });
    mutate(Expr::CounterLessThan {
        field: "missing".into(),
        index: 0,
        threshold: Box::new(Expr::Parameter {
            name: "threshold".into(),
        }),
    });
    let mut wrong = contract.clone();
    wrong.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert!(render_with_capabilities(&wrong).is_err());
    let mut old_schema = contract.clone();
    old_schema.schema_version = 14;
    assert!(render_with_capabilities(&old_schema).is_err());
    let mut pure = contract.clone();
    pure.circuits.push(PureCircuit {
        name: "invalid_pure".into(),
        source: None,
        internal: false,
        parameters: vec![],
        result: Type::Boolean,
        body: Expr::CounterLessThan {
            field: "low".into(),
            index: 0,
            threshold: Box::new(Expr::UnsignedLiteral {
                value: "4".into(),
                max: u64::MAX.to_string(),
            }),
        },
    });
    assert!(render_with_capabilities(&pure).is_err());
}

#[test]
fn qualified_cell_coin_write_is_typed_and_recordable() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("qualified-coin-cell-schema16-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    let write = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "write_coin")
        .unwrap();
    assert!(write.recorded && write.observed_call);
    assert!(write.recording_unavailable.is_none());
    assert!(rendered.source.contains(".write_coin("));
    assert!(rendered.source.contains(".record_write_coin("));
    for mode in 0..4 {
        let mut wrong = contract.clone();
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "write_coin")
            .unwrap();
        let StateAction::CellWriteCoin {
            index,
            coin,
            recipient,
            ..
        } = &mut circuit.actions[0]
        else {
            panic!("expected coin write")
        };
        match mode {
            0 => *index = 1,
            1 => *coin = Expr::Boolean { value: false },
            2 => *recipient = Expr::Boolean { value: false },
            _ => wrong.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean },
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "invalid qualified Cell mode {mode}"
        );
    }
    let mut wrong_field = contract.clone();
    let StateAction::CellWriteCoin { field, .. } = &mut wrong_field
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "write_coin")
        .unwrap()
        .actions[0]
    else {
        unreachable!()
    };
    *field = "missing".into();
    assert!(render_with_capabilities(&wrong_field).is_err());
    let mut escaped = contract.clone();
    let StateAction::CellWriteCoin { coin, .. } = &mut escaped
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "write_coin")
        .unwrap()
        .actions[0]
    else {
        unreachable!()
    };
    *coin = Expr::Parameter {
        name: "escaped".into(),
    };
    assert!(render_with_capabilities(&escaped).is_err());
    let mut nested = contract;
    nested.ledger_fields[0].path = vec![1];
    assert!(render_with_capabilities(&nested).is_err());
}

#[test]
fn native_zswap_intents_validate_types_and_record_bounded_unit_helpers() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("native-zswap-intents-schema18-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    for name in ["produce", "consume", "flow", "witness_order", "read_coin"] {
        let circuit = rendered
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        let proof_required = matches!(name, "flow" | "read_coin");
        assert_eq!(circuit.recorded, proof_required, "{name}");
        assert_eq!(circuit.observed_call, proof_required, "{name}");
    }
    assert!(rendered.source.contains(".create_zswap_output("));
    assert!(rendered.source.contains(".create_zswap_input("));
    assert!(!rendered.source.contains(".call_local("));
    // Removing the only public query leaves input/output intents, including
    // the inlined pair helper, but cannot make a proof API by itself.
    let mut intent_only = contract.clone();
    let flow = intent_only
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "flow")
        .unwrap();
    let StateAction::If { then, .. } = &mut flow.actions[1] else {
        panic!("expected branch")
    };
    let StateAction::Sequence { actions } = then.as_mut() else {
        panic!("expected sequence")
    };
    actions.retain(|action| !matches!(action, StateAction::CellWriteCoin { .. }));
    let native_only = render_with_capabilities(&intent_only).unwrap();
    let flow = native_only
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "flow")
        .unwrap();
    assert!(!flow.recorded && !flow.observed_call);
    for mode in 0..3 {
        let mut wrong = contract.clone();
        let name = if mode == 0 { "consume" } else { "produce" };
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == name)
            .unwrap();
        let StateReturn::Expression { value } = &mut circuit.return_value else {
            panic!("expected native return");
        };
        match value {
            Expr::CreateZswapInput { coin } => **coin = Expr::Boolean { value: false },
            Expr::CreateZswapOutput { coin, recipient } => {
                if mode == 1 {
                    **coin = Expr::Boolean { value: false }
                } else {
                    **recipient = Expr::Boolean { value: false }
                }
            }
            _ => panic!("expected native expression"),
        }
        assert!(render_with_capabilities(&wrong).is_err());
    }
    let mut pure = contract.clone();
    let StateReturn::Expression { value } = pure
        .stateful_circuits
        .iter()
        .find(|c| c.name == "consume")
        .unwrap()
        .return_value
        .clone()
    else {
        panic!()
    };
    pure.circuits.push(PureCircuit {
        name: "invalid_pure".into(),
        source: None,
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        body: value,
    });
    assert!(render_with_capabilities(&pure).is_err());
    for bad in [
        r#"{"kind":"create_zswap_input"}"#,
        r#"{"kind":"create_zswap_output","coin":{"kind":"unit"}}"#,
        r#"{"kind":"create_zswap_input","coin":{"kind":"unit"},"arguments":[]}"#,
    ] {
        assert!(serde_json::from_str::<Expr>(bad).is_err());
    }
}

#[test]
fn kernel_effects_have_typed_arguments_and_no_fictional_ledger_slots() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("kernel-shielded-effects-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    assert_eq!(contract.ledger_fields.len(), 1);
    assert_eq!(contract.ledger_fields[0].id, "marker");
    let rendered = render_with_capabilities(&contract).unwrap();
    for c in &rendered.capabilities.circuits {
        assert!(c.recorded, "{}", c.name);
        assert!(c.observed_call, "{}", c.name);
    }
    assert!(rendered.source.contains(".kernel_mint_shielded("));
    assert!(rendered.source.contains(".kernel_claim_zswap_nullifier("));
    assert!(!rendered.source.contains(".call_local("));
    for mode in 0..4 {
        let mut wrong = contract.clone();
        let name = if mode == 0 { "spend" } else { "mint" };
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == name)
            .unwrap();
        let StateReturn::Expression { value } = &mut circuit.return_value else {
            panic!("expected Kernel return");
        };
        match value {
            Expr::KernelClaim { value, .. } => **value = Expr::Boolean { value: false },
            Expr::KernelMintShielded { domain, amount } => match mode {
                1 => **domain = Expr::Boolean { value: false },
                2 => **amount = Expr::Boolean { value: false },
                _ => {
                    **amount = Expr::UnsignedLiteral {
                        value: "1".into(),
                        max: u128::MAX.to_string(),
                    }
                }
            },
            _ => panic!("expected typed Kernel expression"),
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "invalid Kernel mode {mode}"
        );
    }
    let mut pure = contract.clone();
    let circuit = pure
        .stateful_circuits
        .iter()
        .find(|c| c.name == "mint")
        .unwrap()
        .clone();
    let StateReturn::Expression { value } = circuit.return_value else {
        panic!()
    };
    pure.circuits.push(PureCircuit {
        name: "invalid_pure".into(),
        source: None,
        internal: false,
        parameters: circuit.parameters,
        result: Type::Unit,
        body: value,
    });
    assert!(render_with_capabilities(&pure).is_err());
    for bad in [
        r#"{"kind":"kernel_claim","claim":"coin_spend","value":{"kind":"unit"},"path":[0]}"#,
        r#"{"kind":"kernel_claim","claim":"invented","value":{"kind":"unit"}}"#,
        r#"{"kind":"kernel_mint_shielded","domain":{"kind":"unit"}}"#,
    ] {
        assert!(serde_json::from_str::<Expr>(bad).is_err());
    }
}

#[test]
fn kernel_recording_rejects_public_slot_composition_and_escaped_scopes() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("kernel-shielded-effects-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let mut combined = contract.clone();
    let mint = combined
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "mint")
        .unwrap();
    mint.actions.push(StateAction::CellWrite {
        field: "marker".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "1".into() },
    });
    let rendered = render_with_capabilities(&combined).unwrap();
    let mint = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "mint")
        .unwrap();
    assert!(!mint.recorded && !mint.observed_call);

    let mut escaped = contract.clone();
    let circuit = escaped
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "witness_order")
        .unwrap();
    circuit.actions.push(StateAction::Expression {
        value: Expr::KernelClaim {
            claim: compact_rust_backend::ir::KernelClaimKind::CoinSpend,
            value: Box::new(Expr::Parameter {
                name: "tmp_1".into(),
            }),
        },
    });
    assert!(
        render_with_capabilities(&escaped).is_err(),
        "nested Let binding escaped its Kernel action"
    );

    let mut invalid_branch = contract.clone();
    let circuit = invalid_branch
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "selected")
        .unwrap();
    let StateAction::If { condition, .. } = &mut circuit.actions[0] else {
        panic!()
    };
    *condition = Expr::Parameter {
        name: "value".into(),
    };
    assert!(
        render_with_capabilities(&invalid_branch).is_err(),
        "Bytes32 used as Boolean branch condition"
    );

    let mut invalid_witness = contract.clone();
    invalid_witness
        .witnesses
        .iter_mut()
        .find(|w| w.name == "next_amount")
        .unwrap()
        .result = Type::Field;
    assert!(
        render_with_capabilities(&invalid_witness).is_err(),
        "Field witness used as Uint64 mint amount"
    );
}

#[test]
fn stateful_structs_validate_member_types_and_record_zswap_unit_members() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("stateful-struct-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("__compact_struct_member_"));
    assert_eq!(rendered.capabilities.circuits.len(), 4);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    for mode in 0..4 {
        let mut wrong = contract.clone();
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "reverse")
            .unwrap();
        let StateReturn::Expression {
            value: Expr::StructLiteral { ty, fields },
        } = &mut circuit.return_value
        else {
            panic!("expected literal")
        };
        match mode {
            0 => *ty = Type::Boolean,
            1 => {
                fields.pop();
            }
            2 => fields[0] = Expr::Boolean { value: false },
            3 => {
                let Expr::UnsignedCast { value, .. } = &fields[0] else {
                    panic!("expected explicit widening")
                };
                fields[0] = *value.clone();
            }
            _ => unreachable!(),
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "malformed literal mode {mode}"
        );
    }
    let mut pure = contract.clone();
    let circuit = pure
        .stateful_circuits
        .iter()
        .find(|c| c.name == "planned")
        .unwrap()
        .clone();
    let StateReturn::Expression { value } = circuit.return_value else {
        panic!()
    };
    pure.circuits.push(PureCircuit {
        name: "invalid_pure_struct".into(),
        source: None,
        internal: false,
        parameters: circuit.parameters,
        result: circuit.result,
        body: value,
    });
    assert!(render_with_capabilities(&pure).is_err());
}

#[test]
fn wide_addition_validates_every_bound_and_does_not_admit_other_wide_operators() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("wide-add-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let out = render_with_capabilities(&contract).unwrap();
    assert!(out.source.contains("runtime::add_wide_unsigned"));
    assert!(out.source.contains("runtime::narrow_wide_uint"));
    for mode in 0..6 {
        let mut wrong = contract.clone();
        let c = wrong
            .circuits
            .iter_mut()
            .find(|c| c.name == "add128")
            .unwrap();
        let Expr::UnsignedCast { value, .. } = &mut c.body else {
            panic!()
        };
        let Expr::UnsignedAdd { max, left, right } = value.as_mut() else {
            panic!()
        };
        match mode {
            0 => *max = "0680564733841876926926749214863536422910".into(),
            1 => {
                *max = "452312848583266388373324160190187140051835877600158453279131187530910662656"
                    .into()
            }
            2 => **left = Expr::Boolean { value: false },
            3 => {
                let Expr::UnsignedCast { max, .. } = left.as_mut() else {
                    panic!()
                };
                *max = "-1".into();
            }
            4 => {
                **value = Expr::UnsignedSubtract {
                    max: max.clone(),
                    left: left.clone(),
                    right: right.clone(),
                }
            }
            5 => {
                **value = Expr::UnsignedMultiply {
                    max: max.clone(),
                    left: left.clone(),
                    right: right.clone(),
                }
            }
            _ => unreachable!(),
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "invalid wide arithmetic mode {mode}"
        );
    }
    let mut wrong = contract.clone();
    let c = wrong
        .circuits
        .iter_mut()
        .find(|c| c.name == "mixed")
        .unwrap();
    c.result = Type::Boolean;
    c.body = Expr::Compare {
        operator: compact_rust_backend::ir::ComparisonOperator::Less,
        left: Box::new(Expr::Parameter { name: "a".into() }),
        right: Box::new(Expr::Parameter { name: "b".into() }),
    };
    assert!(render_with_capabilities(&wrong).is_err());
    // Stateful arithmetic shares the same admission checks.
    for kind in ["unsigned_subtract", "unsigned_multiply", "compare"] {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("wide-add-schema20-ir.json")).unwrap();
        fn replace(value: &mut serde_json::Value, kind: &str) {
            if let Some(obj) = value.as_object_mut() {
                if obj.get("kind").and_then(|v| v.as_str()) == Some("unsigned_add") {
                    obj.insert("kind".into(), kind.into());
                    if kind == "compare" {
                        obj.remove("max");
                        obj.insert("operator".into(), "less".into());
                    }
                    return;
                }
                for v in obj.values_mut() {
                    replace(v, kind);
                }
            } else if let Some(values) = value.as_array_mut() {
                for v in values {
                    replace(v, kind);
                }
            }
        }
        replace(&mut value["stateful_circuits"], kind);
        let mut wrong: Contract = serde_json::from_value(value).unwrap();
        wrong.schema_version = SCHEMA_VERSION;
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "stateful wide {kind}"
        );
    }
}

#[test]
fn stateful_assertions_require_boolean_and_record_typed_read_only_results() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("stateful-assert-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 2);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("stateful-assert-schema20-ir.json")).unwrap();
    fn invalidate(value: &mut serde_json::Value) {
        if let Some(object) = value.as_object_mut() {
            if object.get("kind").and_then(|v| v.as_str()) == Some("assert") {
                object.insert("condition".into(), serde_json::json!({"kind":"unit"}));
            } else {
                for value in object.values_mut() {
                    invalidate(value);
                }
            }
        } else if let Some(values) = value.as_array_mut() {
            for value in values {
                invalidate(value);
            }
        }
    }
    invalidate(&mut value["stateful_circuits"]);
    let mut wrong: Contract = serde_json::from_value(value).unwrap();
    wrong.schema_version = SCHEMA_VERSION;
    assert!(render_with_capabilities(&wrong).is_err());
    let mut wrong = contract.clone();
    let c = wrong
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "checked")
        .unwrap();
    c.return_value = StateReturn::Expression {
        value: Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "typed unit".into(),
        },
    };
    assert!(render_with_capabilities(&wrong).is_err());
    // The pure renderer must still reject ledger reads in assertions.
    let mut wrong = contract;
    wrong.circuits.push(PureCircuit {
        name: "invalid_pure_assert".into(),
        source: None,
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        body: Expr::Assert {
            condition: Box::new(Expr::CellRead {
                field: "open".into(),
                index: 0,
            }),
            message: "pure cannot query".into(),
        },
    });
    assert!(render_with_capabilities(&wrong).is_err());
}

#[test]
fn assertion_recording_rejects_nonunit_steps_wrong_slots_and_extra_effects() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("stateful-assert-schema20-ir.json")).unwrap();
    let parse = |value| serde_json::from_value::<Contract>(value).unwrap();
    let mut nonunit = source.clone();
    nonunit["stateful_circuits"][0]["return_value"]["value"]["body"]["steps"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"boolean", "value":true}));
    let result = render_with_capabilities(&parse(nonunit)).unwrap();
    assert!(!result.capabilities.circuits[0].recorded);
    assert!(result.capabilities.circuits[1].recorded);
    let mut extra = parse(source.clone());
    extra.stateful_circuits[0]
        .actions
        .push(StateAction::CounterReset {
            field: "low".into(),
            index: 1,
        });
    assert!(
        !render_with_capabilities(&extra)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    let mut mismatch = source.clone();
    mismatch["ledger_fields"][0]["declaration"]["ty"] = serde_json::json!({"kind":"field"});
    assert!(render(&parse(mismatch)).is_err());
    let mut counter_kind = source.clone();
    counter_kind["ledger_fields"][2]["declaration"] =
        serde_json::json!({"kind":"cell", "ty":{"kind":"boolean"}});
    assert!(render(&parse(counter_kind)).is_err());
    let mut wrong_arity = source.clone();
    wrong_arity["witnesses"][0]["parameters"] = serde_json::json!([]);
    assert!(render(&parse(wrong_arity)).is_err());
    let mut escaped = source.clone();
    escaped["stateful_circuits"][0]["return_value"]["value"]["body"]["value"] =
        serde_json::json!({"kind":"parameter", "name":"tmp_16"});
    assert!(render(&parse(escaped)).is_err());
    // A Field-valued witness remains outside this Boolean assertion domain.
    let mut witness_result = source;
    witness_result["witnesses"][0]["result"] = serde_json::json!({"kind":"field"});
    assert!(render(&parse(witness_result)).is_err());
}

#[test]
fn zswap_recording_does_not_adopt_query_helpers_or_escaped_bindings() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("native-zswap-intents-schema18-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let pair = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "pair")
        .unwrap();
    pair.actions.push(StateAction::Expression {
        value: Expr::KernelSelf {
            ty: Type::Struct {
                name: "ContractAddress".into(),
                fields: vec![compact_rust_backend::ir::StructField {
                    name: "bytes".into(),
                    ty: Type::Bytes { length: 32 },
                }],
            },
        },
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    let flow = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "flow")
        .unwrap();
    assert!(!flow.recorded && !flow.observed_call);
    assert!(!rendered.source.contains(".call_local("));
    let pair = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "pair")
        .unwrap();
    pair.actions.pop();
    pair.actions.push(StateAction::Expression {
        value: Expr::CreateZswapOutput {
            coin: Box::new(Expr::Parameter {
                name: "escaped".into(),
            }),
            recipient: Box::new(Expr::Parameter {
                name: "recipient".into(),
            }),
        },
    });
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn composite_intents_require_public_queries_and_exact_typed_effect_operands() {
    let original: Contract =
        serde_json::from_str(include_str!("stateful-struct-schema20-ir.json")).unwrap();
    let transfer: Contract =
        serde_json::from_str(include_str!("composite-zswap-transfer-schema20-ir.json")).unwrap();
    for source in [&original, &transfer] {
        let out = render_with_capabilities(source).unwrap();
        assert!(
            out.capabilities
                .circuits
                .iter()
                .all(|c| c.recorded && c.observed_call)
        );
        assert!(out.source.contains("create_zswap_output("));
        assert!(out.source.contains("kernel_self()?"));
        assert!(!out.source.contains(".call_local("));
    }
    for mode in 0..4 {
        let mut invalid = original.clone();
        let planned = invalid
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "planned")
            .unwrap();
        let StateReturn::Expression {
            value: Expr::StructLiteral { fields, .. },
        } = &mut planned.return_value
        else {
            panic!("planned literal")
        };
        match mode {
            0 | 1 => {
                let Expr::CreateZswapOutput { coin, recipient } = &mut fields[1] else {
                    panic!("output member")
                };
                if mode == 0 {
                    **coin = Expr::Boolean { value: false };
                } else {
                    **recipient = Expr::Boolean { value: false };
                }
            }
            2 => {
                let Expr::CreateZswapOutput { coin, .. } = &mut fields[1] else {
                    panic!("output member")
                };
                **coin = Expr::Parameter {
                    name: "escaped".into(),
                };
            }
            _ => fields[1] = Expr::Boolean { value: true },
        }
        assert!(
            render_with_capabilities(&invalid).is_err(),
            "malformed effect mode {mode}"
        );
    }
    let mut no_query = original.clone();
    let planned = no_query
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "planned")
        .unwrap();
    let StateReturn::Expression {
        value: Expr::StructLiteral { fields, .. },
    } = &mut planned.return_value
    else {
        panic!("planned literal")
    };
    let Expr::KernelSelf { ty } = &fields[2] else {
        panic!("Kernel.self")
    };
    fields[2] = Expr::Default { ty: ty.clone() };
    let out = render_with_capabilities(&no_query).unwrap();
    let planned = out
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "planned")
        .unwrap();
    assert!(!planned.recorded && !planned.observed_call);
    // Mint composition has no evidence in this narrowly admitted value profile.
    let mut unsupported = transfer.clone();
    let StateReturn::Expression {
        value: Expr::StructLiteral { fields, .. },
    } = &mut unsupported.stateful_circuits[0].return_value
    else {
        panic!("transfer literal")
    };
    fields[2] = Expr::KernelMintShielded {
        domain: Box::new(Expr::Parameter {
            name: "nullifier".into(),
        }),
        amount: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: u64::MAX.to_string(),
        }),
    };
    let out = render_with_capabilities(&unsupported).unwrap();
    assert!(!out.capabilities.circuits[0].recorded);
}

#[test]
fn terminal_lexical_source_records_extracted_returns_but_preserves_explicit_scopes() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("terminal-lexical-return-schema20-ir.json")).unwrap();
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
