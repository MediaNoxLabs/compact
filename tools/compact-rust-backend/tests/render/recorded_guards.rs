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

//! Typed arithmetic guards and ordered stateful assertions.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter,
    PureCircuit, SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{render, render_with_capabilities};

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
fn conditional_assertion_keeps_typed_equality_and_branch_local_observations() {
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
    let recorded = rendered.source.split_once("pub mod recorded").unwrap().1;
    assert!(recorded.contains(" == "));
    assert!(recorded.find("AssertionFailed").unwrap() < recorded.find("record_write").unwrap());
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
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    let recorded = rendered.source.split_once("pub mod recorded").unwrap().1;
    assert_eq!(recorded.matches("record_read(frame)").count(), 1);
    assert!(
        recorded.find("record_read(frame)").unwrap() < recorded.find("AssertionFailed").unwrap()
    );
    // Inspect syntax to ensure the observation remains in the selected arm.
    use quote::ToTokens;
    use syn::visit_mut::VisitMut;
    struct SelectedRead(usize);
    impl VisitMut for SelectedRead {
        fn visit_expr_if_mut(&mut self, branch: &mut syn::ExprIf) {
            if branch
                .then_branch
                .to_token_stream()
                .to_string()
                .contains("record_read")
            {
                assert!(
                    !branch
                        .cond
                        .to_token_stream()
                        .to_string()
                        .contains("record_read")
                );
                self.0 += 1;
            }
            syn::visit_mut::visit_expr_if_mut(self, branch);
        }
    }
    let mut syntax = syn::parse_file(&rendered.source).unwrap();
    let mut selected = SelectedRead(0);
    selected.visit_file_mut(&mut syntax);
    assert_eq!(selected.0, 1);

    // Dynamic opaque hashing remains outside the qualified fixed-FAB domain.
    let StateAction::Assert {
        condition: Expr::If { then, .. },
        ..
    } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    **then = Expr::Equal {
        left: Box::new(Expr::TransientHash {
            value: Box::new(Expr::Default {
                ty: Type::OpaqueString,
            }),
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
fn stateful_assertions_require_boolean_and_record_typed_read_only_results() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../stateful-assert-schema20-ir.json")).unwrap();
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
        serde_json::from_str(include_str!("../stateful-assert-schema20-ir.json")).unwrap();
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
fn assertion_recording_composes_typed_effects_and_rejects_invalid_steps_and_slots() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../stateful-assert-schema20-ir.json")).unwrap();
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
    // A correctly typed reset composes with assertions; it no longer needs
    // to match the old assertion-only circuit shape.
    let report = render_with_capabilities(&extra).unwrap();
    assert!(report.capabilities.circuits[0].recorded);
    assert!(report.capabilities.circuits[0].observed_call);
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
