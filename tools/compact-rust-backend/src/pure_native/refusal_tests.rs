// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Actual native traversal controls; pointer observations confer no origin authority.
use super::NativeSink;
use crate::{
    RenderError,
    ir::{Expr, PureCircuit, Type},
    pure_semantics::{expression::fold_expression, scope::Scope},
};
use quote::ToTokens;
use std::collections::HashMap;

#[path = "refusal_tests/workers.rs"]
mod workers;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn boolean(value: bool) -> Expr {
    Expr::Boolean { value }
}
fn identity(source: &Expr) -> usize {
    std::ptr::from_ref(source) as usize
}
fn observe(source: &Expr, sink: &mut NativeSink<'_>) -> Result<(Type, String), RenderError> {
    let bindings: Scope<'_, syn::Ident> = HashMap::new();
    let declarations: HashMap<&str, &PureCircuit> = HashMap::new();
    fold_expression(source, &bindings, &declarations, sink).map(|typed| {
        (
            typed.ty,
            typed.output.into_expression().to_token_stream().to_string(),
        )
    })
}

#[test]
fn child_failure_precedes_parent_and_left_operand_validation() {
    let source = Expr::Add {
        left: Box::new(boolean(true)),
        right: Box::new(parameter("right_missing")),
    };
    let Expr::Add { right, .. } = &source else {
        unreachable!()
    };
    let mut events = Vec::new();
    let outcome = {
        let mut callback = |node: &Expr, error: &RenderError| {
            events.push((identity(node), error.to_string()));
        };
        observe(
            &source,
            &mut NativeSink::with_refusal_callback(&mut callback),
        )
    };
    let expected = RenderError::UnknownParameter("right_missing".into());
    assert_eq!(
        outcome,
        Err(RenderError::UnknownParameter("right_missing".into()))
    );
    assert_eq!(
        events,
        vec![
            (identity(right), expected.to_string()),
            (identity(&source), expected.to_string()),
        ]
    );
}

#[test]
fn early_control_refusal_does_not_visit_either_arm() {
    let source = Expr::If {
        condition: Box::new(field("0")),
        then: Box::new(parameter("then_missing")),
        otherwise: Box::new(parameter("otherwise_missing")),
    };
    let mut events = Vec::new();
    let outcome = {
        let mut callback = |node: &Expr, _: &RenderError| events.push(identity(node));
        observe(
            &source,
            &mut NativeSink::with_refusal_callback(&mut callback),
        )
    };
    assert_eq!(
        outcome,
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    assert_eq!(events, vec![identity(&source)]);
}

#[test]
fn successful_children_leave_parent_relation_failure_as_first_occurrence() {
    let source = Expr::If {
        condition: Box::new(boolean(true)),
        then: Box::new(field("1")),
        otherwise: Box::new(boolean(false)),
    };
    let mut events = Vec::new();
    let outcome = {
        let mut callback = |node: &Expr, _: &RenderError| events.push(identity(node));
        observe(
            &source,
            &mut NativeSink::with_refusal_callback(&mut callback),
        )
    };
    assert_eq!(
        outcome,
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    assert_eq!(events, vec![identity(&source)]);
}

#[test]
fn unavailable_child_capture_is_not_replaced_by_available_parent() {
    let source = Expr::Assert {
        condition: Box::new(parameter("condition_missing")),
        message: "not reached".into(),
    };
    let Expr::Assert { condition, .. } = &source else {
        unreachable!()
    };
    let mut captured = false;
    let mut selected = None;
    let mut available = None;
    let mut events = Vec::new();
    let outcome = {
        let mut callback = |node: &Expr, _: &RenderError| {
            events.push(identity(node));
            if !captured {
                captured = true;
                selected = Some(identity(node));
                // A test-only availability predicate, not a fabricated source span.
                available = (identity(node) == identity(&source)).then_some(identity(node));
            }
        };
        observe(
            &source,
            &mut NativeSink::with_refusal_callback(&mut callback),
        )
    };
    assert_eq!(
        outcome,
        Err(RenderError::UnknownParameter("condition_missing".into()))
    );
    assert!(captured);
    assert_eq!(selected, Some(identity(condition)));
    assert_eq!(available, None);
    assert_eq!(events, vec![identity(condition), identity(&source)]);
}

#[test]
fn equal_valued_distinct_sources_keep_distinct_failure_identities() {
    let sources = [parameter("same_missing"), parameter("same_missing")];
    assert_eq!(sources[0], sources[1]);
    assert_ne!(identity(&sources[0]), identity(&sources[1]));
    let mut events = Vec::new();
    {
        let mut callback = |node: &Expr, _: &RenderError| events.push(identity(node));
        let mut sink = NativeSink::with_refusal_callback(&mut callback);
        for source in &sources {
            assert_eq!(
                observe(source, &mut sink),
                Err(RenderError::UnknownParameter("same_missing".into()))
            );
        }
    }
    assert_eq!(events, sources.iter().map(identity).collect::<Vec<_>>());
}

#[test]
fn originless_and_observed_paths_preserve_errors_types_and_native_output() {
    let sources = [
        Expr::Add {
            left: Box::new(field("1")),
            right: Box::new(field("2")),
        },
        Expr::Assert {
            condition: Box::new(boolean(true)),
            message: "ok".into(),
        },
        parameter("missing"),
    ];
    for source in &sources {
        let originless = observe(source, &mut NativeSink::new());
        let mut events = Vec::new();
        let observed = {
            let mut callback = |node: &Expr, _: &RenderError| events.push(identity(node));
            observe(
                source,
                &mut NativeSink::with_refusal_callback(&mut callback),
            )
        };
        assert_eq!(observed, originless);
        assert_eq!(events.len(), usize::from(originless.is_err()));
    }
}
