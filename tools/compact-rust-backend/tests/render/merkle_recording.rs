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

//! Plain Merkle roots, hashes, placement, capacity, and reset recording.

use super::support::identity;
use compact_rust_backend::ir::{
    Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, StateAction, StateReturn,
    StatefulCircuit, StructField, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

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

    // Typed composition accepts computed roots and preceding tree effects.
    let mut computed = contract.clone();
    let StateReturn::MerkleCheckRoot { root, .. } = &mut computed.stateful_circuits[0].return_value
    else {
        unreachable!()
    };
    *root = Expr::Default { ty: digest.clone() };
    assert!(
        render_with_capabilities(&computed)
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
        render_with_capabilities(&effectful)
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
        render_with_capabilities(&computed_historic)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
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
        render_with_capabilities(&effectful_historic)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    let rejected = render_with_capabilities(&contract);
    assert!(rejected.is_err() || !rejected.unwrap().capabilities.circuits[0].recorded);
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
    // Conditional operands compose while preserving the exact declared type.
    let composed = render_with_capabilities(&contract).unwrap();
    assert!(composed.capabilities.circuits[0].recorded);
    assert!(composed.capabilities.circuits[0].observed_call);
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
    assert!(
        rendered
            .source
            .split_whitespace()
            .collect::<String>()
            .contains(".record_insert_hash_index(frame,")
    );

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
    // Conditional operands compose while preserving the exact declared type.
    let composed = render_with_capabilities(&contract).unwrap();
    assert!(composed.capabilities.circuits[0].recorded);
    assert!(composed.capabilities.circuits[0].observed_call);
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
    let composed = render_with_capabilities(&contract).unwrap();
    assert!(
        composed
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    let body = super::lifecycle_recording::generated_body(&composed.source, "recorded", "place");
    assert!(body.contains("record_insert_index"));
    assert!(body.contains("if "));

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
    let composed = render_with_capabilities(&contract).unwrap();
    assert!(
        composed
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    let StateAction::MerkleInsertIndex { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *index = 99;
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn indexed_merkle_recording_composes_typed_conditional_position_binding() {
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
    let composed = render_with_capabilities(&contract).unwrap();
    assert!(
        composed.capabilities.circuits[0].recorded
            && composed.capabilities.circuits[0].observed_call
    );
    let body = super::lifecycle_recording::generated_body(&composed.source, "recorded", "replace");
    assert!(body.contains("if ") && body.contains("record_insert_index"));
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::Boolean { value: true };
    assert!(render_with_capabilities(&contract).is_err());
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
