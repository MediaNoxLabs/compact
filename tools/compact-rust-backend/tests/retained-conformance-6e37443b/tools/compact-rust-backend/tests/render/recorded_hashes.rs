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

//! Typed hash helper bindings, ordering, and argument coercions.

use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, PureCircuit,
    SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::render_with_capabilities;

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
fn persistent_hash_helper_recording_preserves_formals_and_rejects_effects() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../inline-type-scope-schema12-ir.json")).unwrap();
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
fn composite_recording_does_not_erase_local_argument_widening() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../struct-helper-widening-schema13-ir.json")).unwrap();
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
