// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source entrypoint first-errors remain outside output-only item assembly.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
fn contract(parameters: Vec<Parameter>, result: Type, body: Expr) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "probe".into(),
            parameters,
            result,
            body,
        }],
        stateful_circuits: vec![],
    }
}

#[test]
fn duplicate_parameter_still_precedes_its_bad_carrier_and_body_errors() {
    let source = contract(
        vec![
            Parameter {
                name: "value".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "value".into(),
                ty: Type::Unsigned { max: "bad".into() },
            },
        ],
        Type::Field,
        reference("missing"),
    );
    assert_eq!(
        crate::render(&source).err(),
        Some(RenderError::DuplicateParameter("value".into()))
    );
}

#[test]
fn body_and_result_mismatch_still_precede_result_carrier_formatting() {
    let result = Type::Unsigned { max: "bad".into() };
    let source = contract(vec![], result.clone(), reference("missing"));
    assert_eq!(
        crate::render(&source).err(),
        Some(RenderError::UnknownParameter("missing".into()))
    );
    let source = contract(
        vec![],
        result.clone(),
        Expr::FieldLiteral { value: "1".into() },
    );
    assert_eq!(
        crate::render(&source).err(),
        Some(RenderError::TypeMismatch {
            expected: result,
            actual: Type::Field
        })
    );
}
