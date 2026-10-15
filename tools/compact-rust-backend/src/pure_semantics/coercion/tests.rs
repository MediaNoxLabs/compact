// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Differential conversion contracts and independently counted work boundaries.
use super::*;
use crate::{ir::StructField, value_lowering::coerce_expression};
use quote::ToTokens;

#[path = "old_reference.rs"]
mod old;

const TWO_128: &str = "340282366920938463463374607431768211456";
const MAX_248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";

fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn tuple(elements: Vec<Type>) -> Type {
    Type::Tuple { elements }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn structure(name: &str, member: &str, ty: Type) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![StructField {
            name: member.into(),
            ty,
        }],
    }
}
fn enumeration(name: &str, variants: &[&str]) -> Type {
    Type::Enum {
        name: name.into(),
        variants: variants.iter().map(|s| (*s).into()).collect(),
    }
}

fn observation(source: &Type, target: &Type, depth: usize) -> Result<(String, bool), RenderError> {
    let plan = plan_coercion(source, target)?;
    Ok((
        coerce_expression(syn::parse_quote!(source()?), source, target, depth)?
            .to_token_stream()
            .to_string(),
        plan.may_fail(),
    ))
}

fn assert_old_output(source: &Type, target: &Type, depth: usize) {
    let prior = old::observed_coercion(syn::parse_quote!(source()?), source, target, depth)
        .map(|(expression, may_fail)| (expression.to_token_stream().to_string(), may_fail));
    assert_eq!(
        observation(source, target, depth),
        prior,
        "{source:?} -> {target:?}, depth {depth}"
    );
    // Exercise the exact frozen entrypoint as well as its private fallibility
    // observer; neither old function silently delegates to the new relation.
    let prior_expression =
        old::coerce_expression(syn::parse_quote!(source()?), source, target, depth)
            .map(|expression| expression.to_token_stream().to_string());
    assert_eq!(
        coerce_expression(syn::parse_quote!(source()?), source, target, depth)
            .map(|expression| expression.to_token_stream().to_string()),
        prior_expression
    );
}

fn corpus() -> Vec<Type> {
    vec![
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 32 },
        uint("0"),
        uint("15"),
        uint("255"),
        uint(TWO_128),
        uint(MAX_248),
        uint("01"),
        uint("bad"),
        tuple(vec![]),
        tuple(vec![uint("15")]),
        tuple(vec![uint("255")]),
        tuple(vec![Type::Boolean, uint("15")]),
        tuple(vec![Type::Boolean, uint("255")]),
        vector(uint("15"), 0),
        vector(uint("255"), 0),
        vector(uint("15"), 1),
        vector(uint("255"), 1),
        vector(uint("255"), 2),
        vector(tuple(vec![uint("15"), Type::Boolean]), 2),
        vector(tuple(vec![uint("255"), Type::Boolean]), 2),
        vector(vector(uint("15"), 2), 3),
        vector(vector(uint("255"), 2), 3),
        structure("Named", "a", uint("15")),
        structure("Named", "a", uint("255")),
        structure("Named", "b", uint("15")),
        structure("Other", "a", uint("15")),
        structure("not a Rust name", "a", uint("01")),
        enumeration("Choice", &["A", "B"]),
        enumeration("Choice", &["B", "A"]),
        enumeration("Other", &["A", "B"]),
        Type::LedgerMap {
            key: Box::new(Type::OpaqueString),
            value: Box::new(uint("15")),
        },
        Type::LedgerMap {
            key: Box::new(Type::OpaqueString),
            value: Box::new(uint("255")),
        },
    ]
}

#[test]
fn bounded_borrowed_equality_matches_structural_type_equality_for_all_variants() {
    let cases = corpus();
    for left in &cases {
        for right in &cases {
            let mut work = Work::new();
            assert_eq!(
                equivalent(left, right, &mut work).unwrap(),
                left == right,
                "{left:?} versus {right:?}"
            );
        }
    }
}

#[test]
fn relation_and_native_spelling_match_the_frozen_owner_over_type_partitions() {
    let cases = corpus();
    for source in &cases {
        for target in &cases {
            assert_old_output(source, target, 0);
        }
    }
}

#[test]
fn identity_precedes_semantic_validity_and_empty_shape_rules_within_bounds() {
    for ty in [
        uint("01"),
        uint("bad"),
        tuple(vec![]),
        vector(uint("01"), 0),
        structure("not a Rust name", "not a member name", uint("01")),
        Type::LedgerMap {
            key: Box::new(uint("bad")),
            value: Box::new(Type::OpaqueBytes),
        },
    ] {
        let plan = plan_coercion(&ty, &ty).unwrap();
        assert!(matches!(plan.kind(), CoercionKind::Identity));
        assert!(!plan.may_fail());
        assert_eq!(
            coerce_expression(syn::parse_quote!(input), &ty, &ty, 0)
                .unwrap()
                .to_token_stream()
                .to_string(),
            "input"
        );
    }
    assert!(plan_coercion(&tuple(vec![]), &vector(Type::Field, 0)).is_err());
    assert!(plan_coercion(&vector(Type::Field, 0), &tuple(vec![])).is_err());
}

#[test]
fn unsigned_bounds_and_bad_children_retain_first_error_and_leaf_payloads() {
    for (source, target, error) in [
        (
            uint("01"),
            uint("bad"),
            RenderError::InvalidUnsignedMaximum("01".into()),
        ),
        (
            uint("255"),
            uint("bad"),
            RenderError::InvalidUnsignedMaximum("bad".into()),
        ),
        (
            uint("255"),
            uint("15"),
            RenderError::TypeMismatch {
                expected: uint("15"),
                actual: uint("255"),
            },
        ),
        (
            tuple(vec![uint("255"), uint("bad")]),
            tuple(vec![uint("15"), Type::Field]),
            RenderError::TypeMismatch {
                expected: uint("15"),
                actual: uint("255"),
            },
        ),
        (
            tuple(vec![uint("01"), uint("255")]),
            tuple(vec![Type::Field, uint("15")]),
            RenderError::InvalidUnsignedMaximum("01".into()),
        ),
        (
            vector(uint("01"), 0),
            vector(Type::Field, 0),
            RenderError::InvalidUnsignedMaximum("01".into()),
        ),
    ] {
        assert_eq!(plan_coercion(&source, &target).err(), Some(error));
        assert_old_output(&source, &target, 0);
    }
}

#[test]
fn mixed_aggregate_temporary_depth_and_owned_opaque_identity_keep_old_output() {
    let sources = [
        tuple(vec![vector(uint("15"), 2), vector(uint("255"), 2)]),
        vector(tuple(vec![uint("15"), Type::OpaqueString]), 2),
        vector(vector(uint("15"), 1), 2),
        tuple(vec![uint("15"), uint("255")]),
    ];
    let targets = [
        vector(vector(Type::Field, 2), 2),
        vector(tuple(vec![uint(TWO_128), Type::OpaqueString]), 2),
        vector(tuple(vec![uint("255")]), 2),
        tuple(vec![Type::Field, uint(TWO_128)]),
    ];
    for (source, target) in sources.iter().zip(&targets) {
        for depth in [0, 3, 17] {
            assert_old_output(source, target, depth);
        }
    }
}

#[test]
fn declared_vector_dimensions_do_not_expand_checked_plan_or_native_output() {
    let build = |length| {
        let source = vector(vector(uint("15"), length), length);
        let target = vector(vector(uint("255"), length), length);
        preflight(&source, &target).unwrap();
        let mut work = Work::new();
        let plan = plan_bounded(&source, &target, &mut work).unwrap();
        let text = coerce_expression(syn::parse_quote!(input), &source, &target, 0)
            .unwrap()
            .to_token_stream()
            .to_string();
        (work.observed, plan.may_fail(), text)
    };
    let small = build(2);
    let large = build(1_000_000);
    assert_eq!(small.0, large.0);
    assert!(large.1);
    assert_eq!(large.2.matches("for ").count(), 2);
    assert_eq!(large.2.matches("cast_unsigned").count(), 1);
    assert!(large.2.len() < small.2.len() + 32);
}

fn cross_shape(n: usize, m: usize) -> (Type, Type) {
    (
        vector(tuple(vec![uint("15"); n]), m),
        tuple(vec![vector(uint("255"), n); m]),
    )
}

#[test]
fn cross_shape_work_has_independently_counted_product_cost_and_exact_small_output() {
    for (n, m) in [(1, 1), (2, 3), (4, 7)] {
        let (source, target) = cross_shape(n, m);
        preflight(&source, &target).unwrap();
        let mut work = Work::new();
        plan_bounded(&source, &target, &mut work).unwrap();
        // Root: two frames + M reserved slots. Each branch: two frames,
        // N slots and N leaves costing 1+1+3+2+3 = 10 units each.
        assert_eq!(work.observed, 2 + 3 * m + 11 * n * m);
        assert_old_output(&source, &target, 0);
    }
}

#[test]
fn ordinary_two_mib_worker_accepts_near_bound_and_refuses_cross_shape_expansion() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            assert_eq!(resource_limits::coercion_work_limit(), 131_072);
            let (source, target) = cross_shape(16, 732);
            preflight(&source, &target).unwrap();
            let mut work = Work::new();
            let plan = plan_bounded(&source, &target, &mut work).unwrap();
            assert_eq!(work.observed, 131_030);
            assert!(plan.may_fail());
            drop(plan);
            let (source, target) = cross_shape(16, 733);
            match plan_coercion(&source, &target).unwrap_err() {
                RenderError::ResourceLimit {
                    resource: "coercion_work",
                    observed,
                    limit: 131_072,
                } => {
                    assert!(observed > 131_072 && observed <= 131_088);
                }
                other => panic!("expected bounded conversion refusal, got {other:?}"),
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn charged_identity_text_and_pending_comparisons_refuse_before_unbounded_equality() {
    let ty = enumeration("Choice", &[&"a".repeat(131_072)]);
    assert!(matches!(
        plan_coercion(&ty, &ty),
        Err(RenderError::ResourceLimit {
            resource: "coercion_work",
            ..
        })
    ));
    let map = Type::LedgerMap {
        key: Box::new(Type::Boolean),
        value: Box::new(Type::Field),
    };
    let mut work = Work {
        observed: 0,
        limit: 100,
        pending_limit: 1,
    };
    assert!(matches!(
        equivalent(&map, &map, &mut work),
        Err(RenderError::ResourceLimit {
            resource: "coercion_pending",
            observed: 2,
            limit: 1
        })
    ));
    let mut work = Work {
        observed: usize::MAX - 1,
        limit: usize::MAX - 2,
        pending_limit: 1,
    };
    assert!(matches!(
        work.charge(10),
        Err(RenderError::ResourceLimit {
            resource: "coercion_work",
            observed: usize::MAX,
            ..
        })
    ));
}

#[test]
fn borrowed_preflight_precedes_equal_type_identity_and_owned_refusal() {
    let mut oversized = Type::Boolean;
    for _ in 0..80 {
        oversized = vector(oversized, 1);
    }
    assert!(matches!(
        plan_coercion(&oversized, &oversized),
        Err(RenderError::ResourceLimit { .. })
    ));
    assert!(matches!(
        plan_coercion(&oversized, &Type::Unit),
        Err(RenderError::ResourceLimit { .. })
    ));
}
