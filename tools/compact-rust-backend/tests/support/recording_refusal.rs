// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-qualified direct fold refusal, with a real pure declaration.
use compact_rust_backend::ir::{Contract, Expr, Parameter, PureCircuit, Type};

const FOLD_STEP: &str = "recording_refusal_fold_step";
pub(crate) fn unsupported_commit_input() -> Expr {
    Expr::VectorFoldCall {
        name: FOLD_STEP.into(),
        initial: Box::new(Expr::FieldLiteral { value: "0".into() }),
        source: Box::new(Expr::Tuple {
            elements: vec![Expr::FieldLiteral { value: "1".into() }],
        }),
        accumulator: Type::Field,
        element: Type::Field,
        length: 1,
    }
}
pub(crate) fn register_unsupported_fold(contract: &mut Contract) {
    assert!(!contract.circuits.iter().any(|c| c.name == FOLD_STEP));
    contract.circuits.push(PureCircuit {
        source: None,
        name: FOLD_STEP.into(),
        internal: true,
        parameters: ["acc", "item"]
            .into_iter()
            .map(|name| Parameter {
                name: name.into(),
                ty: Type::Field,
            })
            .collect(),
        result: Type::Field,
        body: Expr::Add {
            left: Box::new(Expr::Parameter { name: "acc".into() }),
            right: Box::new(Expr::Parameter {
                name: "item".into(),
            }),
        },
    });
}
