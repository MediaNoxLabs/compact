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

//! Witness signatures and ordered standalone witness recording.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind, Parameter, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::render_with_capabilities;

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
