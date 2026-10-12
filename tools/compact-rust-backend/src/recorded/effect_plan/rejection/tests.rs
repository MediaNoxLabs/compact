// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::ir::*;
use crate::recorded::{LegacyActionPrecision, effect_plan};
use std::collections::HashMap;

fn circuit(name: &str, actions: Vec<StateAction>) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: name != "root",
        parameters: vec![],
        actions,
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn unsupported() -> StateAction {
    StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(field()),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    }
}
fn empty() -> StateAction {
    StateAction::Sequence { actions: vec![] }
}
fn call(name: &str) -> StateAction {
    StateAction::CircuitCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn refusal(circuits: &[StatefulCircuit]) -> LoweringRejection {
    let calls = circuits.iter().map(|c| (c.name.as_str(), c)).collect();
    match effect_plan::check_with_calls_diagnostic(
        &circuits[0],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &calls,
    )
    .unwrap_err()
    {
        DiagnosticPlanError::Unsupported(rejection) => rejection,
        other => panic!("unexpected invariant failure: {other:?}"),
    }
}

#[test]
fn root_ordinal_survives_nested_terminal_sequences_and_helper_actions() {
    let helper = circuit("helper", vec![empty(), empty(), unsupported()]);
    let nested = StateAction::Sequence {
        actions: vec![
            empty(),
            StateAction::Let {
                bindings: vec![],
                action: Box::new(StateAction::Sequence {
                    actions: vec![call("helper")],
                }),
            },
        ],
    };
    let root = circuit("root", vec![empty(), nested]);
    let rejection = refusal(&[root, helper]);
    assert_eq!(rejection.root_action, Some(1));
    assert_eq!(rejection.precision, Precision::ConcreteNode);
    assert_eq!(
        rejection.gap.path,
        "actions[1].actions[1].action.actions[0].callee[helper].actions[2].value"
    );
}

#[test]
fn first_refusal_and_unselected_branch_keep_source_order_and_root_origin() {
    let root = circuit(
        "root",
        vec![
            empty(),
            StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(empty()),
                otherwise: Box::new(unsupported()),
            },
            unsupported(),
        ],
    );
    let rejection = refusal(&[root]);
    assert_eq!(rejection.root_action, Some(1));
    assert_eq!(rejection.precision, Precision::ConcreteNode);
    assert_eq!(rejection.gap.path, "actions[1].otherwise.value");
}

#[test]
fn signature_and_return_failures_cannot_inherit_action_origin() {
    let mut root = circuit("root", vec![unsupported()]);
    root.parameters = vec![
        Parameter {
            name: "x".into(),
            ty: Type::Field
        };
        2
    ];
    let rejection = refusal(&[root]);
    assert_eq!(rejection.root_action, None);
    assert_eq!(rejection.precision, Precision::Unclassified);

    let mut root = circuit("root", vec![empty(), empty()]);
    root.result = Type::Field;
    root.return_value = StateReturn::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(field()),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    };
    let rejection = refusal(&[root]);
    assert_eq!(rejection.root_action, None);
    assert_eq!(rejection.precision, Precision::ConcreteNode);
    assert_eq!(rejection.gap.path, "return_value.value");
}

#[test]
fn generic_lowering_failures_stay_unclassified_and_adapters_preserve_gap() {
    let root = circuit(
        "root",
        vec![
            empty(),
            StateAction::Expression {
                value: Expr::Parameter {
                    name: "missing".into(),
                },
            },
        ],
    );
    let rejection = refusal(std::slice::from_ref(&root));
    assert_eq!(rejection.root_action, Some(1));
    assert_eq!(rejection.precision, Precision::Unclassified);
    let calls = HashMap::from([("root", &root)]);
    let old = match effect_plan::lower_with_calls(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &calls,
    ) {
        Ok(_) => panic!("expected lowering refusal"),
        Err(error) => error,
    };
    let PlanError::Unsupported(gap) = old else {
        panic!("expected unsupported")
    };
    assert_eq!(gap, rejection.gap);
    let invalid: PlanError = DiagnosticPlanError::Invalid(gap.clone()).into();
    assert!(matches!(invalid, PlanError::Invalid(actual) if actual == gap));
}

#[test]
fn refinement_requires_explicit_origin_not_display_strings() {
    let mut rejection = refusal(&[circuit("root", vec![empty(), unsupported()])]);
    let coarse = RecordingGap::action(&unsupported(), "arbitrary display path".into());
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::Coarse, coarse.clone()),
        rejection.gap
    );
    assert_eq!(
        rejection.refine_action(0, LegacyActionPrecision::Coarse, coarse.clone()),
        coarse
    );
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::Concrete, coarse.clone()),
        coarse
    );
    let concrete = RecordingGap::expression(&field(), "unrelated display path".into());
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::Coarse, concrete.clone()),
        concrete
    );
    rejection.precision = Precision::Unclassified;
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::Coarse, coarse.clone()),
        coarse
    );
    rejection.precision = Precision::ConcreteNode;
    rejection.root_action = None;
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::Coarse, coarse.clone()),
        coarse
    );
}
