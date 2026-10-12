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

//! Shared recorded call bodies and admitted pure-call composition.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, PureCircuit,
    SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

// Inspect only the requested recorded function, excluding native and observed APIs.
fn recorded_function(source: &str, name: &str) -> String {
    use quote::ToTokens;
    fn find(items: &[syn::Item], name: &str) -> Option<String> {
        for item in items {
            if let syn::Item::Mod(module) = item
                && let Some((_, items)) = &module.content
            {
                if module.ident == "recorded" {
                    return items.iter().find_map(|item| match item {
                        syn::Item::Fn(function) if function.sig.ident == name => {
                            Some(function.block.to_token_stream().to_string())
                        }
                        _ => None,
                    });
                }
                if let Some(found) = find(items, name) {
                    return Some(found);
                }
            }
        }
        None
    }
    find(&syn::parse_file(source).unwrap().items, name).expect("recorded function")
}
fn assert_scoped_witness_write(source: &str, name: &str) {
    let body = recorded_function(source, name);
    assert_eq!(body.matches("try_witness_metered").count(), 1, "{name}");
    assert_eq!(body.matches("record_write").count(), 1, "{name}");
    assert!(body.find("try_witness_metered").unwrap() < body.find("record_write").unwrap());
}

#[test]
fn recorded_field_returning_helpers_compose_once_in_each_caller() {
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
        1
    );
    assert!(source.contains("pub fn outerA<Private"));
    assert!(source.contains("pub fn outerB<Private"));
    for name in ["outerA", "outerB"] {
        assert_scoped_witness_write(&source, name);
    }

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
        1
    );
    assert!(source.contains("__compact_param_0: runtime::Field"));
    assert!(source.contains("__compact_param_1: runtime::Field"));

    // The compiler lowers `value = innerValue(1, 2) + 3` to a Field
    // action binding with an expression-nested call. Its checked scoped body
    // still observes the witness once before the caller writes the result.
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
        1
    );
    assert!(source.contains("pub fn outerNested<Private"));
    assert_scoped_witness_write(&source, "outerNested");

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
        1
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
    let body = recorded_function(&multiplied, "outerNested");
    assert_eq!(body.matches(" * ").count(), 1);
    assert!(body.contains("record_write"));

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
    // Statement calls discard the result, not the Counter observation.
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    assert_eq!(rendered.capabilities.circuits[0].name, "outer");
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert_eq!(
        recorded_function(&rendered.source, "outer")
            .matches("record_read")
            .count(),
        1
    );

    // Scalar defaults are checked even in nested discarded steps.
    contract.stateful_circuits[0]
        .actions
        .push(StateAction::Expression {
            value: Expr::Default { ty: Type::Boolean },
        });
    let defaulted = render_with_capabilities(&contract).unwrap();
    assert!(defaulted.capabilities.circuits[0].recorded);
    assert!(defaulted.capabilities.circuits[0].observed_call);
    // The unchanged commitment input domain refuses a later nested step.
    contract.stateful_circuits[0]
        .actions
        .push(StateAction::Expression {
            value: Expr::PersistentCommit {
                value: Box::new(Expr::FieldLiteral { value: "0".into() }),
                opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
            },
        });
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
    assert!(!rendered.capabilities.circuits[0].observed_call);
    assert!(
        rendered.capabilities.circuits[0]
            .recording_unavailable
            .is_some()
    );
}

#[test]
fn pure_field_calls_preserve_unit_and_record_ordinary_hashed_returns() {
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
    let recorded = recorded_function(&rendered.source, "writeLiteral");
    assert_eq!(recorded.matches("pure_circuits :: literalField").count(), 1);
    assert!(
        recorded.find("pure_circuits :: literalField").unwrap()
            < recorded.find("record_write").unwrap()
    );

    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Call {
            name: "literalField".into(),
            arguments: vec![],
        },
    };
    let returned = render_with_capabilities(&contract).unwrap();
    assert!(returned.capabilities.circuits[0].recorded);
    let recorded = returned.source.split_once("pub mod recorded").unwrap().1;
    assert_eq!(
        recorded
            .matches("crate::pure_circuits::literalField()?")
            .count(),
        2
    );

    contract.stateful_circuits[0].actions = vec![StateAction::CellWrite {
        field: "value".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "7".into() },
    }];

    contract.circuits[1].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "5".into() }),
    };
    let hashed = render_with_capabilities(&contract).unwrap();
    assert!(hashed.capabilities.circuits[0].recorded);
    assert!(hashed.capabilities.circuits[0].observed_call);
    assert!(
        hashed
            .source
            .split_once("pub mod recorded")
            .unwrap()
            .1
            .contains("crate::pure_circuits::literalField()?")
    );

    // Both certified helpers and direct fixed-FAB hashes are available.
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: contract.circuits[1].body.clone(),
    };
    assert!(
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::TransientHash {
            value: Box::new(Expr::Default {
                ty: Type::OpaqueString,
            }),
        },
    };
    let defaulted = render_with_capabilities(&contract).unwrap();
    assert!(defaulted.capabilities.circuits[0].recorded);
    assert!(defaulted.capabilities.circuits[0].observed_call);
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::TransientHash {
            value: Box::new(Expr::PersistentCommit {
                value: Box::new(Expr::FieldLiteral { value: "0".into() }),
                opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
            }),
        },
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
fn typed_pair_hash_call_composes_certified_bodies_and_rejects_invalid_calls() {
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
    let body =
        super::lifecycle_recording::generated_body(&rendered.source, "recorded", "conditional");
    assert!(body.contains("record_read") && body.contains("record_write"));
    assert!(body.contains("hashVector"));

    contract.circuits[0].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "7".into() }),
    };
    // The former recipe required hashing the vector formal. A certified pure
    // helper may instead hash a Field literal while the local vector call stays typed.
    let composed = render_with_capabilities(&contract).unwrap();
    assert!(
        composed
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );

    // An invalid source may be rejected globally or lose recorded admission;
    // neither outcome may expose an executable recorded entry.
    let rejects = |contract: &Contract| match render_with_capabilities(contract) {
        Err(_) => true,
        Ok(rendered) => rendered.capabilities.circuits.iter().all(|c| !c.recorded),
    };
    let mut wrong_argument = contract.clone();
    wrong_argument.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    assert!(rejects(&wrong_argument));
    let mut recursive = contract.clone();
    recursive.stateful_circuits[0]
        .actions
        .push(StateAction::CircuitCall {
            name: "storeVector".into(),
            arguments: vec![Expr::Parameter { name: "v".into() }],
        });
    assert!(rejects(&recursive));
    let mut wrong_slot = contract.clone();
    let StateAction::Let { action, .. } = &mut wrong_slot.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::CellWrite { index, .. } = action.as_mut() else {
        unreachable!()
    };
    *index = 99;
    assert!(rejects(&wrong_slot));
}
