// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::pure_semantics::{
    aggregate::CheckedAggregate,
    arithmetic::ArithmeticFacts,
    body_check::BodyCheckSink,
    carrier::{CarrierSink, CheckSink},
    control::CheckedControl,
    conversion::ConversionFacts,
    crypto::CheckedCrypto,
    expression::fold_expression,
    identifier::CheckedName,
    sink::{
        AggregateSink, ArithmeticSink, CheckedLeaf, ControlSink, ConversionSink, CryptoSink,
        ExpressionSink, LeafSink, OrdinaryView,
    },
};

// Observation only: every output callback forwards checked facts to the actual
// production sink. This contains no source classification or checking rules.
struct Observed {
    inner: BodyCheckSink,
    visits: Vec<*const Expr>,
}
impl LeafSink for Observed {
    type Value = ();
    type Binding = ();
    type Carriers = CheckSink;
    fn carriers(&mut self) -> &mut CheckSink {
        self.inner.carriers()
    }
    fn leaf(&mut self, facts: CheckedLeaf<'_, (), ()>) {
        self.inner.leaf(facts)
    }
}
impl ConversionSink for Observed {
    fn conversion(&mut self, facts: ConversionFacts, child: ()) {
        self.inner.conversion(facts, child)
    }
}
impl ArithmeticSink for Observed {
    fn arithmetic(&mut self, facts: ArithmeticFacts, left: (), right: ()) {
        self.inner.arithmetic(facts, left, right)
    }
}
impl CryptoSink for Observed {
    fn crypto(&mut self, checked: CheckedCrypto<()>) {
        self.inner.crypto(checked)
    }
}
impl AggregateSink for Observed {
    fn binding(&mut self, name: &CheckedName) {
        self.inner.binding(name)
    }
    fn aggregate(&mut self, checked: CheckedAggregate<(), (), ()>) {
        self.inner.aggregate(checked)
    }
}
impl ControlSink for Observed {
    fn local_binding<'a>(
        &mut self,
        name: &CheckedName,
        raw_key: &str,
        occupied: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
        self.inner.local_binding(name, raw_key, occupied)
    }
    fn control<'a>(
        &mut self,
        checked: CheckedControl<'_, (), (), ()>,
        occupied: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
        self.inner.control(checked, occupied)
    }
}
impl ExpressionSink for Observed {
    fn ordinary(&mut self, view: OrdinaryView, value: ()) {
        self.inner.ordinary(view, value)
    }
    fn observe_source(&mut self, source: &Expr) {
        self.visits.push(source as *const Expr);
    }
}

#[test]
fn production_body_sink_has_only_unit_values_bindings_and_carrier_output() {
    fn require_sink<
        S: ExpressionSink + LeafSink<Value = (), Binding = (), Carriers = CheckSink>,
    >() {
    }
    fn require_carrier<S: CarrierSink<Value = (), Tuple = ()>>() {}
    require_sink::<BodyCheckSink>();
    require_carrier::<CheckSink>();
    let pure: HashMap<&str, &PureCircuit> = HashMap::new();
    let scope: HashMap<&str, (&Type, ())> = HashMap::new();
    let checked = fold_expression(&field(), &scope, &pure, &mut BodyCheckSink::new()).unwrap();
    let _: () = checked.output;
    assert_eq!(checked.ty, Type::Field);
}

#[test]
fn equal_arms_check_both_distinct_source_occurrences_once_with_actual_unit_sink() {
    let arm = local("raw$0", Type::Field, field(), reference("raw$0"));
    let expression = branch(boolean(), arm.clone(), arm);
    let Expr::If {
        then, otherwise, ..
    } = &expression
    else {
        unreachable!()
    };
    assert_eq!(then, otherwise);
    let then_address = then.as_ref() as *const Expr;
    let otherwise_address = otherwise.as_ref() as *const Expr;
    assert_ne!(then_address, otherwise_address);
    let mut sink = Observed {
        inner: BodyCheckSink::new(),
        visits: vec![],
    };
    let declarations: HashMap<&str, &PureCircuit> = HashMap::new();
    let checked = fold_expression(&expression, &HashMap::new(), &declarations, &mut sink).unwrap();
    let _: () = checked.output;
    assert_eq!(checked.ty, Type::Field);
    for address in [then_address, otherwise_address] {
        assert_eq!(
            sink.visits
                .iter()
                .filter(|visited| **visited == address)
                .count(),
            1
        );
    }
    for address in &sink.visits {
        assert_eq!(
            sink.visits
                .iter()
                .filter(|visited| *visited == address)
                .count(),
            1
        );
    }
}
