// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete native stateful and constructor boundaries, not recorded admission.
use super::*;
use crate::ir::{
    Constructor, ConstructorStep, Contract, LocalBinding, SCHEMA_VERSION, SourceLocation,
    StateAction, StateReturn,
};

mod cases;
use cases::*;

fn lower(value: &Expr, declarations: &[WitnessDeclaration]) -> (Result<Type, RenderError>, String) {
    let mut statements = Vec::new();
    let result = render_state_expression(
        value,
        &HashMap::new(),
        &declarations.iter().map(|w| (w.name.as_str(), w)).collect(),
        &mut statements,
        &mut 0,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &mut false,
    )
    .map(|(_, ty, _)| ty);
    (result, quote::quote!(#(#statements)*).to_string())
}

#[test]
fn full_native_stateful_and_constructor_boundaries_keep_controls_and_locate_refusals() {
    for case in cases() {
        for route in [Route::Stateful, Route::Constructor] {
            crate::render(&contract(&case, case.limit, route)).unwrap();
            let error = crate::render(&contract(&case, case.limit + 1, route)).unwrap_err();
            assert_eq!(
                error,
                RenderError::Located {
                    location: location(route),
                    error: Box::new(case.refusal()),
                }
            );
        }
    }
}

#[test]
fn bad_operand_and_opening_types_still_precede_the_new_trait_refusal() {
    for case in cases() {
        let mut expression = (case.build)(tuple_value(case.limit + 1));
        match &mut expression {
            Expr::Equal { right, .. } | Expr::NotEqual { right, .. } => {
                **right = Expr::Boolean { value: true };
                assert_eq!(
                    lower(&expression, &[]).0,
                    Err(RenderError::TypeMismatch {
                        expected: tuple_type(13),
                        actual: Type::Boolean,
                    })
                );
            }
            Expr::TransientCommit { opening, .. } | Expr::PersistentCommit { opening, .. } => {
                let expected = if case.operation == "transientCommit" {
                    Type::Field
                } else {
                    Type::Bytes { length: 32 }
                };
                **opening = Expr::Boolean { value: true };
                assert_eq!(
                    lower(&expression, &[]).0,
                    Err(RenderError::TypeMismatch {
                        expected,
                        actual: Type::Boolean
                    })
                );
            }
            _ => {}
        }
    }
    let expression = Expr::Equal {
        left: Box::new(tuple_value(13)),
        right: Box::new(Expr::Parameter {
            name: "missing".into(),
        }),
    };
    assert_eq!(
        lower(&expression, &[]).0,
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn witnessed_commit_operands_keep_left_to_right_error_precedence() {
    let witnesses = [
        WitnessDeclaration {
            source: None,
            name: "preimage".into(),
            parameters: vec![],
            result: Type::Field,
        },
        WitnessDeclaration {
            source: None,
            name: "opening".into(),
            parameters: vec![],
            result: Type::Boolean,
        },
    ];
    let mut elements = match tuple_value(12) {
        Expr::Tuple { elements } => elements,
        _ => unreachable!(),
    };
    elements[0] = Expr::WitnessCall {
        name: "preimage".into(),
        arguments: vec![],
    };
    let expression = Expr::TransientCommit {
        value: Box::new(Expr::Tuple { elements }),
        opening: Box::new(Expr::WitnessCall {
            name: "opening".into(),
            arguments: vec![],
        }),
    };
    let (result, statements) = lower(&expression, &witnesses);
    assert_eq!(
        result,
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    let first = statements.find("witnesses . preimage (").unwrap();
    let second = statements.find("witnesses . opening (").unwrap();
    assert!(first < second);
    assert_eq!(statements.matches("witnesses . preimage (").count(), 1);
    assert_eq!(statements.matches("witnesses . opening (").count(), 1);
}

#[test]
fn hidden_unused_and_dead_branch_sites_use_the_same_operation_policy() {
    for case in cases() {
        let invalid = (case.build)(tuple_value(case.limit + 1));
        for hidden in [
            Expr::Let {
                bindings: vec![LocalBinding {
                    name: "unused".into(),
                    ty: case.result.clone(),
                    value: invalid.clone(),
                }],
                body: Box::new(Expr::Unit),
            },
            Expr::If {
                condition: Box::new(Expr::Boolean { value: false }),
                then: Box::new(invalid),
                otherwise: Box::new(Expr::Default {
                    ty: case.result.clone(),
                }),
            },
        ] {
            assert_eq!(lower(&hidden, &[]).0, Err(case.refusal()));
        }
    }
}

#[test]
fn opaque_nested_preimages_report_hash_input_value_only_for_dynamic_hashes() {
    for case in cases().into_iter().filter(|case| {
        matches!(
            case.operation,
            "transientHash" | "transientCommit" | "hashToCurve"
        )
    }) {
        let preimage = Expr::Tuple {
            elements: vec![
                tuple_value(12),
                Expr::Default {
                    ty: Type::OpaqueString,
                },
            ],
        };
        let mut expected = case.refusal();
        if let RenderError::UnsupportedTupleOperation { required_trait, .. } = &mut expected {
            *required_trait = "HashInputValue";
        }
        assert_eq!(lower(&(case.build)(preimage), &[]).0, Err(expected));
    }
}

#[test]
fn wide_local_owned_values_and_execution_results_remain_supported() {
    let mut source = contract(&cases()[0], 12, Route::Stateful);
    source.stateful_circuits[0].result = tuple_type(13);
    source.stateful_circuits[0].return_value = StateReturn::Expression {
        value: tuple_value(13),
    };
    crate::render(&source).unwrap();
    let moved = Expr::Let {
        bindings: vec![LocalBinding {
            name: "owned".into(),
            ty: tuple_type(13),
            value: tuple_value(13),
        }],
        body: Box::new(Expr::Parameter {
            name: "owned".into(),
        }),
    };
    assert_eq!(lower(&moved, &[]).0.unwrap(), tuple_type(13));
    assert_eq!(
        lower(
            &Expr::TupleIndex {
                value: Box::new(tuple_value(13)),
                index: 12
            },
            &[]
        )
        .0
        .unwrap(),
        Type::Field
    );
    let mut constructor = contract(&cases()[0], 12, Route::Constructor);
    constructor.constructor.as_mut().unwrap().steps =
        vec![ConstructorStep::Expression { value: moved }];
    crate::render(&constructor).unwrap();
}

#[test]
fn pure_value_and_stateful_helper_namespaces_do_not_bypass_root_checks() {
    let case = cases().remove(0);
    let mut pure_source = contract(&case, 12, Route::Stateful);
    pure_source.circuits.push(PureCircuit {
        source: None,
        name: "owned".into(),
        internal: true,
        parameters: vec![],
        result: tuple_type(13),
        body: tuple_value(13),
    });
    pure_source.stateful_circuits[0].return_value = StateReturn::Expression {
        value: (case.build)(Expr::Call {
            name: "owned".into(),
            arguments: vec![],
        }),
    };
    assert_eq!(
        crate::render(&pure_source).unwrap_err(),
        RenderError::Located {
            location: location(Route::Stateful),
            error: Box::new(case.refusal())
        }
    );
    let mut helper_source = contract(&case, 13, Route::Stateful);
    helper_source.stateful_circuits[0].internal = true;
    helper_source.stateful_circuits[0].name = "internalProbe".into();
    helper_source.stateful_circuits.push(StatefulCircuit {
        source: None,
        name: "outer".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Boolean,
        return_value: StateReturn::Expression {
            value: Expr::Call {
                name: "internalProbe".into(),
                arguments: vec![],
            },
        },
    });
    assert_eq!(
        crate::render(&helper_source).unwrap_err(),
        RenderError::Located {
            location: location(Route::Stateful),
            error: Box::new(case.refusal())
        }
    );
}

#[test]
fn retained_source_tuple_execution_results_are_accepted_without_fab_reclassification() {
    for source in [
        include_str!("../../../tests/tuple12-result-ir.json"),
        include_str!("../../../tests/tuple13-result-ir.json"),
    ] {
        let contract: Contract = serde_json::from_str(source).unwrap();
        let generated = crate::render(&contract).unwrap();
        // These retained source-derived fixtures exercise both native and
        // recorded execution result ownership; this is a generation assertion.
        assert!(generated.contains("pub mod recorded"));
    }
}
