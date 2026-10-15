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

//! Historic Merkle reset and typed hash insertion profiles.

use super::support::identity;
use compact_rust_backend::ir::{
    Expr, LedgerField, LedgerFieldKind, Parameter, StateAction, StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::render_with_capabilities;

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
