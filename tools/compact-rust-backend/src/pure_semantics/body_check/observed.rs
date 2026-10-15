//! Optional source-error adapter around the unchanged unit output consumer.
//! It cannot accept/refuse checked facts or construct Rust syntax.
use super::*;
use crate::{RenderError, ir::Expr};

type Callback<'callback> = dyn FnMut(&Expr, &RenderError) + 'callback;
pub(crate) struct ObservedBodyCheckSink<'callback> {
    inner: BodyCheckSink,
    callback: &'callback mut Callback<'callback>,
}
impl<'callback> ObservedBodyCheckSink<'callback> {
    pub(crate) fn new(callback: &'callback mut Callback<'callback>) -> Self {
        Self {
            inner: BodyCheckSink::new(),
            callback,
        }
    }
}
impl LeafSink for ObservedBodyCheckSink<'_> {
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
impl ConversionSink for ObservedBodyCheckSink<'_> {
    fn conversion(&mut self, facts: ConversionFacts, child: ()) {
        self.inner.conversion(facts, child)
    }
}
impl ArithmeticSink for ObservedBodyCheckSink<'_> {
    fn arithmetic(&mut self, facts: ArithmeticFacts, left: (), right: ()) {
        self.inner.arithmetic(facts, left, right)
    }
}
impl CryptoSink for ObservedBodyCheckSink<'_> {
    fn crypto(&mut self, facts: CheckedCrypto<()>) {
        self.inner.crypto(facts)
    }
}
impl AggregateSink for ObservedBodyCheckSink<'_> {
    fn binding(&mut self, name: &CheckedName) {
        self.inner.binding(name)
    }
    fn aggregate(&mut self, facts: CheckedAggregate<(), (), ()>) {
        self.inner.aggregate(facts)
    }
}
impl ControlSink for ObservedBodyCheckSink<'_> {
    fn local_binding<'a>(
        &mut self,
        name: &CheckedName,
        raw: &str,
        occupied: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
        self.inner.local_binding(name, raw, occupied)
    }
    fn control<'a>(
        &mut self,
        facts: CheckedControl<'_, (), (), ()>,
        occupied: impl Iterator<Item = (&'a str, &'a ())> + Clone,
    ) {
        self.inner.control(facts, occupied)
    }
}
impl ExpressionSink for ObservedBodyCheckSink<'_> {
    fn ordinary(&mut self, view: OrdinaryView, value: ()) {
        self.inner.ordinary(view, value)
    }
    fn refused(&mut self, source: &Expr, error: &RenderError) {
        (self.callback)(source, error)
    }
}
