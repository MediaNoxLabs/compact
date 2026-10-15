// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen native output and real no-syntax family outcomes for conversion controls.
//! The no-syntax visitor is an explicit test protocol, not full body certification.
use super::*;
use crate::pure_semantics::{
    carrier::CheckSink,
    children::Children,
    conversion::{ConversionFacts, ConversionSource, fold_conversion},
    leaf::{LeafSource, fold_leaf},
    sink::{CheckedLeaf, ConversionSink, LeafSink, Typed},
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
impl ConversionSink for NoSyntax {
    fn conversion(&mut self, _: ConversionFacts, _: ()) {}
}
struct TestChildren<'a> {
    bindings: HashMap<&'a str, (&'a Type, ())>,
}
impl Children<NoSyntax> for TestChildren<'_> {
    fn visit(&mut self, source: &Expr, sink: &mut NoSyntax) -> Result<Typed<()>, RenderError> {
        if let Some(conversion) = conversion_source(source) {
            return fold_conversion(conversion, self, sink);
        }
        let leaf = match source {
            Expr::Unit => LeafSource::Unit,
            Expr::Default { ty } => LeafSource::Default(ty),
            Expr::Boolean { value } => LeafSource::Boolean(*value),
            Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
            Expr::BytesLiteral { bytes } => LeafSource::BytesLiteral(bytes),
            Expr::UnsignedLiteral { value, max } => LeafSource::UnsignedLiteral { value, max },
            Expr::Parameter { name } => LeafSource::Parameter(name),
            Expr::EnumVariant { ty, variant } => LeafSource::EnumVariant { ty, variant },
            Expr::KernelSelf { .. } => LeafSource::Effectful,
            _ => unreachable!("test protocol contains only conversion and leaf sources"),
        };
        fold_leaf(leaf, &self.bindings, sink)
    }
}
fn conversion_source(source: &Expr) -> Option<ConversionSource<'_>> {
    match source {
        Expr::FieldCast { value } => Some(ConversionSource::FieldCast { value }),
        Expr::FieldToBytes32 { value } => Some(ConversionSource::FieldToBytes32 { value }),
        Expr::Coerce { value, ty } => Some(ConversionSource::Coerce { value, ty }),
        Expr::UnsignedCast { max, value } => Some(ConversionSource::UnsignedCast { max, value }),
        _ => None,
    }
}
fn compare(source: &Expr, parameters: &HashMap<&str, (&Type, syn::Ident)>) {
    let current = observe(source, parameters, &HashMap::new());
    assert_eq!(
        current,
        old_oracle::observe(source, parameters, &HashMap::new()),
        "{source:?}"
    );
    let bindings = parameters
        .iter()
        .map(|(name, (ty, _))| (*name, (*ty, ())))
        .collect();
    let checked = fold_conversion(
        conversion_source(source).unwrap(),
        &mut TestChildren { bindings },
        &mut NoSyntax { carrier: CheckSink },
    )
    .map(|typed| typed.ty);
    assert_eq!(
        checked,
        current.map(|(_, _, ty)| ty),
        "generic consumer {source:?}"
    );
}
fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn tuple(elements: Vec<Type>) -> Type {
    Type::Tuple { elements }
}

#[test]
fn all_cast_carriers_and_field_conversion_outputs_match_frozen_owner() {
    const TWO_128: &str = "340282366920938463463374607431768211456";
    const MAX_248: &str =
        "452312848583266388373324160190187140051835877600158453279131187530910662655";
    for max in ["0", "15", "255", TWO_128, MAX_248, "01", "bad"] {
        let ty = uint(max);
        let parameters = HashMap::from([("original$source", (&ty, syn::parse_quote!(argument)))]);
        compare(
            &Expr::FieldCast {
                value: Box::new(reference("original$source")),
            },
            &parameters,
        );
        for target in ["0", "15", "255", TWO_128, MAX_248, "01", "bad"] {
            compare(
                &Expr::UnsignedCast {
                    max: target.into(),
                    value: Box::new(reference("original$source")),
                },
                &parameters,
            );
        }
    }
    for ty in [Type::Field, Type::Boolean, uint("255")] {
        let parameters = HashMap::from([("value", (&ty, syn::parse_quote!(argument)))]);
        compare(
            &Expr::FieldToBytes32 {
                value: Box::new(reference("value")),
            },
            &parameters,
        );
    }
}

#[test]
fn paired_child_target_and_kind_failures_keep_exact_priority() {
    for source in [
        Expr::UnsignedCast {
            max: "bad".into(),
            value: Box::new(reference("missing")),
        },
        Expr::UnsignedCast {
            max: "255".into(),
            value: Box::new(reference("missing")),
        },
        Expr::UnsignedCast {
            max: "bad".into(),
            value: Box::new(Expr::KernelSelf { ty: Type::Field }),
        },
        Expr::UnsignedCast {
            max: "255".into(),
            value: Box::new(Expr::FieldLiteral { value: "1".into() }),
        },
        Expr::FieldCast {
            value: Box::new(Expr::FieldLiteral { value: "1".into() }),
        },
        Expr::FieldToBytes32 {
            value: Box::new(Expr::Boolean { value: true }),
        },
        Expr::Coerce {
            value: Box::new(reference("missing")),
            ty: uint("bad"),
        },
        Expr::FieldCast {
            value: Box::new(reference("original_source")),
        },
    ] {
        compare(&source, &HashMap::new());
    }
}

#[test]
fn owned_aggregate_and_identity_coercions_reuse_exact_neutral_relation() {
    for (actual, target) in [
        (uint("255"), uint("15")),
        (uint("15"), Type::Field),
        (uint("bad"), uint("bad")),
        (Type::OpaqueString, Type::OpaqueString),
        (tuple(Vec::new()), tuple(Vec::new())),
        (tuple(Vec::new()), vector(Type::Field, 0)),
        (vector(uint("15"), 0), vector(uint("255"), 0)),
        (
            vector(uint("15"), 1_000_000),
            vector(uint("255"), 1_000_000),
        ),
        (
            vector(tuple(vec![uint("15"), Type::OpaqueBytes]), 2),
            vector(tuple(vec![uint("255"), Type::OpaqueBytes]), 2),
        ),
        (tuple(vec![uint("15"), uint("255")]), vector(Type::Field, 2)),
        (vector(uint("15"), 2), tuple(vec![uint("255"), Type::Field])),
        (
            Type::LedgerMap {
                key: Box::new(Type::Field),
                value: Box::new(Type::Boolean),
            },
            Type::LedgerMap {
                key: Box::new(Type::Field),
                value: Box::new(Type::Boolean),
            },
        ),
    ] {
        let parameters = HashMap::from([("value", (&actual, syn::parse_quote!(argument)))]);
        compare(
            &Expr::Coerce {
                value: Box::new(reference("value")),
                ty: target,
            },
            &parameters,
        );
    }
}

#[test]
fn nested_conversion_visits_each_source_child_once_and_preserves_native_views() {
    let source = Expr::FieldToBytes32 {
        value: Box::new(Expr::FieldCast {
            value: Box::new(Expr::UnsignedCast {
                max: "255".into(),
                value: Box::new(Expr::UnsignedLiteral {
                    value: "7".into(),
                    max: "15".into(),
                }),
            }),
        }),
    };
    compare(&source, &HashMap::new());
    let (result, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 4);
    assert!(visits.values().all(|count| *count == 1));
}

#[test]
#[should_panic(expected = "private pure-expression dispatcher selects this family")]
fn wrong_family_private_conversion_adapter_retains_misuse_contract() {
    let _ = crate::pure_conversion_expression(&Expr::Unit, &HashMap::new(), &HashMap::new());
}

mod workers;
