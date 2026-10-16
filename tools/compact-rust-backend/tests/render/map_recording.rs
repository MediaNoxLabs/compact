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

//! Map key/value typing and closed opaque-key lookup sequences.

use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, Parameter, SCHEMA_VERSION, StateAction,
    StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn opaque_string_map_recording_composes_writes_and_keeps_return_domain() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../opaque-string-map-schema13-ir.json")).unwrap();
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
        vec![("put".into(), true), ("ensure".into(), true)]
    );
    let source = render_with_capabilities(&contract).unwrap().source;
    let body = super::lifecycle_recording::generated_body(&source, "recorded", "put");
    assert_eq!(body.matches("record_insert").count(), 2);
    contract.stateful_circuits[0].actions.truncate(1);
    let StateAction::MapInsert { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::FieldLiteral { value: "7".into() };
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), true), ("ensure".into(), true)]
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
        vec![("put".into(), true), ("ensure".into(), true)]
    );
    let output = render_with_capabilities(&contract).unwrap();
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "ensure")
        .unwrap();
    assert!(capability.recorded && capability.observed_call);
    assert!(capability.recording_unavailable.is_none());
    let body = super::lifecycle_recording::generated_body(&output.source, "recorded", "ensure");
    let member = quote::quote!(.record_member).to_string();
    let lookup = quote::quote!(.record_lookup).to_string();
    assert_eq!(body.matches(&member).count(), 1);
    assert_eq!(body.matches(&lookup).count(), 1);
    assert_eq!(body.matches("missing key").count(), 1);
    // The retained membership/assert prefix and discarded lookup still execute
    // once before the now-independent Field result.
    let bytes = [7u8].into_iter().chain([0u8; 31]);
    let literal = quote::quote!([#(#bytes,)*]).to_string();
    assert_eq!(body.matches(&literal).count(), 1);
    assert!(body.find(&member).unwrap() < body.find("missing key").unwrap());
    assert!(body.find("missing key").unwrap() < body.find(&lookup).unwrap());
    assert!(body.find(&lookup).unwrap() < body.find(&literal).unwrap());
    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut contract.stateful_circuits[1].return_value
    else {
        unreachable!()
    };
    **body = Expr::Boolean { value: true };
    assert!(matches!(
        render(&contract),
        Err(RenderError::Located { error, .. })
            if matches!(*error, RenderError::TypeMismatch { expected: Type::Field, actual: Type::Boolean })
    ));
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
    let output = render_with_capabilities(&contract).unwrap();
    assert!(
        output.capabilities.circuits[0].recorded && output.capabilities.circuits[0].observed_call
    );
    assert!(
        output
            .source
            .split("pub mod recorded")
            .nth(1)
            .unwrap()
            .contains("record_insert_default(")
    );
    let StateAction::MapInsertDefault { key, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "value".into(),
    };
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));

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
