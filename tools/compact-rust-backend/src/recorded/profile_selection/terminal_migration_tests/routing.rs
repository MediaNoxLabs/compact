// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::recorded::admission_probe::trace;

fn selected_trace(contract: &Contract) -> (ProfileSelection, serde_json::Value) {
    let circuit = &contract.stateful_circuits[0];
    let context = Context::new(contract);
    let (selected, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(circuit, false);
        let selected = context.selected(circuit).unwrap();
        // This scope observes selector execution only, not final public capability.
        scope.finish(&Ok::<_, RenderError>(
            crate::recorded::RecordingOutcome::Supported(()),
        ));
        selected
    });
    assert_eq!(rows.as_array().unwrap().len(), 1);
    (selected, rows[0].clone())
}
fn attempts(row: &serde_json::Value) -> Vec<&str> {
    row["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect()
}

#[test]
fn migrated_owner_is_actual_once_and_stops_the_later_chain() {
    let contract = subject();
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    let old = context.old(circuit).unwrap();
    let checked = context.algebra(circuit).unwrap();
    assert_ne!(tokens(&old), tokens(&checked));
    let (selected, row) = selected_trace(&contract);
    assert_eq!(selected_tokens(selected), Some(tokens(&checked)));
    assert_eq!(row["body_owner"], "algebra.terminal-returns");
    let attempted = attempts(&row);
    assert_eq!(
        attempted
            .iter()
            .filter(|&&id| id == "algebra.terminal-returns")
            .count(),
        1
    );
    assert!(!attempted.contains(&"selector.terminal-returns"));
    assert!(!attempted.contains(&"selector.phase-reset"));
    assert!(!attempted.contains(&"selector.unit-actions"));
}

#[test]
fn earlier_point_owner_prevents_terminal_attempt() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap();
    let (selected, row) = selected_trace(&contract);
    assert!(selected.typed_result.is_some());
    assert_eq!(row["body_owner"], "algebra.jubjub-values");
    assert!(!attempts(&row).contains(&"algebra.terminal-returns"));
}

#[test]
fn unsupported_in_domain_body_keeps_independent_later_attempts() {
    let mut contract = subject();
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::TransientHash {
            value: Box::new(parameter("seed")),
        },
    };
    let context = Context::new(&contract);
    assert!(matches!(
        context.algebra(&contract.stateful_circuits[0]),
        Err(effect_plan::PlanError::Unsupported(_))
    ));
    assert!(
        context
            .terminal(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let (_, row) = selected_trace(&contract);
    let attempted = attempts(&row);
    let position = |name| attempted.iter().position(|id| *id == name).unwrap();
    assert!(position("algebra.terminal-returns") < position("selector.phase-reset"));
    assert!(position("selector.phase-reset") < position("selector.unit-actions"));
    assert!(position("selector.unit-actions") < position("selector.unit-composition"));
    assert!(position("selector.unit-composition") < position("selector.zswap"));
    assert!(!attempted.contains(&"selector.terminal-returns"));
}

#[test]
fn later_unit_family_still_wins_outside_declared_field_domain() {
    let mut contract = subject();
    let root = &mut contract.stateful_circuits[0];
    root.parameters.truncate(1);
    root.result = Type::Unit;
    root.actions.clear();
    root.return_value = StateReturn::Expression {
        value: call("store", vec![parameter("seed")]),
    };
    contract.stateful_circuits.push(StatefulCircuit {
        source: None,
        name: "store".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Unit,
        actions: vec![write(parameter("value"))],
        return_value: StateReturn::Unit,
    });
    let context = Context::new(&contract);
    assert!(
        typed_plan::lower_unit_actions(
            &contract.stateful_circuits[0],
            &context.ledger,
            &context.witnesses,
            &context.pure,
            &context.circuits
        )
        .is_some()
    );
    assert!(
        context
            .terminal(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let (selected, row) = selected_trace(&contract);
    assert!(selected.typed_result.is_some());
    assert_eq!(row["body_owner"], "selector.unit-actions");
    let attempted = attempts(&row);
    assert!(attempted.contains(&"algebra.terminal-returns"));
    assert!(!attempted.contains(&"selector.unit-composition"));
}

#[test]
fn declared_rollout_boundary_is_not_the_whole_algebra_domain() {
    for mode in 0..4 {
        let mut contract = subject();
        let circuit = &mut contract.stateful_circuits[0];
        match mode {
            0 => {
                circuit.parameters[0].ty = Type::Unsigned { max: "255".into() };
                circuit.return_value = StateReturn::Expression { value: literal() };
            }
            1 => {
                circuit.result = Type::Boolean;
                circuit.return_value = StateReturn::Expression {
                    value: parameter("selected"),
                };
            }
            2 => {
                circuit.return_value = StateReturn::CellRead {
                    field: "stored".into(),
                    index: 0,
                };
            }
            3 => {
                circuit.actions.clear();
                circuit.return_value = StateReturn::Effectful {
                    body: crate::ir::ReturnPlan::Value { value: literal() },
                };
            }
            _ => unreachable!(),
        }
        let context = Context::new(&contract);
        assert!(context.algebra(&contract.stateful_circuits[0]).is_ok());
        assert!(
            context
                .terminal(&contract.stateful_circuits[0])
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn checked_invariant_is_fatal_and_preserves_the_original_diagnostic() {
    let gap = crate::recorded::RecordingGap {
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.terminal.call".into(),
        detail: "uncertified callee".into(),
    };
    assert!(
        resolve_checked_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    match resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))) {
        Err(RenderError::RecordingPlanInvariant { path, detail }) => {
            assert_eq!(path, "forged.terminal.call");
            assert_eq!(detail, "uncertified callee");
        }
        _ => panic!("invariant must not become a legacy fallback"),
    }
}
