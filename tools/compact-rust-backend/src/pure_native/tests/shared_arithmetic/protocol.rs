// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only source selection drives real neutral owners without native fallback.
//! This protocol intentionally covers leaves, conversions and arithmetic only.
use super::*;
use crate::pure_semantics::{
    arithmetic::{ArithmeticFacts, ArithmeticSource, fold_arithmetic},
    carrier::CheckSink,
    children::Children,
    conversion::{ConversionFacts, ConversionSource, fold_conversion},
    leaf::{LeafSource, fold_leaf},
    numeric::ArithmeticKind,
    sink::{ArithmeticSink, CheckedLeaf, ConversionSink, LeafSink, Typed},
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
impl ArithmeticSink for NoSyntax {
    fn arithmetic(&mut self, _: ArithmeticFacts, _: (), _: ()) {}
}
struct TestChildren<'a> {
    bindings: HashMap<&'a str, (&'a Type, ())>,
}
impl Children<NoSyntax> for TestChildren<'_> {
    fn visit(&mut self, source: &Expr, sink: &mut NoSyntax) -> Result<Typed<()>, RenderError> {
        if let Some(arithmetic) = arithmetic_source(source) {
            return fold_arithmetic(arithmetic, self, sink);
        }
        let conversion = match source {
            Expr::UnsignedCast { max, value } => {
                Some(ConversionSource::UnsignedCast { max, value })
            }
            Expr::Coerce { value, ty } => Some(ConversionSource::Coerce { value, ty }),
            Expr::FieldCast { value } => Some(ConversionSource::FieldCast { value }),
            Expr::FieldToBytes32 { value } => Some(ConversionSource::FieldToBytes32 { value }),
            _ => None,
        };
        if let Some(conversion) = conversion {
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
            _ => unreachable!("test source protocol: leaf, conversion and arithmetic only"),
        };
        // Real selected-type resource-only preflight supplies bounded Types.
        fold_leaf(leaf, &self.bindings, sink)
    }
}
fn arithmetic_source(source: &Expr) -> Option<ArithmeticSource<'_>> {
    Some(match source {
        Expr::UnsignedAdd { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Add,
            max,
            left,
            right,
        },
        Expr::UnsignedSubtract { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Subtract,
            max,
            left,
            right,
        },
        Expr::UnsignedMultiply { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Multiply,
            max,
            left,
            right,
        },
        Expr::Add { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Add,
            left,
            right,
        },
        Expr::Subtract { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Subtract,
            left,
            right,
        },
        Expr::Multiply { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Multiply,
            left,
            right,
        },
        Expr::Equal { left, right } => ArithmeticSource::Equality {
            negated: false,
            left,
            right,
        },
        Expr::NotEqual { left, right } => ArithmeticSource::Equality {
            negated: true,
            left,
            right,
        },
        Expr::Compare {
            operator,
            left,
            right,
        } => ArithmeticSource::Ordered {
            operator: *operator,
            left,
            right,
        },
        _ => return None,
    })
}

/// Exact frozen native views plus real () consumer agreement on types/refusals.
/// The old body uses retained helpers; their output-only move is independently
/// pinned by the complete old helper source in old_unsigned.rs.
pub(super) fn compare(
    source: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
) -> Result<Type, RenderError> {
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
    let checked = fold_arithmetic(
        arithmetic_source(source).unwrap(),
        &mut TestChildren { bindings },
        &mut NoSyntax { carrier: CheckSink },
    )
    .map(|typed| typed.ty);
    let native = current.map(|(_, _, ty)| ty);
    assert_eq!(checked, native, "generic consumer {source:?}");
    native
}
