// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Checked family/control facts and complete expression consumer contracts.
//! Production syntax-free certificate admission remains a separate checkpoint.
//! The core owns support decisions; consumers receive checked facts only.
use super::{
    carrier::CarrierSink, identifier::CheckedName, numeric::UnsignedMaximum, ownership::Ownership,
};
use crate::ir::Type;

pub(crate) struct Typed<V> {
    pub(crate) output: V,
    pub(crate) ty: Type,
}

pub(crate) trait LeafSink {
    type Value;
    type Binding;
    type Carriers: CarrierSink;

    fn carriers(&mut self) -> &mut Self::Carriers;
    fn leaf(
        &mut self,
        facts: CheckedLeaf<'_, <Self::Carriers as CarrierSink>::Value, Self::Binding>,
    ) -> Self::Value;
}

pub(crate) enum CheckedLeaf<'a, C, B> {
    Unit,
    Default {
        carrier: C,
    },
    Boolean(bool),
    Field(FieldLiteralFacts),
    Bytes(&'a [u8]),
    Unsigned(UnsignedLiteralFacts<'a>),
    Parameter {
        binding: &'a B,
        ownership: Ownership,
    },
    EnumVariant {
        carrier: C,
        variant: CheckedName,
    },
}

pub(crate) enum FieldLiteralFacts {
    Small(u128),
    CanonicalBytes([u8; 32]),
}

/// Only the semantic leaf owner can mint numeric value/bound facts.
/// Native formatting consumes these immutable getters without parsing again.
pub(crate) struct UnsignedLiteralFacts<'a> {
    maximum: UnsignedMaximum,
    value: &'a str,
    max: &'a str,
    bytes: [u8; 32],
}

impl<'a> UnsignedLiteralFacts<'a> {
    pub(super) fn new(
        maximum: UnsignedMaximum,
        value: &'a str,
        max: &'a str,
        bytes: [u8; 32],
    ) -> Self {
        Self {
            maximum,
            value,
            max,
            bytes,
        }
    }
    pub(crate) fn maximum(&self) -> UnsignedMaximum {
        self.maximum
    }
    pub(crate) fn value(&self) -> &str {
        self.value
    }
    pub(crate) fn max(&self) -> &str {
        self.max
    }
    pub(crate) fn bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

/// Consumers render complete checked conversions; they cannot accept/refuse
/// support. Associated value/binding/carrier types are shared with leaf output.
pub(crate) trait ConversionSink: LeafSink {
    fn conversion(
        &mut self,
        facts: super::conversion::ConversionFacts,
        child: Self::Value,
    ) -> Self::Value;
}

/// Arithmetic support is checked before rendering either already-visited value.
pub(crate) trait ArithmeticSink: LeafSink {
    fn arithmetic(
        &mut self,
        facts: super::arithmetic::ArithmeticFacts,
        left: Self::Value,
        right: Self::Value,
    ) -> Self::Value;
}

/// Crypto consumers receive arity-safe checked operands, never source Types.
pub(crate) trait CryptoSink: LeafSink {
    fn crypto(&mut self, checked: super::crypto::CheckedCrypto<Self::Value>) -> Self::Value;
}

/// Aggregate syntax and lexical symbols consume checked facts only.
pub(crate) trait AggregateSink: LeafSink {
    fn binding(&mut self, name: &CheckedName) -> Self::Binding;
    fn aggregate(
        &mut self,
        checked: super::aggregate::CheckedAggregate<
            Self::Value,
            Self::Binding,
            <Self::Carriers as CarrierSink>::Value,
        >,
    ) -> Self::Value;
}

/// Ordinary Unit views are decided after the family has finished checking.
/// Empty tuple and other Rust-unit carriers do not become Compact Unit here.
pub(crate) enum OrdinaryView {
    Value,
    EmptyUnit,
    ExecutableUnit,
}

pub(crate) trait ControlSink: LeafSink {
    fn local_binding<'a>(
        &mut self,
        name: &CheckedName,
        raw_key: &str,
        occupied: impl Iterator<Item = (&'a str, &'a Self::Binding)> + Clone,
    ) -> Self::Binding
    where
        Self::Binding: 'a;
    fn control<'a>(
        &mut self,
        checked: super::control::CheckedControl<
            '_,
            Self::Value,
            Self::Binding,
            <Self::Carriers as CarrierSink>::Value,
        >,
        occupied: impl Iterator<Item = (&'a str, &'a Self::Binding)> + Clone,
    ) -> Self::Value
    where
        Self::Binding: 'a;
}

/// Complete expression consumer. No family is delegated to a native checker.
pub(crate) trait ExpressionSink:
    LeafSink + ConversionSink + ArithmeticSink + CryptoSink + AggregateSink + ControlSink
{
    fn ordinary(&mut self, view: OrdinaryView, value: Self::Value) -> Self::Value;
    /// Observe a checked failure at this source occurrence. Child failures are
    /// reported before their enclosing owners; consumers cannot replace errors.
    fn refused(&mut self, _source: &crate::ir::Expr, _error: &crate::RenderError) {}
    #[cfg(test)]
    fn observe_source(&mut self, _source: &crate::ir::Expr) {}
}
