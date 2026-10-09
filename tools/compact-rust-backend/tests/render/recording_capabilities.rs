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

//! Capability classification and deterministic recording gap reporting.

use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, Parameter, PureCircuit, SCHEMA_VERSION,
    SourceLocation, StateAction, StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{RenderError, render_with_capabilities, render_with_proof_capabilities};

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
                        value: Expr::PersistentHash {
                            value: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
                        },
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
    // Compositional Unit recording can be empty; preparation is a separate gate.
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);
    assert!(report.circuits[0].recording_unavailable.is_none());

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
                    value: Expr::PersistentHash {
                        value: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
                    },
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
    // A discarded Boolean expression in a checked statement call is supported.
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert_eq!(report.circuits.len(), 1);
    assert_eq!(report.circuits[0].name, "outer");
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);

    // Preserve the nested callee-path objective with an actually unsupported
    // direct operation (native hashing still renders successfully).
    contract.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::PersistentHash {
            value: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    }];
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
