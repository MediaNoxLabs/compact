// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete current pure leaf-family checks, with generic output consumers.
//! This is not an exhaustive Expr fold or production body CheckSink.
use super::{
    carrier::fold_carrier,
    identifier::check_name,
    numeric::{UnsignedMaximum, field_literal_bytes, unsigned_maximum},
    ownership::{Ownership, copy_type},
    scope::Bindings,
    sink::{CheckedLeaf, FieldLiteralFacts, LeafSink, Typed, UnsignedLiteralFacts},
};
use crate::{RenderError, ir::Type, resource_limits::recording_type_nodes};

pub(crate) enum LeafSource<'a> {
    Unit,
    Default(&'a Type),
    Boolean(bool),
    FieldLiteral(&'a str),
    BytesLiteral(&'a [u8]),
    UnsignedLiteral { value: &'a str, max: &'a str },
    Parameter(&'a str),
    EnumVariant { ty: &'a Type, variant: &'a str },
    Effectful,
}

pub(crate) fn fold_leaf<S: LeafSink>(
    source: LeafSource<'_>,
    bindings: &impl Bindings<S::Binding>,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError> {
    let (facts, ty) = match source {
        LeafSource::Unit => (CheckedLeaf::Unit, Type::Unit),
        LeafSource::Default(ty) => {
            let carrier = fold_carrier(ty, sink.carriers())?;
            (CheckedLeaf::Default { carrier }, ty.clone())
        }
        LeafSource::Boolean(value) => (CheckedLeaf::Boolean(value), Type::Boolean),
        LeafSource::FieldLiteral(value) => {
            let bytes = field_literal_bytes(value)?;
            let facts = match value.parse::<u128>() {
                Ok(small) => FieldLiteralFacts::Small(small),
                Err(_) => FieldLiteralFacts::CanonicalBytes(bytes),
            };
            (CheckedLeaf::Field(facts), Type::Field)
        }
        LeafSource::BytesLiteral(bytes) => (
            CheckedLeaf::Bytes(bytes),
            Type::Bytes {
                length: bytes.len(),
            },
        ),
        LeafSource::UnsignedLiteral { value, max } => (
            CheckedLeaf::Unsigned(unsigned_literal(value, max)?),
            Type::Unsigned {
                max: max.to_owned(),
            },
        ),
        LeafSource::Parameter(name) => {
            let (ty, binding) = bindings
                .get(name)
                .ok_or_else(|| RenderError::UnknownParameter(name.to_owned()))?;
            // Private calls can carry unrelated or oversized annotations. Lookup
            // wins first; resource-only bounds precede ownership and result clone.
            // Bounded malformed annotations retain their prior native behavior.
            drop(recording_type_nodes(ty)?);
            let ownership = if copy_type(ty) {
                Ownership::Copy
            } else {
                Ownership::Owned
            };
            (CheckedLeaf::Parameter { binding, ownership }, ty.clone())
        }
        LeafSource::EnumVariant { ty, variant } => {
            let Type::Enum { variants, .. } = ty else {
                return Err(RenderError::InvalidEnumVariant(variant.to_owned()));
            };
            if !variants.iter().any(|member| member == variant) {
                return Err(RenderError::InvalidEnumVariant(variant.to_owned()));
            }
            let carrier = fold_carrier(ty, sink.carriers())?;
            let variant = check_name(variant)?;
            (CheckedLeaf::EnumVariant { carrier, variant }, ty.clone())
        }
        LeafSource::Effectful => return Err(RenderError::EffectfulExpression),
    };
    Ok(Typed {
        output: sink.leaf(facts),
        ty,
    })
}

fn unsigned_literal<'a>(
    value: &'a str,
    max: &'a str,
) -> Result<UnsignedLiteralFacts<'a>, RenderError> {
    let maximum = unsigned_maximum(max)?;
    let invalid = || RenderError::InvalidUnsignedLiteral {
        value: value.to_owned(),
        max: max.to_owned(),
    };
    let bytes = field_literal_bytes(value).map_err(|_| invalid())?;
    let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
    let high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
    let maximum_limbs = match maximum {
        UnsignedMaximum::Small(maximum) => (0, maximum),
        UnsignedMaximum::Wide { high, low } => (high, low),
    };
    if (high, low) > maximum_limbs {
        return Err(invalid());
    }
    Ok(UnsignedLiteralFacts::new(maximum, value, max, bytes))
}

#[cfg(test)]
mod tests;
