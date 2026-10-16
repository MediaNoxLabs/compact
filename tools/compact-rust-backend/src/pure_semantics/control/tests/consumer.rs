// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Syntax-free output callbacks expose checks/visits without duplicating semantics.
use crate::pure_semantics::{
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

pub(super) struct Consumer {
    carrier: CheckSink,
    pub(super) visits: Vec<usize>,
    pub(super) outputs: Vec<&'static str>,
    pub(super) locals: Vec<String>,
}
impl Default for Consumer {
    fn default() -> Self {
        Self {
            carrier: CheckSink,
            visits: vec![],
            outputs: vec![],
            locals: vec![],
        }
    }
}
impl LeafSink for Consumer {
    type Value = ();
    type Binding = ();
    type Carriers = CheckSink;
    fn carriers(&mut self) -> &mut CheckSink {
        &mut self.carrier
    }
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {
        self.outputs.push("leaf");
    }
}
impl ConversionSink for Consumer {
    fn conversion(&mut self, _: ConversionFacts, _: ()) {
        self.outputs.push("conversion");
    }
}
impl ArithmeticSink for Consumer {
    fn arithmetic(&mut self, _: ArithmeticFacts, _: (), _: ()) {
        self.outputs.push("arithmetic");
    }
}
impl CryptoSink for Consumer {
    fn crypto(&mut self, _: CheckedCrypto<()>) {
        self.outputs.push("crypto");
    }
}
impl AggregateSink for Consumer {
    fn binding(&mut self, _: &CheckedName) {}
    fn aggregate(&mut self, _: CheckedAggregate<(), (), ()>) {
        self.outputs.push("aggregate");
    }
}
impl ControlSink for Consumer {
    fn local_binding<'a>(
        &mut self,
        _: &CheckedName,
        raw_key: &str,
        _: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
        self.locals.push(raw_key.into());
    }
    fn control<'a>(
        &mut self,
        _: CheckedControl<'_, (), (), ()>,
        _: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
        self.outputs.push("control");
    }
}
impl ExpressionSink for Consumer {
    fn ordinary(&mut self, _: OrdinaryView, _: ()) {}
    fn observe_source(&mut self, source: &crate::ir::Expr) {
        self.visits.push(std::ptr::from_ref(source) as usize);
    }
}
