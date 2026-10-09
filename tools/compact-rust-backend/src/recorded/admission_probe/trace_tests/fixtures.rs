// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::ir::{SCHEMA_VERSION, WitnessDeclaration};

fn unit(name: &str, internal: bool) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        internal,
        name: name.into(),
        parameters: vec![],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}
fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}
fn local(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn slot(name: &str, index: u8, declaration: LedgerFieldKind) -> LedgerField {
    LedgerField {
        source: None,
        id: name.into(),
        index,
        path: vec![index],
        declaration,
    }
}
fn empty() -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: vec![],
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
    }
}

pub(super) fn counter() -> Contract {
    let mut contract = empty();
    contract
        .ledger_fields
        .push(slot("round", 0, LedgerFieldKind::Counter));
    let mut read = unit("read", false);
    read.result = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    read.return_value = StateReturn::CounterRead {
        field: "round".into(),
        index: 0,
    };
    contract.stateful_circuits.push(read);
    contract
}

pub(super) fn shared_helper() -> Contract {
    let mut contract = empty();
    contract
        .ledger_fields
        .push(slot("cell", 0, LedgerFieldKind::Cell { ty: Type::Field }));
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![],
        result: Type::Field,
    });
    let mut helper = unit("innerValue", true);
    helper.result = Type::Field;
    helper.return_value = StateReturn::Expression {
        value: Expr::WitnessCall {
            name: "secret".into(),
            arguments: vec![],
        },
    };
    contract.stateful_circuits.push(helper);
    for name in ["outerA", "outerB"] {
        let mut caller = unit(name, false);
        caller.actions.push(StateAction::Let {
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
                value: local("next"),
            }),
        });
        contract.stateful_circuits.push(caller);
    }
    contract
}

pub(super) fn audited_local(local_only: bool) -> Contract {
    let mut contract = empty();
    contract.ledger_fields = vec![
        slot("open", 0, LedgerFieldKind::Cell { ty: Type::Boolean }),
        slot(
            "key",
            1,
            LedgerFieldKind::Cell {
                ty: Type::JubjubPoint,
            },
        ),
    ];
    let mut caller = unit("verify", false);
    caller.parameters = vec![parameter("a", Type::Field), parameter("b", Type::Field)];
    caller.actions = vec![
        StateAction::Assert {
            condition: Expr::CellRead {
                field: "open".into(),
                index: 0,
            },
            message: "closed".into(),
        },
        StateAction::CircuitCall {
            name: "check".into(),
            arguments: vec![
                Expr::Coerce {
                    value: Box::new(local("a")),
                    ty: Type::Field,
                },
                Expr::Coerce {
                    value: Box::new(local("b")),
                    ty: Type::Field,
                },
                Expr::Coerce {
                    value: Box::new(Expr::CellRead {
                        field: "key".into(),
                        index: 1,
                    }),
                    ty: Type::JubjubPoint,
                },
            ],
        },
    ];
    let mut helper = unit("check", true);
    helper.parameters = vec![
        parameter("x", Type::Field),
        parameter("y", Type::Field),
        parameter("key", Type::JubjubPoint),
    ];
    if !local_only {
        helper.actions.push(StateAction::CellWrite {
            field: "open".into(),
            index: 0,
            value: Expr::Boolean { value: false },
        });
    }
    contract.stateful_circuits = vec![caller, helper];
    contract
}
