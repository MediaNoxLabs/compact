// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original Action/argument sites at the shared signature-only call boundary.
use super::*;
use crate::ir::{Parameter, PureCircuit, StateReturn, StatefulCircuit};
use crate::source_diagnostics::{RefusalObserver, Slot};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (kind, pointer) = match site.owner.expect("original source") {
            Owner::Action(source) => ("action", std::ptr::from_ref(source) as usize),
            Owner::Expr(source) => ("expr", std::ptr::from_ref(source) as usize),
            _ => panic!("unexpected borrowed-call owner"),
        };
        self.0.borrow_mut().push((kind, pointer, site.slot));
    }
}
fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn missing() -> Expr {
    Expr::Parameter {
        name: "first".into(),
    }
}
fn declaration(name: &str, count: usize, result: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: (0..count)
            .map(|index| Parameter {
                name: format!("p{index}"),
                ty: Type::Field,
            })
            .collect(),
        body: Expr::Default { ty: result.clone() },
        result,
    }
}
fn action(name: &str, arguments: Vec<Expr>) -> StateAction {
    StateAction::PureCall {
        name: name.into(),
        arguments,
    }
}
fn run(
    source: &StateAction,
    declarations: &[PureCircuit],
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Expr, RenderError> {
    let pure = declarations
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let environment = NativeExpressionContext {
        witnesses: &HashMap::new(),
        circuits: &pure,
        stateful_circuits: &HashMap::new(),
        ledger_fields: &HashMap::new(),
        observer,
    };
    let StateAction::PureCall { name, arguments } = source else {
        panic!("fixture")
    };
    render(source, name, arguments, &HashMap::new(), &environment)
}
fn original(source: &StateAction, declarations: &[PureCircuit]) -> Result<syn::Expr, RenderError> {
    let StateAction::PureCall { name, arguments } = source else {
        panic!("fixture")
    };
    // Exact retained old Action projection, test-only: clone before old Expr entry.
    let call = Expr::Call {
        name: name.clone(),
        arguments: arguments.clone(),
    };
    let pure = declarations
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    crate::expression_with_calls(&call, &HashMap::new(), &pure).map(|(expression, _)| expression)
}
fn error(source: &StateAction, declarations: &[PureCircuit], trace: &Trace) -> RenderError {
    let expected = original(source, declarations)
        .err()
        .expect("original refusal");
    let bare = run(source, declarations, None).err().expect("bare refusal");
    let observed = run(source, declarations, Some(trace))
        .err()
        .expect("observed refusal");
    assert_eq!(bare, expected);
    assert_eq!(observed, expected);
    observed
}
fn name_site(trace: &Trace, source: &StateAction) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "action",
            std::ptr::from_ref(source) as usize,
            Slot::Field("name")
        )]
    );
}
fn first_expr(trace: &Trace, source: &Expr) {
    assert_eq!(
        trace.0.borrow().first(),
        Some(&("expr", std::ptr::from_ref(source) as usize, Slot::Node))
    );
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(kind, _, _)| *kind != "action")
    );
}

#[test]
fn lookup_and_arity_use_original_action_name_before_any_argument() {
    let source = action("absent", vec![missing()]);
    let trace = Trace::default();
    assert_eq!(
        error(&source, &[], &trace),
        RenderError::UnknownCircuit("absent".into())
    );
    name_site(&trace, &source);
    let sources = [declaration("helper", 1, Type::Field)];
    let source = action("helper", vec![missing(), literal()]);
    let trace = Trace::default();
    assert_eq!(
        error(&source, &sources, &trace),
        RenderError::ArgumentCount {
            circuit: "helper".into(),
            expected: 1,
            actual: 2
        }
    );
    name_site(&trace, &source);
}

#[test]
fn equal_distinct_argument_occurrences_keep_the_actual_first_failed_leaf() {
    let sources = [declaration("helper", 2, Type::Field)];
    let source = action("helper", vec![missing(), missing()]);
    let StateAction::PureCall { arguments, .. } = &source else {
        panic!("fixture")
    };
    assert_eq!(arguments[0], arguments[1]);
    assert!(!std::ptr::eq(&arguments[0], &arguments[1]));
    let trace = Trace::default();
    assert_eq!(
        error(&source, &sources, &trace),
        RenderError::UnknownParameter("first".into())
    );
    first_expr(&trace, &arguments[0]);
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(_, pointer, _)| *pointer != std::ptr::from_ref(&arguments[1]) as usize)
    );
}

#[test]
fn immediate_argument_mismatch_uses_original_argument_before_later_child_and_name() {
    let sources = [declaration("crate", 2, Type::Field)];
    let source = action("crate", vec![Expr::Boolean { value: true }, missing()]);
    let StateAction::PureCall { arguments, .. } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        error(&source, &sources, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    );
    first_expr(&trace, &arguments[0]);
    assert_eq!(trace.0.borrow().len(), 1);
}

#[test]
fn final_identifier_guard_runs_after_successful_original_arguments() {
    let sources = [declaration("crate", 1, Type::Field)];
    let source = action("crate", vec![literal()]);
    let trace = Trace::default();
    assert_eq!(
        error(&source, &sources, &trace),
        RenderError::InvalidIdentifier("crate".into())
    );
    name_site(&trace, &source);
}

#[test]
fn nested_unknown_same_name_and_zero_argument_calls_never_capture_outer_action() {
    for name in ["absent", "helper"] {
        let sources = [declaration("helper", 1, Type::Field)];
        let source = action(
            "helper",
            vec![Expr::Call {
                name: name.into(),
                arguments: vec![],
            }],
        );
        let StateAction::PureCall { arguments, .. } = &source else {
            panic!("fixture")
        };
        let trace = Trace::default();
        let expected = if name == "absent" {
            RenderError::UnknownCircuit("absent".into())
        } else {
            RenderError::ArgumentCount {
                circuit: "helper".into(),
                expected: 1,
                actual: 0,
            }
        };
        assert_eq!(error(&source, &sources, &trace), expected);
        first_expr(&trace, &arguments[0]);
    }
}

#[test]
fn borrowed_arguments_are_folded_once_without_a_synthetic_source_root() {
    let sources = [declaration("helper", 1, Type::Field)];
    let source = action(
        "helper",
        vec![Expr::Add {
            left: Box::new(literal()),
            right: Box::new(literal()),
        }],
    );
    let StateAction::PureCall { arguments, .. } = &source else {
        panic!("fixture")
    };
    let Expr::Add { left, right } = &arguments[0] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let (result, visits) =
        crate::pure_native::visitation::observe(|| run(&source, &sources, Some(&trace)));
    assert!(result.is_ok());
    assert_eq!(visits.len(), 3);
    for original in [&arguments[0], left.as_ref(), right.as_ref()] {
        assert_eq!(
            visits.get(&(std::ptr::from_ref(original) as usize)),
            Some(&1)
        );
    }
    assert!(trace.0.borrow().is_empty());
}

#[test]
fn value_unit_and_rust_unit_carrier_calls_keep_original_expression_projection() {
    for result in [Type::Field, Type::Unit, Type::Tuple { elements: vec![] }] {
        let sources = [declaration("helper", 0, result)];
        let source = action("helper", vec![]);
        let original = original(&source, &sources).unwrap();
        let trace = Trace::default();
        for observer in [None, Some(&trace as &dyn RefusalObserver)] {
            let actual = run(&source, &sources, observer).unwrap();
            assert_eq!(
                quote::quote!(#actual).to_string(),
                quote::quote!(#original).to_string()
            );
        }
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn ordinary_native_and_unit_body_checker_keep_call_errors_without_action_callbacks() {
    let sources = [
        declaration("helper", 1, Type::Field),
        declaration("crate", 1, Type::Field),
    ];
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let nodes = [
        Expr::Call {
            name: "absent".into(),
            arguments: vec![missing()],
        },
        Expr::Call {
            name: "helper".into(),
            arguments: vec![],
        },
        Expr::Call {
            name: "helper".into(),
            arguments: vec![Expr::Boolean { value: true }],
        },
        Expr::Call {
            name: "crate".into(),
            arguments: vec![literal()],
        },
    ];
    for source in &nodes {
        let native = crate::expression_with_calls(source, &HashMap::new(), &pure)
            .err()
            .expect("native refusal");
        let typed = crate::pure_semantics::expression::fold_expression(
            source,
            &HashMap::new(),
            &pure,
            &mut crate::pure_semantics::body_check::BodyCheckSink::new(),
        )
        .err()
        .expect("typed refusal");
        assert_eq!(typed, native);
    }
}

#[test]
fn actual_stateful_junction_observes_original_pure_action_argument() {
    let helper = declaration("helper", 1, Type::Field);
    let pure = HashMap::from([("helper", &helper)]);
    let source = StatefulCircuit {
        source: None,
        name: "run".into(),
        internal: false,
        parameters: vec![],
        actions: vec![action("helper", vec![missing()])],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    let StateAction::PureCall { arguments, .. } = &source.actions[0] else {
        panic!("fixture")
    };
    let expected = crate::stateful::render_stateful_circuit(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::new(),
    )
    .err()
    .expect("bare refusal");
    let trace = Trace::default();
    let observed = crate::stateful::render_stateful_circuit_observed(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::new(),
        Some(&trace),
    )
    .err()
    .expect("observed refusal");
    assert_eq!(observed, expected);
    assert_eq!(observed, RenderError::UnknownParameter("first".into()));
    first_expr(&trace, &arguments[0]);
}

mod workers;
