// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Actual effect producers and deliberate fences around graph/PureCall gaps.
use super::*;
use crate::ir::{Contract, Expr, Parameter, SCHEMA_VERSION, StateReturn};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (kind, pointer) = match site.owner {
            Some(Owner::Expr(value)) => ("expr", std::ptr::from_ref(value) as usize),
            Some(Owner::Action(value)) => ("action", std::ptr::from_ref(value) as usize),
            _ => ("other", 0),
        };
        self.0.borrow_mut().push((kind, pointer, site.slot));
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    outcome: Result<(), RenderError>,
    statements: String,
    next: usize,
    witness: bool,
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn missing(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn callee(name: &str) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}
fn run(
    action: &StateAction,
    stateful_sources: &[StatefulCircuit],
    witness_sources: &[WitnessDeclaration],
    pure_sources: &[PureCircuit],
    observer: Option<&dyn RefusalObserver>,
) -> Snapshot {
    let stateful = stateful_sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let witnesses = witness_sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let pure = pure_sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let ledger = HashMap::new();
    let parameters = HashMap::new();
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &pure,
        stateful_circuits: &stateful,
        ledger_fields: &ledger,
        observer,
    };
    let mut statements = Vec::new();
    let mut next = 17;
    let mut witness = false;
    let outcome = if observer.is_some() {
        lower_observed(
            action,
            &parameters,
            &environment,
            ActionOutput {
                statements: &mut statements,
                next_temp: &mut next,
                uses_witness: &mut witness,
            },
        )
    } else {
        lower(
            action,
            &parameters,
            &ledger,
            &witnesses,
            &pure,
            &stateful,
            &mut statements,
            &mut next,
            &mut witness,
        )
    };
    Snapshot {
        outcome,
        statements: quote::quote!(#(#statements)*).to_string(),
        next,
        witness,
    }
}
fn same_error(
    action: &StateAction,
    callees: &[StatefulCircuit],
    pure: &[PureCircuit],
    trace: &Trace,
) -> RenderError {
    let expected = run(action, callees, &[], pure, None);
    let observed = run(action, callees, &[], pure, Some(trace));
    assert_eq!(
        observed, expected,
        "partial ordered statements/flags must also match"
    );
    observed
        .outcome
        .expect_err("expected original effect refusal")
}
fn first_expr(trace: &Trace, original: &Expr) {
    assert_eq!(
        trace.0.borrow().first(),
        Some(&("expr", std::ptr::from_ref(original) as usize, Slot::Node,))
    );
}
fn action_name(trace: &Trace, original: &StateAction) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "action",
            std::ptr::from_ref(original) as usize,
            Slot::Field("name"),
        )]
    );
}

#[test]
fn expression_effect_keeps_first_original_child_before_later_refusal() {
    let action = StateAction::Expression {
        value: Expr::Add {
            left: Box::new(field("bad")),
            right: Box::new(missing("later")),
        },
    };
    let StateAction::Expression {
        value: Expr::Add { left, right },
        ..
    } = &action
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &[], &[], &trace),
        RenderError::InvalidFieldLiteral("bad".into())
    );
    first_expr(&trace, left);
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(_, pointer, _)| *pointer != std::ptr::from_ref(right.as_ref()) as usize)
    );
}

#[test]
fn assert_child_and_successful_condition_type_failures_keep_actual_condition() {
    let action = StateAction::Assert {
        condition: missing("first"),
        message: "check".into(),
    };
    let StateAction::Assert { condition, .. } = &action else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &[], &[], &trace),
        RenderError::UnknownParameter("first".into())
    );
    first_expr(&trace, condition);
    let action = StateAction::Assert {
        condition: field("1"),
        message: "check".into(),
    };
    let StateAction::Assert { condition, .. } = &action else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &[], &[], &trace),
        RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        }
    );
    assert_eq!(trace.0.borrow().len(), 1);
    first_expr(&trace, condition);
}

#[test]
fn circuit_lookup_and_arity_remain_before_argument_traversal() {
    let action = StateAction::CircuitCall {
        name: "absent".into(),
        arguments: vec![missing("later")],
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &[], &[], &trace),
        RenderError::UnknownCircuit("absent".into())
    );
    action_name(&trace, &action);
    let sources = [callee("target")];
    let action = StateAction::CircuitCall {
        name: "target".into(),
        arguments: vec![missing("later"), field("1")],
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &sources, &[], &trace),
        RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 2,
        }
    );
    action_name(&trace, &action);
}

#[test]
fn circuit_graph_refusal_precedes_arity_and_does_not_guess_caller_site() {
    let mut source = callee("looping");
    source.actions.push(StateAction::CircuitCall {
        name: "looping".into(),
        arguments: vec![],
    });
    let action = StateAction::CircuitCall {
        name: "looping".into(),
        arguments: vec![],
    };
    let trace = Trace::default();
    let sources = [source];
    assert_eq!(
        same_error(&action, &sources, &[], &trace),
        RenderError::UnsupportedStatefulCall("looping".into())
    );
    action_name(&trace, &sources[0].actions[0]);
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(_, pointer, _)| *pointer != std::ptr::from_ref(&action) as usize),
        "the caller is not the callee's actual closing edge"
    );
}

#[test]
fn circuit_arguments_and_type_check_precede_final_name_and_preserve_partial_output() {
    let sources = [callee("crate")];
    for (argument, expected) in [
        (
            missing("first"),
            RenderError::UnknownParameter("first".into()),
        ),
        (
            Expr::Boolean { value: true },
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
        ),
    ] {
        let action = StateAction::CircuitCall {
            name: "crate".into(),
            arguments: vec![argument],
        };
        let StateAction::CircuitCall { arguments, .. } = &action else {
            panic!("fixture")
        };
        let trace = Trace::default();
        assert_eq!(same_error(&action, &sources, &[], &trace), expected);
        first_expr(&trace, &arguments[0]);
        assert!(
            trace
                .0
                .borrow()
                .iter()
                .all(|(category, _, _)| *category != "action")
        );
    }
    let action = StateAction::CircuitCall {
        name: "crate".into(),
        arguments: vec![field("1")],
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &sources, &[], &trace),
        RenderError::InvalidIdentifier("crate".into())
    );
    action_name(&trace, &action);
    let partial = run(&action, &sources, &[], &[], None);
    assert_eq!(partial.next, 18);
    assert!(partial.statements.contains("__compact_call_argument_17"));
}

#[test]
fn valid_effects_keep_statements_query_context_temporaries_and_witness_flags() {
    let witnesses = [WitnessDeclaration {
        source: None,
        name: "input".into(),
        parameters: vec![],
        result: Type::Field,
    }];
    let witness_value = || Expr::WitnessCall {
        name: "input".into(),
        arguments: vec![],
    };
    let mut target = callee("target");
    target.actions.push(StateAction::Expression {
        value: witness_value(),
    });
    let callees = [target];
    for (action, expected_witness) in [
        (StateAction::Expression { value: field("1") }, false),
        (
            StateAction::Assert {
                condition: Expr::Boolean { value: true },
                message: "check".into(),
            },
            false,
        ),
        (
            StateAction::Expression {
                value: witness_value(),
            },
            true,
        ),
        (
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(witness_value()),
                    right: Box::new(field("1")),
                },
                message: "check".into(),
            },
            true,
        ),
        (
            StateAction::CircuitCall {
                name: "target".into(),
                arguments: vec![field("1")],
            },
            true,
        ),
        (
            StateAction::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            },
            false,
        ),
    ] {
        let trace = Trace::default();
        let observed = run(&action, &callees, &witnesses, &[], Some(&trace));
        assert!(observed.outcome.is_ok());
        assert_eq!(observed, run(&action, &callees, &witnesses, &[], None));
        assert_eq!(observed.witness, expected_witness);
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn borrowed_pure_call_observes_original_argument_after_owner_qualification() {
    let sources = [PureCircuit {
        source: None,
        name: "identity".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "value".into(),
        },
    }];
    let action = StateAction::PureCall {
        name: "identity".into(),
        arguments: vec![missing("first")],
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&action, &[], &sources, &trace),
        RenderError::UnknownParameter("first".into())
    );
    let StateAction::PureCall { arguments, .. } = &action else {
        panic!("fixture")
    };
    first_expr(&trace, &arguments[0]);
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(category, _, _)| *category != "action")
    );
}

#[test]
fn actual_stateful_junction_observes_effect_after_frame_selection_miss() {
    let source = StatefulCircuit {
        source: None,
        name: "effect".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::Expression {
            value: missing("first"),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    let StateAction::Expression { value } = &source.actions[0] else {
        panic!("fixture")
    };
    let expected = crate::stateful::render_stateful_circuit(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .err()
    .expect("native refusal");
    let trace = Trace::default();
    let observed = crate::stateful::render_stateful_circuit_observed(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    )
    .err()
    .expect("native refusal");
    assert_eq!(observed, expected);
    first_expr(&trace, value);
}

#[test]
fn full_effect_actions_keep_actual_default_two_mib_boundaries() {
    fn fixture(depth: usize, shape: usize) -> Contract {
        let mut value = field("1");
        for _ in 0..depth {
            value = Expr::Add {
                left: Box::new(value),
                right: Box::new(field("1")),
            };
        }
        let action = match shape {
            0 => StateAction::Expression { value },
            1 => StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(value),
                    right: Box::new(field("1")),
                },
                message: "check".into(),
            },
            _ => StateAction::CircuitCall {
                name: "target".into(),
                arguments: vec![value],
            },
        };
        let mut circuits = vec![StatefulCircuit {
            source: None,
            name: "run_effect".into(),
            internal: false,
            parameters: vec![],
            actions: vec![action],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }];
        if shape == 2 {
            circuits.push(callee("target"));
        }
        Contract {
            schema_version: SCHEMA_VERSION,
            type_aliases: vec![],
            constructor: None,
            witnesses: vec![],
            circuits: vec![],
            ledger_fields: vec![],
            stateful_circuits: circuits,
        }
    }
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
        for shape in 0..3 {
            let depth = (1..=64).take_while(|depth|
                crate::resource_limits::validate(&fixture(*depth, shape)).is_ok())
                .last().expect("small effect source admitted");
            assert!((3..48).contains(&depth), "unexpected effect shape{shape} depth{depth}");
            let near = fixture(depth, shape);
            let trace = Trace::default();
            let observed = crate::render_with_native_refusal(&near, None, Some(&trace)).unwrap();
            let bare = crate::render_with_capabilities(&near).unwrap();
            assert_eq!(observed.source, bare.source);
            assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(), serde_json::to_value(&bare.capabilities).unwrap());
            assert!(trace.0.borrow().is_empty());
            let over = fixture(depth + 1, shape);
            let observed = crate::render_with_native_refusal(&over, None, Some(&trace)).err().expect("over boundary");
            let bare = crate::render_with_capabilities(&over).err().expect("original over boundary");
            assert!(matches!(observed, RenderError::ResourceLimit { .. }));
            assert_eq!(observed, bare);

            let mut malformed = fixture(depth, shape);
            let action = &mut malformed.stateful_circuits[0].actions[0];
            let value = match action {
                StateAction::Expression { value } => value,
                StateAction::Assert { condition: Expr::Equal { left, .. }, .. } => left.as_mut(),
                StateAction::CircuitCall { arguments, .. } => &mut arguments[0],
                _ => panic!("fixture"),
            };
            let mut leaf = value;
            while let Expr::Add { left, .. } = leaf { leaf = left; }
            *leaf = field("bad");
            let pointer = std::ptr::from_ref(leaf) as usize;
            let trace = Trace::default();
            let observed = crate::render_with_native_refusal(&malformed, None, Some(&trace)).err().expect("malformed leaf");
            let bare = crate::render_with_capabilities(&malformed).err().expect("original malformed leaf");
            assert_eq!(observed, bare);
            assert_eq!(trace.0.borrow().first(), Some(&("expr", pointer, Slot::Node)));
            eprintln!("Full effect shape{shape}: DEFAULT depth{depth} accepted/{} refused, malformed original leaf on ordinary2MiB", depth + 1);
        }
    }).unwrap().join().unwrap();
}
