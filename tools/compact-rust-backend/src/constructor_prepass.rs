// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Direct prepass controls. No parser ranges or new public cycle route are claimed.
use crate::ir::{Constructor, Contract, LocalBinding, Parameter, SCHEMA_VERSION, StateReturn};
use crate::source_diagnostics::Slot;
use crate::*;
use std::cell::RefCell;

mod graph;
mod ordering;

type Event = (Option<(&'static str, usize)>, Slot);
#[derive(Default)]
struct Trace(RefCell<Vec<Event>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let owner = match site.owner {
            Some(Owner::Expr(value)) => Some(("expr", std::ptr::from_ref(value) as usize)),
            Some(Owner::Action(value)) => Some(("action", std::ptr::from_ref(value) as usize)),
            None => None,
            _ => panic!("prepass must not invent a constructor/name parent"),
        };
        self.0.borrow_mut().push((owner, site.slot));
    }
    fn related(&self, _: RefusalSite<'_>) {
        panic!("prepass has no related-edge producer");
    }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
fn witness() -> Expr {
    Expr::WitnessCall {
        name: "direct".into(),
        arguments: vec![],
    }
}
fn expression(value: Expr) -> ConstructorStep {
    ConstructorStep::Expression { value }
}
fn circuit(name: &str, actions: Vec<StateAction>, return_value: StateReturn) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: false,
        parameters: vec![],
        actions,
        result: Type::Unit,
        return_value,
    }
}
fn loop_action(name: &str) -> StatefulCircuit {
    circuit(
        name,
        vec![StateAction::CircuitCall {
            name: name.into(),
            arguments: vec![],
        }],
        StateReturn::Unit,
    )
}
fn same(
    step: &ConstructorStep,
    sources: &HashMap<&str, &StatefulCircuit>,
    trace: &Trace,
) -> Result<bool, RenderError> {
    // Both old owner bodies are retained byte-exact under cfg(test); the common
    // graph helpers are unchanged H176 premises, not a second new checker.
    let old = constructor_step_uses_witness(step, sources);
    let bare = constructor_step_uses_witness_observed(step, sources, None);
    let observed = constructor_step_uses_witness_observed(step, sources, Some(trace));
    assert_eq!(bare, old);
    assert_eq!(observed, old);
    observed
}
fn action_site(trace: &Trace, source: &StateAction) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            Some(("action", std::ptr::from_ref(source) as usize)),
            Slot::Field("name")
        )]
    );
}
fn expr_site(trace: &Trace, source: &Expr) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            Some(("expr", std::ptr::from_ref(source) as usize)),
            Slot::Field("name")
        )]
    );
}
fn contract(steps: Vec<ConstructorStep>, stateful_circuits: Vec<StatefulCircuit>) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: Some(Constructor {
            source: None,
            parameters: vec![],
            steps,
        }),
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits,
    }
}
