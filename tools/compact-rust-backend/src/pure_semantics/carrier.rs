// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Ordinary value carriers: one bounded semantic traversal, two consumers.
//!
//! This owner decides representation support without constructing Rust syntax.
//! Named products check their name only; declaration owners validate their
//! members. Tuple operation traits are separate from value representation.

use super::{
    identifier::{CheckedName, check_name},
    numeric::{UnsignedMaximum, unsigned_maximum},
};
use crate::{RenderError, ir::Type, resource_limits::recording_type_nodes};

pub(crate) enum Leaf<'a> {
    Unit,
    Boolean,
    Field,
    JubjubPoint,
    OpaqueString,
    OpaqueBytes,
    Bytes(usize),
    Named(CheckedName),
    Unsigned {
        spelling: &'a str,
        maximum: UnsignedMaximum,
    },
}

/// Consumers cannot accept or refuse carriers. They receive checked facts and
/// already checked children; only the fold makes semantic decisions.
/// A tuple accumulator lets CheckSink visit children without a child Vec.
pub(crate) trait CarrierSink {
    type Value;
    type Tuple;

    fn leaf(&mut self, leaf: Leaf<'_>) -> Self::Value;
    fn tuple(&mut self) -> Self::Tuple;
    fn element(&mut self, tuple: &mut Self::Tuple, value: Self::Value);
    fn finish_tuple(&mut self, tuple: Self::Tuple) -> Self::Value;
    fn vector(&mut self, element: Self::Value, length: usize) -> Self::Value;
}

/// A real no-syntax consumer, also used by signature certification.
pub(crate) struct CheckSink;

impl CarrierSink for CheckSink {
    type Value = ();
    type Tuple = ();

    fn leaf(&mut self, _: Leaf<'_>) {}
    fn tuple(&mut self) {}
    fn element(&mut self, _: &mut (), _: ()) {}
    fn finish_tuple(&mut self, _: ()) {}
    fn vector(&mut self, _: (), _: usize) {}
}

pub(crate) fn check_carrier(ty: &Type) -> Result<(), RenderError> {
    fold_carrier(ty, &mut CheckSink)
}

/// Preflight the whole borrowed annotation once before cloning a refusal type,
/// checking names/bounds or allocating output. The semantic recursion below
/// does not repeat this scan at each tuple or vector child. The borrowed scan
/// includes named members for resource bounds, without changing their separate
/// declaration semantics; its temporary collection is released before output.
pub(crate) fn fold_carrier<S: CarrierSink>(
    ty: &Type,
    sink: &mut S,
) -> Result<S::Value, RenderError> {
    drop(recording_type_nodes(ty)?);
    fold_bounded(ty, sink)
}

fn fold_bounded<S: CarrierSink>(ty: &Type, sink: &mut S) -> Result<S::Value, RenderError> {
    let leaf = match ty {
        Type::Unit => Leaf::Unit,
        Type::Boolean => Leaf::Boolean,
        Type::Field => Leaf::Field,
        Type::JubjubPoint => Leaf::JubjubPoint,
        Type::OpaqueString => Leaf::OpaqueString,
        Type::OpaqueBytes => Leaf::OpaqueBytes,
        Type::Bytes { length } => Leaf::Bytes(*length),
        Type::Struct { name, .. } | Type::Enum { name, .. } => Leaf::Named(check_name(name)?),
        Type::Unsigned { max } => Leaf::Unsigned {
            spelling: max,
            maximum: unsigned_maximum(max)?,
        },
        Type::Tuple { elements } => {
            let mut tuple = sink.tuple();
            for element in elements {
                let value = fold_bounded(element, sink)?;
                sink.element(&mut tuple, value);
            }
            return Ok(sink.finish_tuple(tuple));
        }
        Type::Vector { element, length } => {
            let element = fold_bounded(element, sink)?;
            return Ok(sink.vector(element, *length));
        }
        Type::LedgerMap { .. } => {
            return Err(RenderError::UnsupportedLedgerValueType(ty.clone()));
        }
    };
    Ok(sink.leaf(leaf))
}

#[cfg(test)]
mod tests;
