// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

type Build = fn(Expr) -> Expr;
pub(super) struct Case {
    pub(super) operation: &'static str,
    pub(super) limit: usize,
    pub(super) required_trait: &'static str,
    pub(super) result: Type,
    pub(super) build: Build,
}
impl Case {
    pub(super) fn refusal(&self) -> RenderError {
        RenderError::UnsupportedTupleOperation {
            operation: self.operation,
            arity: self.limit + 1,
            limit: self.limit,
            required_trait: self.required_trait,
        }
    }
}

pub(super) fn cases() -> Vec<Case> {
    vec![
        Case {
            operation: "equality",
            limit: 12,
            required_trait: "PartialEq",
            result: Type::Boolean,
            build: |value| Expr::Equal {
                left: Box::new(value.clone()),
                right: Box::new(value),
            },
        },
        Case {
            operation: "inequality",
            limit: 12,
            required_trait: "PartialEq",
            result: Type::Boolean,
            build: |value| Expr::NotEqual {
                left: Box::new(value.clone()),
                right: Box::new(value),
            },
        },
        Case {
            operation: "transientHash",
            limit: 11,
            required_trait: "Into<Value> + Aligned",
            result: Type::Field,
            build: |value| Expr::TransientHash {
                value: Box::new(value),
            },
        },
        Case {
            operation: "transientCommit",
            limit: 11,
            required_trait: "Into<Value> + Aligned",
            result: Type::Field,
            build: |value| Expr::TransientCommit {
                value: Box::new(value),
                opening: Box::new(Expr::FieldLiteral { value: "7".into() }),
            },
        },
        Case {
            operation: "persistentHash",
            limit: 11,
            required_trait: "Into<Value> + Aligned",
            result: Type::Bytes { length: 32 },
            build: |value| Expr::PersistentHash {
                value: Box::new(value),
            },
        },
        Case {
            operation: "persistentCommit",
            limit: 11,
            required_trait: "Into<Value> + Aligned",
            result: Type::Bytes { length: 32 },
            build: |value| Expr::PersistentCommit {
                value: Box::new(value),
                opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
            },
        },
        Case {
            operation: "keccak256",
            limit: 11,
            required_trait: "Into<Value>",
            result: Type::Bytes { length: 32 },
            build: |value| Expr::Keccak256 {
                value: Box::new(value),
            },
        },
        Case {
            operation: "hashToCurve",
            limit: 11,
            required_trait: "Into<Value> + Aligned",
            result: Type::JubjubPoint,
            build: |value| Expr::HashToCurve {
                value: Box::new(value),
            },
        },
    ]
}

#[derive(Clone, Copy)]
pub(super) enum Route {
    Stateful,
    Constructor,
}

pub(super) fn location(route: Route) -> SourceLocation {
    SourceLocation {
        file: "tuple-capability.compact".into(),
        line: match route {
            Route::Stateful => 7,
            Route::Constructor => 3,
        },
        column: 1,
    }
}

pub(super) fn tuple_type(arity: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; arity],
    }
}
pub(super) fn tuple_value(arity: usize) -> Expr {
    Expr::Tuple {
        elements: vec![Expr::FieldLiteral { value: "1".into() }; arity],
    }
}

pub(super) fn contract(case: &Case, arity: usize, route: Route) -> Contract {
    let body = (case.build)(tuple_value(arity));
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "count".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
    };
    match route {
        Route::Stateful => contract.stateful_circuits.push(StatefulCircuit {
            source: Some(location(route)),
            name: "probe".into(),
            internal: false,
            parameters: vec![],
            actions: vec![StateAction::CounterReset {
                field: "count".into(),
                index: 0,
            }],
            result: case.result.clone(),
            return_value: StateReturn::Expression { value: body },
        }),
        Route::Constructor => {
            contract.constructor = Some(Constructor {
                source: Some(location(route)),
                parameters: vec![],
                steps: vec![ConstructorStep::Expression { value: body }],
            })
        }
    }
    contract
}
