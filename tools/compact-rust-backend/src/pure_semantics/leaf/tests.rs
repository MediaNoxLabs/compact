// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! No-syntax leaf consumer and ordered semantic/resource contracts.
use super::*;
use crate::pure_semantics::{
    carrier::{CarrierSink, Leaf},
    sink::CheckedLeaf,
};
use std::collections::HashMap;

#[derive(Default)]
struct CountingCarriers {
    leaves: usize,
}
impl CarrierSink for CountingCarriers {
    type Value = ();
    type Tuple = ();
    fn leaf(&mut self, _: Leaf<'_>) {
        self.leaves += 1;
    }
    fn tuple(&mut self) {}
    fn element(&mut self, _: &mut (), _: ()) {}
    fn finish_tuple(&mut self, _: ()) {}
    fn vector(&mut self, _: (), _: usize) {}
}

#[derive(Default)]
struct CheckLeafSink {
    carriers: CountingCarriers,
    emissions: usize,
}
impl LeafSink for CheckLeafSink {
    type Value = ();
    type Binding = ();
    type Carriers = CountingCarriers;
    fn carriers(&mut self) -> &mut CountingCarriers {
        &mut self.carriers
    }
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {
        self.emissions += 1;
    }
}

fn check(source: LeafSource<'_>) -> Result<Type, RenderError> {
    fold_leaf(
        source,
        &HashMap::<&str, (&Type, ())>::new(),
        &mut CheckLeafSink::default(),
    )
    .map(|value| value.ty)
}
fn unsigned<'a>(value: &'a str, max: &'a str) -> LeafSource<'a> {
    LeafSource::UnsignedLiteral { value, max }
}

#[test]
fn no_syntax_consumer_checks_every_leaf_kind_and_effectful_refusal() {
    let tuple = Type::Tuple {
        elements: vec![Type::Boolean, Type::Field],
    };
    let choice = Type::Enum {
        name: "Choice".into(),
        variants: vec!["gen".into()],
    };
    for (source, expected) in [
        (LeafSource::Unit, Type::Unit),
        (LeafSource::Default(&tuple), tuple.clone()),
        (LeafSource::Boolean(false), Type::Boolean),
        (LeafSource::FieldLiteral("42"), Type::Field),
        (
            LeafSource::BytesLiteral(&[0, 255]),
            Type::Bytes { length: 2 },
        ),
        (unsigned("15", "255"), Type::Unsigned { max: "255".into() }),
        (
            LeafSource::EnumVariant {
                ty: &choice,
                variant: "gen",
            },
            choice.clone(),
        ),
    ] {
        assert_eq!(check(source), Ok(expected));
    }
    let malformed = Type::Unsigned { max: "01".into() };
    let bindings = HashMap::from([("parameter", (&malformed, ()))]);
    assert_eq!(
        fold_leaf(
            LeafSource::Parameter("parameter"),
            &bindings,
            &mut CheckLeafSink::default()
        )
        .unwrap()
        .ty,
        malformed
    );
    assert_eq!(
        check(LeafSource::Effectful),
        Err(RenderError::EffectfulExpression)
    );
}

#[test]
fn carrier_support_is_folded_once_before_emitting_default_and_enum_facts() {
    let ty = Type::Tuple {
        elements: vec![
            Type::Field,
            Type::Vector {
                element: Box::new(Type::Boolean),
                length: 1_000_000,
            },
        ],
    };
    let mut sink = CheckLeafSink::default();
    fold_leaf(LeafSource::Default(&ty), &HashMap::new(), &mut sink).unwrap();
    assert_eq!((sink.carriers.leaves, sink.emissions), (2, 1));
    let choice = Type::Enum {
        name: "Choice".into(),
        variants: vec!["A".into()],
    };
    let mut sink = CheckLeafSink::default();
    fold_leaf(
        LeafSource::EnumVariant {
            ty: &choice,
            variant: "A",
        },
        &HashMap::new(),
        &mut sink,
    )
    .unwrap();
    assert_eq!((sink.carriers.leaves, sink.emissions), (1, 1));
    let mut sink = CheckLeafSink::default();
    assert!(
        fold_leaf(
            LeafSource::EnumVariant {
                ty: &choice,
                variant: "absent"
            },
            &HashMap::new(),
            &mut sink
        )
        .is_err()
    );
    assert_eq!((sink.carriers.leaves, sink.emissions), (0, 0));
}

#[test]
fn unsigned_and_enum_first_errors_precede_later_invalid_inputs() {
    assert_eq!(
        check(unsigned("bad", "01")),
        Err(RenderError::InvalidUnsignedMaximum("01".into()))
    );
    assert_eq!(
        check(unsigned("bad", "255")),
        Err(RenderError::InvalidUnsignedLiteral {
            value: "bad".into(),
            max: "255".into()
        })
    );
    assert_eq!(
        check(unsigned("256", "255")),
        Err(RenderError::InvalidUnsignedLiteral {
            value: "256".into(),
            max: "255".into()
        })
    );
    let bad_name = Type::Enum {
        name: "not a Rust name".into(),
        variants: vec!["also invalid".into()],
    };
    assert_eq!(
        check(LeafSource::EnumVariant {
            ty: &bad_name,
            variant: "absent"
        }),
        Err(RenderError::InvalidEnumVariant("absent".into()))
    );
    assert_eq!(
        check(LeafSource::EnumVariant {
            ty: &bad_name,
            variant: "also invalid"
        }),
        Err(RenderError::InvalidIdentifier("not a Rust name".into()))
    );
    let good_name = Type::Enum {
        name: "Choice".into(),
        variants: vec!["also invalid".into()],
    };
    assert_eq!(
        check(LeafSource::EnumVariant {
            ty: &good_name,
            variant: "also invalid"
        }),
        Err(RenderError::InvalidIdentifier("also invalid".into()))
    );
    assert_eq!(
        check(LeafSource::EnumVariant {
            ty: &Type::Boolean,
            variant: "A"
        }),
        Err(RenderError::InvalidEnumVariant("A".into()))
    );
}

#[test]
fn parameter_lookup_borrows_original_binding_and_preserves_bounded_malformed_annotations() {
    let malformed = Type::Unsigned { max: "bad".into() };
    let bindings = HashMap::from([("not a Rust key", (&malformed, ()))]);
    let (ty, _) = Bindings::get(&bindings, "not a Rust key").unwrap();
    assert!(std::ptr::eq(ty, &malformed));
    assert_eq!(
        fold_leaf(
            LeafSource::Parameter("not a Rust key"),
            &bindings,
            &mut CheckLeafSink::default()
        )
        .unwrap()
        .ty,
        malformed
    );
    assert_eq!(
        fold_leaf(
            LeafSource::Parameter("missing"),
            &bindings,
            &mut CheckLeafSink::default()
        )
        .err(),
        Some(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn ordinary_worker_private_parameter_scan_precedes_recursive_ownership_and_clone() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut near = Type::Boolean;
            for _ in 0..47 {
                near = Type::Tuple {
                    elements: vec![near],
                };
            }
            let bindings = HashMap::from([("near", (&near, ()))]);
            let typed = fold_leaf(
                LeafSource::Parameter("near"),
                &bindings,
                &mut CheckLeafSink::default(),
            )
            .unwrap();
            assert_eq!(typed.ty, near);
            let over = Type::Tuple {
                elements: vec![near],
            };
            let bindings = HashMap::from([("oversized", (&over, ()))]);
            assert!(matches!(
                fold_leaf(
                    LeafSource::Parameter("oversized"),
                    &bindings,
                    &mut CheckLeafSink::default()
                ),
                Err(RenderError::ResourceLimit {
                    resource: "syntax_depth",
                    observed: 49,
                    limit: 48
                })
            ));
            assert_eq!(
                fold_leaf(
                    LeafSource::Parameter("missing"),
                    &bindings,
                    &mut CheckLeafSink::default()
                )
                .err(),
                Some(RenderError::UnknownParameter("missing".into()))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
