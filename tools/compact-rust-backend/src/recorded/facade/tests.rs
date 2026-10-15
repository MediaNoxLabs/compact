// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Public encoding checks must precede conversion and follow result production.
use super::*;
use crate::ir::{Parameter, StateReturn, StructField};
use quote::ToTokens;

fn circuit(parameters: Vec<Parameter>, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "echo".into(),
        internal: false,
        parameters,
        actions: vec![],
        result,
        return_value: StateReturn::Unit,
    }
}
fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}
fn structure(ty: Type) -> Type {
    Type::Struct {
        name: "Envelope".into(),
        fields: vec![StructField {
            name: "body".into(),
            ty,
        }],
    }
}
fn validation_target(statement: &syn::Stmt) -> Option<String> {
    let syn::Stmt::Expr(syn::Expr::Try(attempt), Some(_)) = statement else {
        return None;
    };
    let syn::Expr::Call(call) = attempt.expr.as_ref() else {
        return None;
    };
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    if path.path.segments.last()?.ident != "validate_cell_value" {
        return None;
    }
    assert_eq!(
        path.to_token_stream().to_string(),
        "runtime :: ledger :: CellValue :: validate_cell_value"
    );
    assert_eq!(call.args.len(), 1);
    Some(call.args.first().unwrap().to_token_stream().to_string())
}
fn local_name(statement: &syn::Stmt) -> Option<String> {
    let syn::Stmt::Local(local) = statement else {
        return None;
    };
    let syn::Pat::Ident(name) = &local.pat else {
        return None;
    };
    Some(name.ident.to_string())
}

#[test]
fn observed_inputs_validate_individually_before_conversion_and_callbacks() {
    for witnesses in [false, true] {
        let mut parameters = (0..10)
            .map(|i| parameter(&format!("message{i}"), Type::OpaqueBytes))
            .collect::<Vec<_>>();
        parameters.insert(1, parameter("fixed", Type::Field));
        let root = circuit(parameters, structure(Type::OpaqueString));
        let method = render_observed_call_method(&root, witnesses).unwrap();
        let statements = &method.block.stmts;
        assert_eq!(statements.len(), 14);
        for (i, statement) in statements[..10].iter().enumerate() {
            assert_eq!(validation_target(statement), Some(format!("& message{i}")));
        }
        assert_eq!(local_name(&statements[10]).as_deref(), Some("input"));
        assert_eq!(local_name(&statements[11]).as_deref(), Some("recorded"));
        assert_eq!(
            validation_target(&statements[12]).as_deref(),
            Some("& (recorded . execution . result) . body")
        );
        assert!(matches!(
            &statements[13],
            syn::Stmt::Expr(syn::Expr::Call(_), None)
        ));
        assert_eq!(method.sig.generics.where_clause.is_some(), witnesses);
        // Native/recorded entry methods still support raw local values.
        assert!(
            !render_recorded_contract_method(&root)
                .unwrap()
                .to_token_stream()
                .to_string()
                .contains("validate_cell_value")
        );
        assert!(
            !render_borrowed_recorded_contract_method(&root, witnesses)
                .unwrap()
                .to_token_stream()
                .to_string()
                .contains("validate_cell_value")
        );
    }
}

#[test]
fn nested_opaque_aggregates_are_checked_and_parameter_names_stay_hygienic() {
    let nested = structure(Type::Tuple {
        elements: vec![
            Type::Field,
            Type::Vector {
                element: Box::new(Type::OpaqueBytes),
                length: 2,
            },
        ],
    });
    let root = circuit(
        vec![
            parameter("input", nested.clone()),
            parameter("recorded", Type::OpaqueString),
        ],
        nested,
    );
    let names = public_parameter_idents(&root.parameters);
    let method = render_observed_call_method(&root, true).unwrap();
    fn loop_body(statement: &syn::Stmt) -> (&syn::Expr, &[syn::Stmt]) {
        let syn::Stmt::Expr(syn::Expr::ForLoop(loop_), None) = statement else {
            panic!("expected borrowed vector traversal")
        };
        assert!(matches!(loop_.expr.as_ref(), syn::Expr::Reference(_)));
        (&loop_.expr, &loop_.body.stmts)
    }
    let (input_vector, input_body) = loop_body(&method.block.stmts[0]);
    assert!(
        input_vector
            .to_token_stream()
            .to_string()
            .contains(&names[0].to_string())
    );
    assert_eq!(input_body.len(), 1);
    assert_eq!(
        validation_target(&input_body[0]).as_deref(),
        Some("__compact_opaque_element")
    );
    assert_eq!(
        validation_target(&method.block.stmts[1]),
        Some(format!("& {}", names[1]))
    );
    assert_ne!(names[0], "input");
    assert_ne!(names[1], "recorded");
    assert_eq!(local_name(&method.block.stmts[2]).as_deref(), Some("input"));
    assert_eq!(
        local_name(&method.block.stmts[3]).as_deref(),
        Some("recorded")
    );
    let (output_vector, output_body) = loop_body(&method.block.stmts[4]);
    assert!(
        output_vector
            .to_token_stream()
            .to_string()
            .contains("recorded . execution . result")
    );
    assert_eq!(output_body.len(), 1);
    assert_eq!(
        validation_target(&output_body[0]).as_deref(),
        Some("__compact_opaque_element")
    );
}

#[test]
fn output_only_checks_run_after_execution_for_witness_and_plain_methods() {
    for witnesses in [false, true] {
        let method =
            render_observed_call_method(&circuit(vec![], Type::OpaqueBytes), witnesses).unwrap();
        assert_eq!(method.block.stmts.len(), 4);
        assert_eq!(local_name(&method.block.stmts[0]).as_deref(), Some("input"));
        assert_eq!(
            local_name(&method.block.stmts[1]).as_deref(),
            Some("recorded")
        );
        assert_eq!(
            validation_target(&method.block.stmts[2]).as_deref(),
            Some("& recorded . execution . result")
        );
    }
}

#[test]
fn fixed_public_values_do_not_gain_validation_or_new_trait_requirements() {
    let fixed = structure(Type::Tuple {
        elements: vec![
            Type::Unit,
            Type::Vector {
                element: Box::new(Type::Field),
                length: 3,
            },
        ],
    });
    for witnesses in [false, true] {
        let root = circuit(
            vec![
                parameter("fixed", fixed.clone()),
                parameter("bytes", Type::Bytes { length: 32 }),
            ],
            fixed.clone(),
        );
        let method = render_observed_call_method(&root, witnesses).unwrap();
        assert_eq!(method.block.stmts.len(), 3);
        assert_eq!(local_name(&method.block.stmts[0]).as_deref(), Some("input"));
        assert_eq!(
            local_name(&method.block.stmts[1]).as_deref(),
            Some("recorded")
        );
        assert!(
            method
                .block
                .stmts
                .iter()
                .all(|stmt| validation_target(stmt).is_none())
        );
    }
}

#[test]
fn nine_tuple_public_values_validate_leaves_without_aggregate_trait_bounds() {
    let tuple = Type::Tuple {
        elements: vec![Type::OpaqueBytes; 9],
    };
    for witnesses in [false, true] {
        let method = render_observed_call_method(
            &circuit(vec![parameter("many", tuple.clone())], tuple.clone()),
            witnesses,
        )
        .unwrap();
        let statements = &method.block.stmts;
        assert_eq!(statements.len(), 21);
        for index in 0..9 {
            assert_eq!(
                validation_target(&statements[index]),
                Some(format!("& (many) . {index}"))
            );
            assert_eq!(
                validation_target(&statements[index + 11]),
                Some(format!("& (recorded . execution . result) . {index}"))
            );
        }
        assert_eq!(local_name(&statements[9]).as_deref(), Some("input"));
        assert_eq!(local_name(&statements[10]).as_deref(), Some("recorded"));
    }
}

#[test]
fn vector_of_nine_tuples_is_borrowed_and_not_unrolled_by_length() {
    let vector = Type::Vector {
        element: Box::new(Type::Tuple {
            elements: vec![Type::OpaqueBytes; 9],
        }),
        length: 1000,
    };
    let checks = opaque_checks(syn::parse_quote!(values), &vector).unwrap();
    assert_eq!(checks.len(), 1);
    let syn::Stmt::Expr(syn::Expr::ForLoop(loop_), None) = &checks[0] else {
        panic!("expected loop")
    };
    assert_eq!(loop_.expr.to_token_stream().to_string(), "& (values) . 0");
    assert_eq!(loop_.body.stmts.len(), 9);
    for (index, statement) in loop_.body.stmts.iter().enumerate() {
        assert_eq!(
            validation_target(statement),
            Some(format!("& (__compact_opaque_element) . {index}"))
        );
    }
}
