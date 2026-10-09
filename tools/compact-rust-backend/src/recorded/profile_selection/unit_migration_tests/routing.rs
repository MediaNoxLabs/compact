// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::recorded::admission_probe::trace;

fn selected_trace(contract: &Contract) -> (ProfileSelection, serde_json::Value) {
    let root = &contract.stateful_circuits[0];
    let context = Context::new(contract);
    let (selected, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(root, false);
        let selected = context.selected(root).unwrap();
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
fn actual_unit_owner_runs_once_without_old_shape_or_later_profiles() {
    let contract = subject();
    let context = Context::new(&contract);
    let root = &contract.stateful_circuits[0];
    let old = context.old(root).unwrap();
    let algebra = context.algebra(root).unwrap();
    assert_ne!(tokens(&old), tokens(&algebra));
    let (selected, row) = selected_trace(&contract);
    let (steps, result) = (
        selected.organizer_steps.unwrap(),
        selected.typed_result.unwrap(),
    );
    assert_eq!(quote!(#(#steps)* #result).to_string(), tokens(&algebra));
    assert_eq!(row["body_owner"], "algebra.unit-actions");
    let attempted = attempts(&row);
    assert_eq!(
        attempted
            .iter()
            .filter(|&&r| r == "algebra.unit-actions")
            .count(),
        1
    );
    assert!(!attempted.contains(&"selector.unit-actions"));
    assert!(!attempted.contains(&"selector.unit-composition"));
}

#[test]
fn earlier_terminal_owner_keeps_its_priority() {
    let mut contract = subject();
    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: parameter("seed"),
    };
    let (_, row) = selected_trace(&contract);
    assert_eq!(row["body_owner"], "algebra.terminal-returns");
    assert!(!attempts(&row).contains(&"algebra.unit-actions"));
}

#[test]
fn unsupported_unit_body_continues_independent_suffix_and_invalid_is_fatal() {
    let mut contract = subject();
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Sequence {
            steps: vec![Expr::Unit],
            value: Box::new(Expr::TransientHash {
                value: Box::new(parameter("seed")),
            }),
        },
    };
    let context = Context::new(&contract);
    assert!(
        context
            .unit(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let (_, row) = selected_trace(&contract);
    let attempted = attempts(&row);
    let at = |name| attempted.iter().position(|r| *r == name).unwrap();
    assert!(at("algebra.unit-actions") < at("selector.unit-composition"));
    assert!(at("selector.unit-composition") < at("selector.zswap"));
    assert!(!attempted.contains(&"selector.unit-actions"));
    let gap = crate::recorded::RecordingGap {
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.unit.member".into(),
        detail: "inconsistent member definition".into(),
    };
    assert!(
        resolve_checked_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    assert!(
        matches!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))), Err(RenderError::RecordingPlanInvariant { path, detail }) if path == "forged.unit.member" && detail == "inconsistent member definition")
    );
}

#[test]
fn declared_unit_rollout_is_narrower_than_checked_operation_support() {
    for mode in 0..3 {
        let mut contract = subject();
        let root = &mut contract.stateful_circuits[0];
        match mode {
            0 => {
                root.parameters[0].ty = Type::Unsigned { max: "255".into() };
                root.return_value = StateReturn::Expression { value: Expr::Unit };
            }
            1 => {
                root.actions = vec![write("amount", 0, field())];
                root.return_value = StateReturn::Unit;
            }
            2 => {
                root.return_value = StateReturn::Effectful {
                    body: crate::ir::ReturnPlan::Value { value: Expr::Unit },
                }
            }
            _ => unreachable!(),
        }
        let context = Context::new(&contract);
        assert!(context.algebra(&contract.stateful_circuits[0]).is_ok());
        assert!(
            context
                .unit(&contract.stateful_circuits[0])
                .unwrap()
                .is_none()
        );
    }
}
