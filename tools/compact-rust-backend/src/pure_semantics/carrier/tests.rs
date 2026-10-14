// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{ir::StructField, value_lowering::carrier::NativeCarrierSink};
use quote::ToTokens;

#[path = "reference.rs"]
mod reference;

fn emitted(ty: &Type) -> Result<String, RenderError> {
    fold_carrier(ty, &mut NativeCarrierSink).map(|value| value.into_token_stream().to_string())
}

fn old(ty: &Type) -> Result<String, RenderError> {
    reference::rust_type(ty).map(|value| value.into_token_stream().to_string())
}

fn agree(ty: &Type) {
    assert_eq!(emitted(ty), old(ty), "native carrier changed for {ty:?}");
    assert_eq!(
        check_carrier(ty),
        old(ty).map(|_| ()),
        "check outcome changed for {ty:?}"
    );
}

fn unsigned(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}

fn map() -> Type {
    Type::LedgerMap {
        key: Box::new(Type::Field),
        value: Box::new(Type::Boolean),
    }
}

#[test]
fn every_leaf_and_unsigned_carrier_boundary_matches_frozen_policy() {
    for ty in [
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 32 },
        Type::Bytes { length: usize::MAX },
    ] {
        agree(&ty);
    }
    for max in [
        "0",
        "1",
        "255",
        "340282366920938463463374607431768211455",
        "340282366920938463463374607431768211456",
        "452312848583266388373324160190187140051835877600158453279131187530910662655",
        "452312848583266388373324160190187140051835877600158453279131187530910662656",
        "",
        "00",
        "-1",
        " 1",
        "1u128",
        "１２",
    ] {
        agree(&unsigned(max));
    }
}

#[test]
fn aggregates_preserve_order_commas_and_vector_lengths() {
    let leaves = [
        Type::Unit,
        Type::Boolean,
        Type::Field,
        unsigned("255"),
        Type::OpaqueString,
        Type::Bytes { length: 8 },
    ];
    for count in 0..=14 {
        agree(&Type::Tuple {
            elements: (0..count)
                .map(|i| leaves[i % leaves.len()].clone())
                .collect(),
        });
    }
    for length in [0, 1, 7, usize::MAX] {
        agree(&Type::Vector {
            element: Box::new(Type::Tuple {
                elements: leaves.to_vec(),
            }),
            length,
        });
    }
    let mut nested = Type::Boolean;
    for _ in 0..10 {
        nested = Type::Tuple {
            elements: vec![
                Type::Field,
                Type::Vector {
                    element: Box::new(nested),
                    length: 3,
                },
            ],
        };
        agree(&nested);
    }
}

#[test]
fn named_carriers_validate_only_names_not_member_semantics() {
    for name in [
        "Example",
        "match",
        "r#match",
        "a$b",
        "λ",
        " /* ordinary */ Example ",
        "",
        "_",
        "not a name",
        "self",
    ] {
        agree(&Type::Struct {
            name: name.into(),
            fields: vec![
                StructField {
                    name: "invalid field!".into(),
                    ty: unsigned("bad"),
                },
                StructField {
                    name: "map".into(),
                    ty: map(),
                },
            ],
        });
        agree(&Type::Enum {
            name: name.into(),
            variants: vec!["invalid variant!".into(), "".into()],
        });
    }
}

#[test]
fn invalid_children_and_map_refusals_keep_declared_order() {
    let invalid_name = Type::Struct {
        name: "bad name".into(),
        fields: vec![],
    };
    let invalid_uint = unsigned("not decimal");
    for first in [invalid_name.clone(), invalid_uint.clone(), map()] {
        for second in [invalid_name.clone(), invalid_uint.clone(), map()] {
            let ty = Type::Tuple {
                elements: vec![first.clone(), second],
            };
            agree(&ty);
            assert_eq!(check_carrier(&ty), check_carrier(&first));
        }
    }
    agree(&Type::Vector {
        element: Box::new(invalid_uint),
        length: 0,
    });
    agree(&Type::LedgerMap {
        key: Box::new(unsigned("bad")),
        value: Box::new(invalid_name),
    });
}

struct NoOutput;
impl CarrierSink for NoOutput {
    type Value = ();
    type Tuple = ();
    fn leaf(&mut self, _: Leaf<'_>) {
        panic!("output before preflight")
    }
    fn tuple(&mut self) {
        panic!("output before preflight")
    }
    fn element(&mut self, _: &mut (), _: ()) {
        panic!("output before preflight")
    }
    fn finish_tuple(&mut self, _: ()) {
        panic!("output before preflight")
    }
    fn vector(&mut self, _: (), _: usize) {
        panic!("output before preflight")
    }
}

#[test]
fn whole_annotation_is_bounded_before_sink_or_owned_refusal() {
    let mut deep = map();
    for _ in 0..60 {
        deep = Type::Vector {
            element: Box::new(deep),
            length: 0,
        };
    }
    assert!(matches!(
        fold_carrier(&deep, &mut NoOutput),
        Err(RenderError::ResourceLimit { .. })
    ));
    let wide = Type::Tuple {
        elements: vec![Type::Boolean; 5000],
    };
    assert!(matches!(
        fold_carrier(&wide, &mut NoOutput),
        Err(RenderError::ResourceLimit { .. })
    ));
    // Named members remain semantically independent but cannot evade budgets.
    let named = Type::Struct {
        name: "bad name".into(),
        fields: vec![StructField {
            name: "hidden".into(),
            ty: deep,
        }],
    };
    assert!(matches!(
        fold_carrier(&named, &mut NoOutput),
        Err(RenderError::ResourceLimit { .. })
    ));
}

#[test]
fn check_sink_owns_no_output_accumulator_or_vector_expansion() {
    assert_eq!(std::mem::size_of::<CheckSink>(), 0);
    assert_eq!(std::mem::size_of::<<CheckSink as CarrierSink>::Value>(), 0);
    assert_eq!(std::mem::size_of::<<CheckSink as CarrierSink>::Tuple>(), 0);
    assert_eq!(
        check_carrier(&Type::Vector {
            element: Box::new(Type::Boolean),
            length: usize::MAX
        }),
        Ok(())
    );
    assert_eq!(
        check_carrier(&Type::Tuple {
            elements: vec![Type::Field; 13]
        }),
        Ok(())
    );
}

/// The optional observer follows the same bounded carrier recursion on the
/// ordinary worker stack. The whole-type guard still wins before any sink.
#[test]
fn observed_carrier_keeps_default_boundary_on_two_mib_worker() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot};
            use std::cell::Cell;
            struct Capture<'a> {
                leaf: &'a Type,
                calls: Cell<usize>,
            }
            impl RefusalObserver for Capture<'_> {
                fn refused(&self, site: RefusalSite<'_>) {
                    assert_eq!(site.owner, Some(Owner::Type(self.leaf)));
                    assert_eq!(site.slot, Slot::Field("max"));
                    self.calls.set(self.calls.get() + 1);
                }
            }
            fn nested(depth: usize, leaf: Type) -> Type {
                (0..depth).fold(leaf, |ty, _| Type::Vector {
                    element: Box::new(ty),
                    length: 1,
                })
            }
            let valid = nested(47, Type::Field);
            let bad = nested(46, unsigned("bad"));
            let mut leaf = &bad;
            for _ in 0..46 {
                let Type::Vector { element, .. } = leaf else {
                    unreachable!()
                };
                leaf = element;
            }
            let capture = Capture {
                leaf,
                calls: Cell::new(0),
            };
            let plain = fold_carrier(&valid, &mut NativeCarrierSink)
                .unwrap()
                .into_token_stream()
                .to_string();
            let observed = fold_carrier_observed(&valid, &mut NativeCarrierSink, Some(&capture))
                .unwrap()
                .into_token_stream()
                .to_string();
            assert_eq!(observed, plain);
            assert_eq!(capture.calls.get(), 0);
            assert_eq!(
                fold_carrier_observed(&bad, &mut CheckSink, Some(&capture)),
                check_carrier(&bad)
            );
            assert_eq!(capture.calls.get(), 1);
            let over = nested(48, unsigned("bad"));
            assert!(matches!(
                fold_carrier_observed(&over, &mut NoOutput, Some(&capture)),
                Err(RenderError::ResourceLimit { .. })
            ));
            assert_eq!(
                capture.calls.get(),
                1,
                "unavailable preflight must not invent a leaf"
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
