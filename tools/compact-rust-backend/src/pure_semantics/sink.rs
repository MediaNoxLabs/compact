// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared leaf facts and consumer contract. Full Expr checking remains pending.
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
