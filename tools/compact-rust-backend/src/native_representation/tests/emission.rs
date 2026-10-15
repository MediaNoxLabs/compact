// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::ir::{Contract, LocalBinding, PureCircuit, SCHEMA_VERSION, SourceLocation};
use std::collections::HashMap;

fn inspect(value: &Expr) -> Result<(syn::Expr, Type), RenderError> {
    crate::expression_with_calls(value, &HashMap::new(), &HashMap::new())
}

#[test]
fn native_emission_checks_each_operation_and_preserves_the_smaller_controls() {
    for operation in OPERATIONS {
        let limit = operation.tuple_limit();
        inspect(&expression(operation, tuple_value(limit))).unwrap();
        assert_refusal(
            inspect(&expression(operation, tuple_value(limit + 1)))
                .err()
                .unwrap(),
            operation,
            limit + 1,
            operation.required_trait(false),
        );
    }
}

#[test]
fn native_dynamic_hashes_report_the_exact_hash_input_obligation() {
    for operation in [
        Operation::TransientHash,
        Operation::TransientCommit,
        Operation::HashToCurve,
    ] {
        let preimage = Expr::Tuple {
            elements: vec![
                tuple_value(12),
                Expr::Default {
                    ty: Type::OpaqueString,
                },
            ],
        };
        assert_refusal(
            inspect(&expression(operation, preimage)).err().unwrap(),
            operation,
            12,
            "HashInputValue",
        );
    }
}

#[test]
fn wide_local_tuple_construction_projection_and_movement_remain_valid() {
    assert_eq!(inspect(&tuple_value(13)).unwrap().1, tuple(13));
    assert_eq!(
        inspect(&Expr::TupleIndex {
            value: Box::new(tuple_value(13)),
            index: 12,
        })
        .unwrap()
        .1,
        Type::Field
    );
    let moved = Expr::Let {
        bindings: vec![LocalBinding {
            name: "owned".into(),
            ty: tuple(13),
            value: tuple_value(13),
        }],
        body: Box::new(Expr::Parameter {
            name: "owned".into(),
        }),
    };
    assert_eq!(inspect(&moved).unwrap().1, tuple(13));
}

#[test]
fn unused_let_and_dead_conditional_operations_are_still_checked() {
    for operation in OPERATIONS {
        let limit = operation.tuple_limit();
        let invalid = expression(operation, tuple_value(limit + 1));
        let output_type = inspect(&expression(operation, tuple_value(limit)))
            .unwrap()
            .1;
        let unused = Expr::Let {
            bindings: vec![LocalBinding {
                name: "unused".into(),
                ty: output_type.clone(),
                value: invalid.clone(),
            }],
            body: Box::new(Expr::Unit),
        };
        let conditional = Expr::If {
            condition: Box::new(Expr::Boolean { value: false }),
            then: Box::new(invalid),
            otherwise: Box::new(Expr::Default { ty: output_type }),
        };
        for carrier in [unused, conditional] {
            assert_refusal(
                inspect(&carrier).err().unwrap(),
                operation,
                limit + 1,
                operation.required_trait(false),
            );
        }
    }
}

#[test]
fn existing_child_and_opening_type_errors_keep_precedence() {
    for mut operation in [
        expression(Operation::Equal, tuple_value(13)),
        expression(Operation::NotEqual, tuple_value(13)),
    ] {
        match &mut operation {
            Expr::Equal { right, .. } | Expr::NotEqual { right, .. } => {
                **right = Expr::Boolean { value: true };
            }
            _ => unreachable!(),
        }
        assert_eq!(
            inspect(&operation).err().unwrap(),
            RenderError::TypeMismatch {
                expected: tuple(13),
                actual: Type::Boolean,
            }
        );
    }
    for (operation, expected) in [
        (Operation::TransientCommit, Type::Field),
        (Operation::PersistentCommit, Type::Bytes { length: 32 }),
    ] {
        let mut invalid = expression(operation, tuple_value(12));
        match &mut invalid {
            Expr::TransientCommit { opening, .. } | Expr::PersistentCommit { opening, .. } => {
                **opening = Expr::Boolean { value: true };
            }
            _ => unreachable!(),
        }
        assert_eq!(
            inspect(&invalid).err().unwrap(),
            RenderError::TypeMismatch {
                expected,
                actual: Type::Boolean
            }
        );
    }
    let unknown = Expr::Equal {
        left: Box::new(tuple_value(13)),
        right: Box::new(Expr::Parameter {
            name: "missing".into(),
        }),
    };
    assert_eq!(
        inspect(&unknown).err().unwrap(),
        RenderError::UnknownParameter("missing".into())
    );
}

#[test]
fn full_generation_locates_pure_operation_refusal_before_recording_publication() {
    let location = SourceLocation {
        file: "pure-operation.compact".into(),
        line: 7,
        column: 1,
    };
    for operation in OPERATIONS {
        let limit = operation.tuple_limit();
        let output_type = inspect(&expression(operation, tuple_value(limit)))
            .unwrap()
            .1;
        let contract = Contract {
            schema_version: SCHEMA_VERSION,
            type_aliases: vec![],
            ledger_fields: vec![],
            constructor: None,
            witnesses: vec![],
            circuits: vec![PureCircuit {
                source: Some(location.clone()),
                name: "probe".into(),
                internal: true,
                parameters: vec![],
                result: output_type,
                body: expression(operation, tuple_value(limit + 1)),
            }],
            stateful_circuits: vec![],
        };
        let error = crate::render(&contract).unwrap_err();
        match error {
            RenderError::Located {
                location: actual,
                error,
            } => {
                assert_eq!(actual, location);
                assert_refusal(
                    *error,
                    operation,
                    limit + 1,
                    operation.required_trait(false),
                );
            }
            other => panic!("expected a located native operation refusal, got {other:?}"),
        }
    }
}
