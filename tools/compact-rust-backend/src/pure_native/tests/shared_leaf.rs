// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Compare shared leaf decisions with the authenticated old complete family.
use super::*;
use crate::pure_semantics::{
    carrier::CheckSink,
    leaf::{LeafSource, fold_leaf},
    sink::{CheckedLeaf, LeafSink},
};

struct NoSyntax {
    carrier: CheckSink,
}
impl LeafSink for NoSyntax {
    type Value = ();
    type Binding = ();
    type Carriers = CheckSink;
    fn carriers(&mut self) -> &mut CheckSink {
        &mut self.carrier
    }
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {}
}

// Test protocol classification only, with no semantic decisions or fallback.
fn leaf_source(expr: &Expr) -> LeafSource<'_> {
    match expr {
        Expr::Unit => LeafSource::Unit,
        Expr::Default { ty } => LeafSource::Default(ty),
        Expr::Boolean { value } => LeafSource::Boolean(*value),
        Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
        Expr::BytesLiteral { bytes } => LeafSource::BytesLiteral(bytes),
        Expr::UnsignedLiteral { value, max } => LeafSource::UnsignedLiteral { value, max },
        Expr::Parameter { name } => LeafSource::Parameter(name),
        Expr::EnumVariant { ty, variant } => LeafSource::EnumVariant { ty, variant },
        Expr::KernelSelf { .. } => LeafSource::Effectful,
        _ => unreachable!("this corpus supplies complete leaf-family sources only"),
    }
}

fn compare(source: &Expr, params: &HashMap<&str, (&Type, syn::Ident)>) {
    let current = observe(source, params, &HashMap::new());
    let old = old_oracle::observe(source, params, &HashMap::new());
    assert_eq!(current, old, "{source:?}");
    let check_params: HashMap<&str, (&Type, ())> = params
        .iter()
        .map(|(name, (ty, _))| (*name, (*ty, ())))
        .collect();
    let checked = fold_leaf(
        leaf_source(source),
        &check_params,
        &mut NoSyntax { carrier: CheckSink },
    )
    .map(|typed| typed.ty);
    assert_eq!(
        checked,
        current.map(|(_, _, ty)| ty),
        "generic consumer {source:?}"
    );
}

#[test]
fn shared_leaf_consumers_match_old_types_errors_and_exact_literal_spelling() {
    const TWO_128: &str = "340282366920938463463374607431768211456";
    const MAX_248: &str =
        "452312848583266388373324160190187140051835877600158453279131187530910662655";
    const TWO_248: &str =
        "452312848583266388373324160190187140051835877600158453279131187530910662656";
    const P_MINUS_ONE: &str =
        "52435875175126190479447740508185965837690552500527637822603658699938581184512";
    const P: &str = "52435875175126190479447740508185965837690552500527637822603658699938581184513";
    for source in [
        Expr::Unit,
        Expr::Boolean { value: true },
        Expr::BytesLiteral { bytes: vec![] },
        Expr::BytesLiteral {
            bytes: vec![0, 1, 255],
        },
        Expr::Default { ty: Type::Unit },
        Expr::KernelSelf { ty: Type::Field },
    ] {
        compare(&source, &HashMap::new());
    }
    for value in [
        "0",
        "1",
        "255",
        "340282366920938463463374607431768211455",
        TWO_128,
        P_MINUS_ONE,
        P,
        "01",
        "-1",
        "１",
    ] {
        compare(
            &Expr::FieldLiteral {
                value: value.into(),
            },
            &HashMap::new(),
        );
    }
    for (value, max) in [
        ("0", "0"),
        ("15", "255"),
        ("256", "255"),
        ("bad", "01"),
        ("00", "255"),
        (
            "340282366920938463463374607431768211455",
            "340282366920938463463374607431768211455",
        ),
        (TWO_128, TWO_128),
        (MAX_248, MAX_248),
        (TWO_248, MAX_248),
        (P_MINUS_ONE, MAX_248),
        ("0", TWO_248),
    ] {
        compare(
            &Expr::UnsignedLiteral {
                value: value.into(),
                max: max.into(),
            },
            &HashMap::new(),
        );
    }
    for ty in [
        Type::Tuple {
            elements: vec![Type::Field; 13],
        },
        Type::Unsigned { max: "01".into() },
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Boolean),
        },
    ] {
        compare(&Expr::Default { ty }, &HashMap::new());
    }
}

#[test]
fn bounded_private_parameter_ownership_and_original_key_lookup_match_old_owner() {
    for ty in [
        Type::Unit,
        Type::Tuple {
            elements: Vec::new(),
        },
        Type::Field,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Tuple {
            elements: vec![Type::Field, Type::Boolean],
        },
        Type::Tuple {
            elements: vec![Type::Field, Type::OpaqueString],
        },
        Type::Vector {
            element: Box::new(Type::Field),
            length: 2,
        },
        Type::Unsigned { max: "bad".into() },
        Type::Struct {
            name: "not a Rust name".into(),
            fields: Vec::new(),
        },
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Boolean),
        },
    ] {
        let params = HashMap::from([("original$source key", (&ty, syn::parse_quote!(argument)))]);
        compare(&reference("original$source key"), &params);
        compare(&reference("original_source key"), &params);
    }
}

#[test]
fn enum_membership_carrier_and_selected_name_order_match_frozen_owner() {
    for (ty, variant) in [
        (Type::Boolean, "A"),
        (
            Type::Enum {
                name: "Choice".into(),
                variants: vec!["A".into(), "unrelated invalid".into()],
            },
            "A",
        ),
        (
            Type::Enum {
                name: "Choice".into(),
                variants: vec!["gen".into()],
            },
            "gen",
        ),
        (
            Type::Enum {
                name: "gen".into(),
                variants: vec!["A".into()],
            },
            "A",
        ),
        (
            Type::Enum {
                name: "not a Rust name".into(),
                variants: vec!["invalid selected".into()],
            },
            "missing",
        ),
        (
            Type::Enum {
                name: "not a Rust name".into(),
                variants: vec!["invalid selected".into()],
            },
            "invalid selected",
        ),
        (
            Type::Enum {
                name: "Choice".into(),
                variants: vec!["invalid selected".into()],
            },
            "invalid selected",
        ),
    ] {
        compare(
            &Expr::EnumVariant {
                ty,
                variant: variant.into(),
            },
            &HashMap::new(),
        );
    }
}

#[test]
#[should_panic(expected = "private pure-expression dispatcher selects this family")]
fn wrong_family_private_adapter_retains_its_internal_misuse_contract() {
    let source = Expr::Add {
        left: Box::new(Expr::FieldLiteral { value: "1".into() }),
        right: Box::new(Expr::FieldLiteral { value: "2".into() }),
    };
    let _ = crate::pure_leaf_expression(&source, &HashMap::new(), &HashMap::new());
}

#[test]
fn private_found_parameter_boundary_is_resource_only_and_explicitly_additive() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut near = Type::Boolean;
            for _ in 0..47 {
                near = Type::Tuple {
                    elements: vec![near],
                };
            }
            let params = HashMap::from([("value", (&near, syn::parse_quote!(argument)))]);
            compare(&reference("value"), &params);
            let over = Type::Tuple {
                elements: vec![near],
            };
            let params = HashMap::from([("value", (&over, syn::parse_quote!(argument)))]);
            // The new private boundary is qualified separately: do not replay the
            // old unchecked clone above the borrowed annotation support limit.
            let current = observe(&reference("value"), &params, &HashMap::new());
            assert!(matches!(
                &current,
                Err(RenderError::ResourceLimit {
                    resource: "syntax_depth",
                    observed: 49,
                    limit: 48
                })
            ));
            let check_params: HashMap<&str, (&Type, ())> = params
                .iter()
                .map(|(name, (ty, _))| (*name, (*ty, ())))
                .collect();
            let checked = fold_leaf(
                LeafSource::Parameter("value"),
                &check_params,
                &mut NoSyntax { carrier: CheckSink },
            )
            .map(|typed| typed.ty);
            assert_eq!(checked, current.map(|(_, _, ty)| ty));
            compare(&reference("missing"), &params);
        })
        .unwrap()
        .join()
        .unwrap();
}
