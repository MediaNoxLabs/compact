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

//! Declared ledger slots, public views, and witness view generation.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, Parameter, SCHEMA_VERSION, StateAction,
    StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::render;

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
fn cell_and_counter_declarations_emit_typed_slots() {
    let mut cell: Contract = serde_json::from_str(include_str!("../../fixtures/cell_boolean.json"))
        .expect("Cell fixture parses");
    cell.schema_version = SCHEMA_VERSION;
    let source = render(&cell).unwrap();
    assert!(source.contains("pub mod ledger_slots"));
    assert!(source.contains("pub const flag: runtime::slots::CellSlot<bool>"));
    assert!(source.contains("&[0u8]"));

    let mut counter: Contract = serde_json::from_str(include_str!("../../fixtures/counter.json"))
        .expect("Counter fixture parses");
    counter.schema_version = SCHEMA_VERSION;
    let source = render(&counter).unwrap();
    assert!(source.contains("pub const round: runtime::slots::CounterSlot"));
    assert!(source.contains("&[0u8]"));
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
