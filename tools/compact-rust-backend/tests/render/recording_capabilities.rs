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
fn certified_pure_call_and_field_arithmetic_have_exact_capabilities() {
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
    assert!(rendered.source.contains("pub mod recorded"));
    assert!(rendered.source.contains("pub fn write_call"));
    let body = super::lifecycle_recording::generated_body(&rendered.source, "recorded", "write");
    assert!(body.find("pure_circuits :: square").unwrap() < body.find("record_write").unwrap());
    let report = serde_json::to_value(&rendered.capabilities).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["circuits"][0]["name"], "write");
    assert_eq!(report["circuits"][0]["source"]["file"], "pure-call.compact");
    assert_eq!(report["circuits"][0]["source"]["line"], 4);
    assert_eq!(report["circuits"][0]["recorded"], true);
    assert_eq!(report["circuits"][0]["observed_call"], true);
    assert!(report["circuits"][0].get("recording_unavailable").is_none());
    assert!(
        report["circuits"][0]
            .get("observed_call_unavailable")
            .is_none()
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
        "available"
    );
    assert!(matches!(
        render_with_proof_capabilities(&contract, &serde_json::json!({"circuits": []})),
        Err(RenderError::ProofApplicability(_))
    ));

    // Certification does not turn a malformed pure call into a capability.
    let mut wrong_arity = contract.clone();
    let StateAction::CellWrite {
        value: Expr::Call { arguments, .. },
        ..
    } = &mut wrong_arity.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    arguments.clear();
    assert!(render_with_capabilities(&wrong_arity).is_err());

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
    let body = super::lifecycle_recording::generated_body(&rendered.source, "recorded", "write");
    assert_eq!(body.matches(" - ").count(), 1);
    assert!(body.find(" - ").unwrap() < body.find("record_write").unwrap());
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::Multiply { left, right };
    let rendered = render_with_capabilities(&contract).unwrap();
    let body = super::lifecycle_recording::generated_body(&rendered.source, "recorded", "write");
    assert_eq!(body.matches(" * ").count(), 1);
    assert!(body.find(" * ").unwrap() < body.find("record_write").unwrap());
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
                        value: Expr::Default { ty: Type::Boolean },
                    },
                    StateAction::Expression {
                        value: Expr::Boolean { value: false },
                    },
                ],
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);
    let StateAction::Sequence { actions } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    actions[1] = StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "0".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
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
    // The membership interface now admits a parameterless Boolean result.
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);
    assert!(report.circuits[0].recording_unavailable.is_none());
}

#[test]
fn conditional_recording_rejects_an_unsupported_unselected_branch() {
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
            actions: vec![StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(StateAction::CounterReset {
                    field: "round".into(),
                    index: 0,
                }),
                otherwise: Box::new(StateAction::Expression {
                    value: Expr::Default { ty: Type::Boolean },
                }),
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);
    let StateAction::If { otherwise, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    **otherwise = StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "0".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
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
    let owned_cell_type = Type::OpaqueString;
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
                ty: owned_cell_type.clone(),
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
                    ty: owned_cell_type,
                },
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);
    // Owned scalar storage is supported; an opaque vector remains outside the
    // qualified storage codec even though its local Default is representable.
    let unsupported_storage = Type::Vector {
        element: Box::new(Type::OpaqueString),
        length: 2,
    };
    contract.ledger_fields[0].declaration = LedgerFieldKind::Cell {
        ty: unsupported_storage.clone(),
    };
    contract.stateful_circuits[0].actions[0] = StateAction::CellWrite {
        field: "small".into(),
        index: 0,
        value: Expr::Default {
            ty: unsupported_storage,
        },
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

    // Preserve the newly supported nested scalar default, then exercise the
    // same callee diagnostic with a refused commitment input policy.
    contract.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::Default { ty: Type::Boolean },
    }];
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert!(report.circuits[0].recorded && report.circuits[0].observed_call);
    contract.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "0".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
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
