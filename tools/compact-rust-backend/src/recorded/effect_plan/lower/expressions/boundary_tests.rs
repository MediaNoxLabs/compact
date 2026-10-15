// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Real private lowering boundaries, not physical source authority.
use super::*;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;
mod checks;
mod resources;
mod workers;
#[derive(Default)]
struct Trace {
    primary: RefCell<Option<Option<usize>>>,
    events: RefCell<Vec<Option<usize>>>,
    // Test attachability only: no codec seal or source-range claim.
    available: Option<Vec<usize>>,
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        assert_eq!(site.slot, Slot::Node);
        let event = match site.owner {
            Some(Owner::Expr(value)) => {
                let pointer = std::ptr::from_ref(value) as usize;
                self.available.as_ref().map_or(Some(pointer), |ids| {
                    ids.contains(&pointer).then_some(pointer)
                })
            }
            None => None,
            _ => panic!("boundary invented a Type/action owner"),
        };
        if self.primary.borrow().is_none() {
            *self.primary.borrow_mut() = Some(event);
        }
        self.events.borrow_mut().push(event);
    }
}
fn first(trace: &Trace, source: Option<&Expr>) {
    assert_eq!(
        *trace.primary.borrow(),
        Some(source.map(|value| std::ptr::from_ref(value) as usize))
    );
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    result: Result<Value, RecordingGap>,
    next: usize,
    spent: usize,
    stack: Vec<String>,
    bindings: String,
}
fn lower<'a>(
    declarations: &'a Declarations,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    observer: Option<&'a dyn RefusalObserver>,
    spent: usize,
) -> Lowerer<'a> {
    let (max_call_depth, max_expanded_steps) = crate::resource_limits::recording_call_limits();
    Lowerer {
        observer,
        declarations,
        circuits,
        pure: Certifier::new(pure, circuits),
        call_stack: vec![],
        expanded_steps: spent,
        max_call_depth,
        max_expanded_steps,
        next: 11,
        root_action: None,
        failure_precision: Precision::Unclassified,
        concrete_node: None,
    }
}
fn run(
    source: &Expr,
    scope: &Scope,
    observer: Option<&dyn RefusalObserver>,
    spent: usize,
) -> Outcome {
    let declarations = Declarations::default();
    let pure = HashMap::new();
    let circuits = HashMap::new();
    let mut lower = lower(&declarations, &circuits, &pure, observer, spent);
    let mut bindings = vec![];
    let result = lower.expression(source, scope, &mut bindings, "probe");
    Outcome {
        result,
        next: lower.next,
        spent: lower.expanded_steps,
        stack: lower.call_stack,
        bindings: format!("{bindings:?}"),
    }
}
fn error(source: &Expr, scope: &Scope, trace: &Trace, spent: usize) -> Outcome {
    let observed = run(source, scope, Some(trace), spent);
    assert_eq!(
        observed,
        run(source, scope, None, spent),
        "exact gap/SSA/path/fuel/stack/bindings"
    );
    assert!(observed.result.is_err());
    observed
}
