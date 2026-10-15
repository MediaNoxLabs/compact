// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Real syntax-free leaf children, explicit ordering and checked strategy facts.
//! This visitor is a leaf-only test protocol, not a complete body checker.
use super::*;
use crate::pure_semantics::{
    carrier::CheckSink,
    leaf::{LeafSource, fold_leaf},
    sink::{CheckedLeaf, LeafSink},
};
use std::collections::HashMap;

struct Consumer {
    carrier: CheckSink,
    facts: Vec<ArithmeticFacts>,
}
impl Default for Consumer {
    fn default() -> Self {
        Self {
            carrier: CheckSink,
            facts: vec![],
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
impl ArithmeticSink for Consumer {
    fn arithmetic(&mut self, facts: ArithmeticFacts, _: (), _: ()) {
        self.facts.push(facts);
    }
}
struct LeafChildren<'a> {
    bindings: HashMap<&'a str, (&'a Type, ())>,
    visits: Vec<usize>,
}
impl Children<Consumer> for LeafChildren<'_> {
    fn visit(&mut self, source: &Expr, sink: &mut Consumer) -> Result<Typed<()>, RenderError> {
        self.visits.push(std::ptr::from_ref(source) as usize);
        let leaf = match source {
            Expr::Parameter { name } => LeafSource::Parameter(name),
            Expr::Boolean { value } => LeafSource::Boolean(*value),
            Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
            _ => unreachable!("prepared test protocol is leaf-only"),
        };
        // fold_leaf retains selected-type preflight; inferred types meet DEFAULT.
        fold_leaf(leaf, &self.bindings, sink)
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
const WIDE: &str = "340282366920938463463374607431768211456";

mod ordering;
mod strategies;
mod successes;
