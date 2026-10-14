// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    ir::{Expr, LocalBinding, PureCircuit},
    pure_semantics::{
        body_check::{BodyCheckSink, ObservedBodyCheckSink},
        expression::{fold_expression, fold_expression_in_session},
    },
};
use std::collections::HashMap;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn oversized() -> Type {
    let mut ty = Type::Field;
    for _ in 0..60 {
        ty = Type::Vector {
            length: 1,
            element: Box::new(ty),
        };
    }
    ty
}
fn check(expr: &Expr, bindings: &Scope<'_, ()>) -> Result<Type, RenderError> {
    fold_expression(
        expr,
        bindings,
        &HashMap::<&str, &PureCircuit>::new(),
        &mut BodyCheckSink::new(),
    )
    .map(|typed| typed.ty)
}

#[test]
fn repeated_uses_share_only_successful_resource_bounds() {
    // Carrier/malformed-maximum acceptance is deliberately unchanged at a use.
    let ty = Type::Unsigned { max: "01".into() };
    let raw = Scope::from([("p", (&ty, ()))]);
    let checked = TypeBoundsSession::new(&raw);
    let expr = Expr::Tuple {
        elements: vec![parameter("p"), parameter("p"), parameter("p")],
    };
    let declarations = HashMap::<&str, &PureCircuit>::new();
    let result =
        fold_expression_in_session(&expr, &checked, &declarations, &mut BodyCheckSink::new())
            .unwrap()
            .ty;
    assert_eq!(
        result,
        Type::Tuple {
            elements: vec![ty.clone(), ty.clone(), ty.clone()]
        }
    );
    assert_eq!(checked.checks.get(), 1);
    assert_eq!(checked.successful.borrow().len(), 1);
    assert!(matches!(checked.bindings, Cow::Borrowed(_)));
    assert_eq!(check(&expr, &raw), Ok(result));
}

#[test]
fn raw_private_bindings_keep_oversized_guard_and_unknown_lookup_priority() {
    let bad = oversized();
    let raw = Scope::from([("oversized", (&bad, ()))]);
    for _ in 0..2 {
        assert!(matches!(
            raw.get_bounded("oversized"),
            Err(RenderError::ResourceLimit {
                resource: "syntax_depth",
                observed: 49,
                limit: 48
            })
        ));
    }
    assert_eq!(raw.get_bounded("missing"), Ok(None));
    assert_eq!(
        check(&parameter("missing"), &raw),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn failed_checked_annotations_never_become_successful_facts() {
    let bad = oversized();
    let raw = Scope::from([("bad", (&bad, ()))]);
    let checked = TypeBoundsSession::new(&raw);
    for _ in 0..2 {
        assert!(matches!(
            checked.get_bounded("bad"),
            Err(RenderError::ResourceLimit {
                resource: "syntax_depth",
                ..
            })
        ));
        assert!(checked.successful.borrow().is_empty());
        assert_eq!(checked.checks.get(), 0);
    }
}

#[test]
fn same_key_child_replacement_invalidates_and_parent_is_restored() {
    let good = Type::Field;
    let bad = oversized();
    let raw = Scope::from([("p", (&good, ()))]);
    let parent = TypeBoundsSession::new(&raw);
    parent.get_bounded("p").unwrap();
    let mut child = parent.child();
    assert_eq!(child.get_bounded("p"), Ok(Some((&good, &()))));
    assert_eq!(child.checks.get(), 0);
    child.insert("p", &bad, ());
    assert!(!child.successful.borrow().contains("p"));
    assert!(child.get_bounded("p").is_err());
    child.insert("p", &good, ());
    child.get_bounded("p").unwrap();
    assert_eq!(child.checks.get(), 1);
    drop(child);
    assert_eq!(parent.get_bounded("p"), Ok(Some((&good, &()))));
    assert_eq!(parent.checks.get(), 1);
}

#[test]
fn equal_names_in_separate_invocations_cannot_share_annotation_facts() {
    let good = Type::Field;
    let bad = oversized();
    let first = Scope::from([("same", (&good, ()))]);
    assert_eq!(check(&parameter("same"), &first), Ok(Type::Field));
    let second = Scope::from([("same", (&bad, ()))]);
    assert!(matches!(
        check(&parameter("same"), &second),
        Err(RenderError::ResourceLimit {
            resource: "syntax_depth",
            ..
        })
    ));
    let changed = Type::Boolean;
    let third = Scope::from([("same", (&changed, ()))]);
    assert_eq!(check(&parameter("same"), &third), Ok(Type::Boolean));
    // A later invocation may reuse the same storage after the prior borrow ends.
    let mut annotation = Type::Field;
    {
        let scope = Scope::from([("same", (&annotation, ()))]);
        assert_eq!(check(&parameter("same"), &scope), Ok(Type::Field));
    }
    annotation = oversized();
    let scope = Scope::from([("same", (&annotation, ()))]);
    assert!(matches!(
        check(&parameter("same"), &scope),
        Err(RenderError::ResourceLimit {
            resource: "syntax_depth",
            ..
        })
    ));
}

#[test]
fn lexical_shadow_checks_initializer_in_prior_scope_and_restores_outer_type() {
    let outer = Type::Field;
    let raw = Scope::from([("p", (&outer, ()))]);
    let nested = Expr::Let {
        bindings: vec![LocalBinding {
            name: "p".into(),
            ty: Type::Boolean,
            value: Expr::Boolean { value: true },
        }],
        body: Box::new(parameter("p")),
    };
    let expr = Expr::Tuple {
        elements: vec![parameter("p"), nested, parameter("p")],
    };
    assert_eq!(
        check(&expr, &raw),
        Ok(Type::Tuple {
            elements: vec![Type::Field, Type::Boolean, Type::Field]
        })
    );
    let mismatched = Expr::Let {
        bindings: vec![LocalBinding {
            name: "p".into(),
            ty: Type::Boolean,
            value: parameter("p"),
        }],
        body: Box::new(parameter("p")),
    };
    assert_eq!(
        check(&mismatched, &raw),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field
        })
    );
    assert_eq!(check(&parameter("p"), &raw), Ok(Type::Field));
}

#[test]
fn source_observer_keeps_actual_unknown_occurrence_and_originless_error() {
    let ty = Type::Field;
    let raw = Scope::from([("p", (&ty, ()))]);
    let expr = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Equal {
                left: Box::new(parameter("p")),
                right: Box::new(parameter("p")),
            }),
            message: String::new(),
        }],
        value: Box::new(parameter("missing")),
    };
    let Expr::Sequence { value, .. } = &expr else {
        panic!("fixture")
    };
    let mut seen = Vec::new();
    let mut observer = |source: &Expr, _: &RenderError| {
        seen.push(std::ptr::from_ref(source) as usize);
    };
    let result = fold_expression(
        &expr,
        &raw,
        &HashMap::<&str, &PureCircuit>::new(),
        &mut ObservedBodyCheckSink::new(&mut observer),
    )
    .map(|typed| typed.ty);
    assert_eq!(result, check(&expr, &raw));
    assert_eq!(result, Err(RenderError::UnknownParameter("missing".into())));
    assert_eq!(
        seen.first(),
        Some(&(std::ptr::from_ref(value.as_ref()) as usize))
    );
}
