// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Only actual stateful callee-body failures add incoming call occurrences.
use super::*;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

type Site = (&'static str, usize, Slot);
#[derive(Default)]
struct Trace {
    primary: RefCell<Option<Site>>,
    related: RefCell<Vec<Site>>,
    events: RefCell<usize>,
}
fn site(source: RefusalSite<'_>) -> Site {
    let (kind, pointer) = match source.owner.expect("original occurrence") {
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected source in these scalar call fixtures"),
    };
    (kind, pointer, source.slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, source: RefusalSite<'_>) {
        let site = site(source);
        if self.primary.borrow().is_none() {
            *self.primary.borrow_mut() = Some(site);
        }
        *self.events.borrow_mut() += 1;
    }
    fn related(&self, source: RefusalSite<'_>) {
        self.related.borrow_mut().push(site(source));
    }
}
fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn refusal() -> Expr {
    Expr::PersistentCommit {
        value: Box::new(Expr::Default {
            ty: Type::OpaqueString,
        }),
        opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
    }
}
fn circuit(name: &str, parameters: &[&str], value: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: name != "root",
        parameters: parameters
            .iter()
            .map(|name| Parameter {
                name: (*name).into(),
                ty: Type::Field,
            })
            .collect(),
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value },
    }
}
fn value(source: &StatefulCircuit) -> &Expr {
    let StateReturn::Expression { value } = &source.return_value else {
        panic!("fixture")
    };
    value
}
fn expr_site(value: &Expr) -> Site {
    ("expr", std::ptr::from_ref(value) as usize, Slot::Node)
}
fn action_site(value: &StateAction) -> Site {
    ("action", std::ptr::from_ref(value) as usize, Slot::Node)
}
fn run(
    sources: &[StatefulCircuit],
    root: usize,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(String, String), RecordingGap> {
    let calls = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let pure = HashMap::new();
    let declarations = Declarations::with_calls(&HashMap::new(), &HashMap::new(), &pure, &calls)
        .with_root(&sources[root]);
    let (plan, certificates) = super::super::lower_with_pure_calls_observed(
        &sources[root],
        &declarations,
        &pure,
        &calls,
        observer,
    )
    .map_err(LoweringRejection::into_checked_gap)?;
    let declarations = declarations.with_certificates(certificates, &pure)?;
    let checked = super::super::super::validate::check(plan, declarations)?;
    let rendered = super::super::super::render(&checked)?;
    let steps = rendered.steps;
    let result = rendered.result;
    Ok((
        format!("{:?}", checked.plan()),
        quote::quote!(#(#steps)* #result).to_string(),
    ))
}
fn same_error(sources: &[StatefulCircuit], root: usize, trace: &Trace) -> RecordingGap {
    let expected = run(sources, root, None).unwrap_err();
    let observed = run(sources, root, Some(trace)).unwrap_err();
    assert_eq!(observed, expected);
    observed
}

#[test]
fn nested_callee_body_keeps_original_primary_and_actual_inner_to_outer_calls() {
    let ty = Type::Bytes { length: 32 };
    let sources = [
        circuit("root", &[], call("mid", vec![]), ty.clone()),
        circuit("mid", &[], call("bad", vec![]), ty.clone()),
        circuit("bad", &[], refusal(), ty),
    ];
    let trace = Trace::default();
    let error = same_error(&sources, 0, &trace);
    assert!(error.detail.contains("persistent commitment requires"));
    assert!(error.path.contains(".callee[mid]"));
    assert!(error.path.contains(".callee[bad]"));
    assert_eq!(*trace.primary.borrow(), Some(expr_site(value(&sources[2]))));
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[expr_site(value(&sources[1])), expr_site(value(&sources[0]))]
    );
}

#[test]
fn statement_body_joins_use_original_actions_not_synthetic_call_expressions() {
    let mut root = circuit("root", &[], Expr::Unit, Type::Unit);
    root.return_value = StateReturn::Unit;
    root.actions = vec![StateAction::CircuitCall {
        name: "mid".into(),
        arguments: vec![],
    }];
    let mut mid = circuit("mid", &[], Expr::Unit, Type::Unit);
    mid.return_value = StateReturn::Unit;
    mid.actions = vec![StateAction::CircuitCall {
        name: "bad".into(),
        arguments: vec![],
    }];
    let mut bad = circuit("bad", &[], Expr::Unit, Type::Unit);
    bad.return_value = StateReturn::Unit;
    bad.actions = vec![StateAction::Expression { value: refusal() }];
    let sources = [root, mid, bad];
    let StateAction::Expression { value: failed } = &sources[2].actions[0] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    same_error(&sources, 0, &trace);
    assert_eq!(*trace.primary.borrow(), Some(expr_site(failed)));
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[
            action_site(&sources[1].actions[0]),
            action_site(&sources[0].actions[0])
        ]
    );
}

#[test]
fn equal_distinct_call_occurrences_only_join_the_first_failing_branch() {
    let ty = Type::Bytes { length: 32 };
    let root = circuit(
        "root",
        &[],
        Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(call("bad", vec![])),
            otherwise: Box::new(call("bad", vec![])),
        },
        ty.clone(),
    );
    let sources = [root, circuit("bad", &[], refusal(), ty)];
    let Expr::If {
        then, otherwise, ..
    } = value(&sources[0])
    else {
        panic!("fixture")
    };
    assert_eq!(then, otherwise);
    assert!(!std::ptr::eq(then.as_ref(), otherwise.as_ref()));
    let trace = Trace::default();
    same_error(&sources, 0, &trace);
    assert_eq!(trace.related.borrow().as_slice(), &[expr_site(then)]);
    assert_eq!(*trace.primary.borrow(), Some(expr_site(value(&sources[1]))));
}

#[test]
fn argument_and_earlier_call_guards_are_not_relabelled_as_callee_body_failures() {
    let ty = Type::Bytes { length: 32 };
    let sources = [
        circuit("root", &[], call("bad", vec![refusal()]), ty.clone()),
        circuit("bad", &["input"], refusal(), ty.clone()),
    ];
    let Expr::Call { arguments, .. } = value(&sources[0]) else {
        panic!("fixture")
    };
    let trace = Trace::default();
    same_error(&sources, 0, &trace);
    assert_eq!(*trace.primary.borrow(), Some(expr_site(&arguments[0])));
    assert!(trace.related.borrow().is_empty());

    for root in [
        circuit("root", &[], call("absent", vec![refusal()]), ty.clone()),
        circuit("root", &[], call("bad", vec![]), ty.clone()),
        circuit("root", &[], call("root", vec![]), ty.clone()),
    ] {
        let trace = Trace::default();
        same_error(
            &[root, circuit("bad", &["input"], refusal(), ty.clone())],
            0,
            &trace,
        );
        assert!(
            trace.related.borrow().is_empty(),
            "no callee body was entered"
        );
    }
}

#[test]
fn successful_nested_calls_keep_complete_ssa_paths_and_rendered_syntax() {
    let sources = [
        circuit("root", &[], call("mid", vec![literal()]), Type::Field),
        circuit(
            "mid",
            &["input"],
            call("leaf", vec![parameter("input")]),
            Type::Field,
        ),
        circuit(
            "leaf",
            &["value"],
            Expr::Add {
                left: Box::new(parameter("value")),
                right: Box::new(parameter("value")),
            },
            Type::Field,
        ),
    ];
    let trace = Trace::default();
    assert_eq!(
        run(&sources, 0, Some(&trace)).unwrap(),
        run(&sources, 0, None).unwrap()
    );
    assert!(trace.primary.borrow().is_none());
    assert!(trace.related.borrow().is_empty());
    assert_eq!(*trace.events.borrow(), 0);
}

#[test]
fn independent_roots_do_not_reemit_stale_call_edges_into_later_success_or_failure() {
    // The caller/Recorder owns begin/finish/reset. Lowerer creates a fresh call
    // stack per root; reuse this observer to detect any phantom appended edge.
    let ty = Type::Bytes { length: 32 };
    let sources = [
        circuit("root", &[], call("bad", vec![]), ty.clone()),
        circuit(
            "supported",
            &[],
            Expr::BytesLiteral { bytes: vec![0; 32] },
            ty.clone(),
        ),
        circuit("later", &[], call("bad", vec![]), ty.clone()),
        circuit("bad", &[], refusal(), ty),
    ];
    let trace = Trace::default();
    same_error(&sources, 0, &trace);
    let before = trace.related.borrow().clone();
    let events = *trace.events.borrow();
    assert_eq!(
        run(&sources, 1, Some(&trace)).unwrap(),
        run(&sources, 1, None).unwrap()
    );
    assert_eq!(*trace.related.borrow(), before);
    assert_eq!(*trace.events.borrow(), events);
    same_error(&sources, 2, &trace);
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[expr_site(value(&sources[0])), expr_site(value(&sources[2]))]
    );
}
