// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete fixture shared by public emission and actual helper planner laws.
use super::*;

pub(super) fn contract(helper_name: &str, caller: bool, collide: bool) -> Contract {
    let seed = Expr::Parameter {
        name: "seed".into(),
    };
    let helper = StatefulCircuit {
        source: None,
        internal: true,
        name: helper_name.into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "value".into(),
                ty: Type::Field,
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![seed.clone()],
                },
            }],
            action: Box::new(StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    let caller_circuit = StatefulCircuit {
        source: None,
        internal: false,
        name: "outer".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::CircuitCall {
            name: helper_name.into(),
            arguments: vec![seed],
        }],
        result: Type::Unsigned {
            max: u64::MAX.to_string(),
        },
        return_value: StateReturn::CounterRead {
            field: "count".into(),
            index: 1,
        },
    };
    let mut stateful_circuits = vec![helper];
    if caller {
        stateful_circuits.push(caller_circuit.clone());
    }
    if collide {
        let mut collision = caller_circuit;
        let normalized = helper_name.strip_prefix("r#").unwrap_or(helper_name);
        collision.name = format!("__compact_recorded_body_{normalized}");
        stateful_circuits.push(collision);
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "cell".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "count".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Counter,
            },
        ],
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
        circuits: vec![],
        stateful_circuits,
    }
}
