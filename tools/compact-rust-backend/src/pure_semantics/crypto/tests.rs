// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Syntax-free consumers and real leaf checks, not a production body fallback.
use super::*;
use crate::{
    ir::Type,
    pure_semantics::{
        carrier::CheckSink,
        leaf::{LeafSource, fold_leaf},
        sink::{CheckedLeaf, LeafSink},
    },
};
use std::collections::HashMap;
mod encoding;
mod ordering;
mod successes;

struct Consumer {
    carrier: CheckSink,
    outputs: Vec<CheckedCrypto<()>>,
}
impl Default for Consumer {
    fn default() -> Self {
        Self {
            carrier: CheckSink,
            outputs: vec![],
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
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {}
}
impl CryptoSink for Consumer {
    fn crypto(&mut self, output: CheckedCrypto<()>) {
        self.outputs.push(output);
    }
}
struct LeafChildren<'a> {
    bindings: HashMap<&'a str, (&'a Type, ())>,
    visits: Vec<usize>,
}
impl Children<Consumer> for LeafChildren<'_> {
    fn visit(&mut self, source: &Expr, sink: &mut Consumer) -> Result<Typed<()>, RenderError> {
        self.visits.push(std::ptr::from_ref(source) as usize);
        let source = match source {
            Expr::Parameter { name } => LeafSource::Parameter(name),
            Expr::Boolean { value } => LeafSource::Boolean(*value),
            Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
            _ => unreachable!("leaf-only test protocol"),
        };
        fold_leaf(source, &self.bindings, sink)
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
