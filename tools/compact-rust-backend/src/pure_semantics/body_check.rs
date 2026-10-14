// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Production body consumer of the complete shared expression fold.
//! Checked values, lexical binding metadata and carrier output are all unit.
//! Callbacks make no support decisions and construct no Rust syntax.
use super::{
    aggregate::CheckedAggregate,
    arithmetic::ArithmeticFacts,
    carrier::CheckSink,
    control::CheckedControl,
    conversion::ConversionFacts,
    crypto::CheckedCrypto,
    identifier::CheckedName,
    sink::{
        AggregateSink, ArithmeticSink, CheckedLeaf, ControlSink, ConversionSink, CryptoSink,
        ExpressionSink, LeafSink, OrdinaryView,
    },
};

pub(crate) struct BodyCheckSink {
    carrier: CheckSink,
}
impl BodyCheckSink {
    pub(crate) fn new() -> Self {
        Self { carrier: CheckSink }
    }
}
impl LeafSink for BodyCheckSink {
    type Value = ();
    type Binding = ();
    type Carriers = CheckSink;
    fn carriers(&mut self) -> &mut CheckSink {
        &mut self.carrier
    }
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {}
}
impl ConversionSink for BodyCheckSink {
    fn conversion(&mut self, _: ConversionFacts, _: ()) {}
}
impl ArithmeticSink for BodyCheckSink {
    fn arithmetic(&mut self, _: ArithmeticFacts, _: (), _: ()) {}
}
impl CryptoSink for BodyCheckSink {
    fn crypto(&mut self, _: CheckedCrypto<()>) {}
}
impl AggregateSink for BodyCheckSink {
    fn binding(&mut self, _: &CheckedName) {}
    fn aggregate(&mut self, _: CheckedAggregate<(), (), ()>) {}
}
impl ControlSink for BodyCheckSink {
    fn local_binding<'a>(
        &mut self,
        _: &CheckedName,
        _: &str,
        _: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
    }
    fn control<'a>(
        &mut self,
        _: CheckedControl<'_, (), (), ()>,
        _: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
    }
}
impl ExpressionSink for BodyCheckSink {
    fn ordinary(&mut self, _: OrdinaryView, _: ()) {}
}

mod observed;
pub(crate) use observed::ObservedBodyCheckSink;
