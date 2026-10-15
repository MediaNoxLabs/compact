// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn mapping(name: &str, input: Type, source: Expr, body: Expr, result: Type, length: usize) -> Expr {
    Expr::VectorMap {
        parameter: Parameter {
            name: name.into(),
            ty: input,
        },
        source: Box::new(source),
        body: Box::new(body),
        result,
        length,
    }
}

#[test]
fn map_coercion_name_body_and_result_refusals_stay_ordered_even_at_zero_length() {
    let empty = || vector(vec![]);
    assert_eq!(
        compare(
            &mapping(
                "bad-name",
                Type::Field,
                boolean(true),
                reference("missing"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::TypeMismatch {
            expected: vector_type(Type::Field, 0),
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(
            &mapping(
                "bad-name",
                Type::Field,
                empty(),
                reference("missing"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
    assert_eq!(
        compare(
            &mapping(
                "item$raw",
                Type::Field,
                empty(),
                reference("missing"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        compare(
            &mapping(
                "item$raw",
                Type::Field,
                empty(),
                reference("item$raw"),
                Type::Boolean,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field
        })
    );
    assert_eq!(
        compare(
            &mapping(
                "item$raw",
                Type::Field,
                empty(),
                reference("item$raw"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Ok(vector_type(Type::Field, 0))
    );
}

#[test]
fn map_widening_raw_names_nested_scope_and_outer_binding_restore_match_old_views() {
    let outer = Type::Boolean;
    let params = HashMap::from([("item$raw", (&outer, syn::parse_quote!(outer_symbol)))]);
    let small = uint("15");
    let large = uint("255");
    let input = Expr::Vector {
        element: small,
        elements: vec![Expr::UnsignedLiteral {
            value: "7".into(),
            max: "15".into(),
        }],
    };
    let mapped = mapping(
        "item$raw",
        large.clone(),
        input,
        reference("item$raw"),
        large.clone(),
        1,
    );
    assert_eq!(
        compare(&mapped, &params, &HashMap::new()),
        Ok(vector_type(large, 1))
    );
    let inner = mapping(
        "item$raw",
        Type::Field,
        vector(vec![field("2")]),
        reference("item$raw"),
        Type::Field,
        1,
    );
    let mapped = mapping(
        "item$raw",
        Type::Field,
        vector(vec![field("1")]),
        inner,
        vector_type(Type::Field, 1),
        1,
    );
    let source = Expr::Tuple {
        elements: vec![mapped, reference("item$raw")],
    };
    assert_eq!(
        compare(&source, &params, &HashMap::new()),
        Ok(Type::Tuple {
            elements: vec![vector_type(vector_type(Type::Field, 1), 1), Type::Boolean]
        })
    );
    let (result, visits) = visitation::observe(|| observe(&source, &params, &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 9);
    assert!(visits.values().all(|count| *count == 1));
}

#[test]
fn prepared_map_body_keeps_binary_association_without_child_reparse() {
    let source = mapping(
        "item",
        Type::Field,
        vector(vec![field("1")]),
        Expr::Subtract {
            left: Box::new(field("10")),
            right: Box::new(Expr::Subtract {
                left: Box::new(field("7")),
                right: Box::new(field("2")),
            }),
        },
        Type::Field,
        1,
    );
    assert_eq!(
        compare(&source, &HashMap::new(), &HashMap::new()),
        Ok(vector_type(Type::Field, 1))
    );
    let (syntax, _) =
        crate::pure_aggregate_expression(&source, &HashMap::new(), &HashMap::new()).unwrap();
    let syn::Expr::Block(block) = syntax else {
        panic!("Map block");
    };
    let syn::Stmt::Expr(syn::Expr::ForLoop(mapped_loop), _) = &block.block.stmts[2] else {
        panic!("Map loop");
    };
    let syn::Stmt::Expr(syn::Expr::MethodCall(push), _) = &mapped_loop.body.stmts[0] else {
        panic!("Map push");
    };
    let syn::Expr::Binary(binary) = push.args.first().unwrap() else {
        panic!("binary body");
    };
    assert!(
        matches!(binary.right.as_ref(), syn::Expr::Binary(inner) if matches!(inner.op, syn::BinOp::Sub(_)))
    );
}
