//! Actual private certifier producers; no public reachability or physical-range claim.
use super::*;
use crate::ir::{Parameter, StateReturn, Type};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;
mod headers;
mod lifecycle;
mod resources;

type Event = (Option<(&'static str, usize)>, Slot);
#[derive(Default)]
struct Trace {
    first: RefCell<Option<Event>>,
    events: RefCell<Vec<Event>>,
    related: RefCell<Vec<Event>>,
}
fn event(site: RefusalSite<'_>) -> Event {
    (
        site.owner.map(|owner| match owner {
            Owner::Expr(source) => ("expr", std::ptr::from_ref(source) as usize),
            Owner::Pure(source) => ("pure", std::ptr::from_ref(source) as usize),
            Owner::Type(source) => ("type", std::ptr::from_ref(source) as usize),
            Owner::Parameter(source) => ("parameter", std::ptr::from_ref(source) as usize),
            _ => panic!("unexpected certifier owner"),
        }),
        site.slot,
    )
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let event = event(site);
        if self.first.borrow().is_none() {
            *self.first.borrow_mut() = Some(event);
        }
        self.events.borrow_mut().push(event);
    }
    fn related(&self, site: RefusalSite<'_>) {
        self.related.borrow_mut().push(event(site));
    }
}
fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
fn source(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![],
        result: Type::Field,
        body,
    }
}
fn same_error(
    pure: &HashMap<&str, &PureCircuit>,
    stateful: &HashMap<&str, &StatefulCircuit>,
    name: &str,
    trace: &Trace,
) -> RecordingGap {
    let expected = Certifier::new(pure, stateful)
        .certify(name, "root.call")
        .unwrap_err();
    let actual = Certifier::new(pure, stateful)
        .certify_observed(name, "root.call", Some(trace))
        .unwrap_err();
    assert_eq!(actual, expected, "cause/detail/path retained exactly");
    actual
}
fn root() -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "recorded_root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Expression { value: literal() },
    }
}
