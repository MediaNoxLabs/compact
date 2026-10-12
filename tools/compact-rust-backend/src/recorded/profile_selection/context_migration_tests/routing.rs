// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// The direct VectorMap child is a native-valid recording refusal; fixed Field
// commitments themselves are supported. Do not replace this with malformed types.
use super::*;
use crate::recorded::admission_probe::trace;
fn trace(contract: &Contract) -> serde_json::Value {
    let root = &contract.stateful_circuits[0];
    let context = Context::new(contract);
    let (_, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(root, false);
        let selected = context.selected(root).unwrap();
        scope.finish(&Ok::<_, RenderError>(
            crate::recorded::RecordingOutcome::Supported(()),
        ));
        selected
    });
    assert_eq!(rows.as_array().unwrap().len(), 1);
    rows[0].clone()
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
fn context_owner_runs_once_and_stops_later_profiles() {
    let row = trace(&subject(commit(project(address()), bytes(), bytes())));
    assert_eq!(row["body_owner"], "algebra.context-query");
    let attempted = attempts(&row);
    assert_eq!(
        attempted
            .iter()
            .filter(|&&r| r == "algebra.context-query")
            .count(),
        1
    );
    assert!(!attempted.contains(&"selector.context-query"));
    assert!(!attempted.contains(&"algebra.composite-values"));
    assert!(!attempted.contains(&"selector.composite-values"));
}
#[test]
fn earlier_point_route_keeps_priority() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap();
    let row = trace(&contract);
    assert_eq!(row["body_owner"], "algebra.jubjub-values");
    assert!(!attempts(&row).contains(&"algebra.context-query"));
}
#[test]
fn unsupported_continues_independent_suffix_but_invariant_is_fatal() {
    let contract = subject(Expr::FieldToBytes32 {
        value: Box::new(Expr::JubjubPointX {
            value: Box::new(Expr::HashToCurve {
                value: Box::new(Expr::PersistentCommit {
                    value: Box::new(crate::recorded::test_support::unsupported_fixed_input()),
                    opening: Box::new(Expr::FieldToBytes32 {
                        value: Box::new(Expr::FieldLiteral { value: "1".into() }),
                    }),
                }),
            }),
        }),
    });
    assert!(
        Context::new(&contract)
            .domain(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let row = trace(&contract);
    let attempted = attempts(&row);
    let position = |name| attempted.iter().position(|r| *r == name).unwrap();
    assert!(position("algebra.context-query") < position("algebra.composite-values"));
    assert!(!attempted.contains(&"selector.context-query"));
    let gap = crate::recorded::RecordingGap {
        resource: None,
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.context".into(),
        detail: "wrong address result".into(),
    };
    assert!(
        matches!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))),Err(RenderError::RecordingPlanInvariant {path,detail}) if path=="forged.context" && detail=="wrong address result")
    );
}
#[test]
fn no_observation_and_root_actions_are_separate_expansions() {
    for with_action in [false, true] {
        let mut contract = subject(bytes());
        if with_action {
            contract.stateful_circuits[0]
                .actions
                .push(crate::ir::StateAction::Assert {
                    condition: Expr::Boolean { value: true },
                    message: "guard".into(),
                });
        }
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none());
        assert!(context.domain(root).unwrap().is_some());
        let rendered = crate::render_with_capabilities(&contract).unwrap();
        let entries: Vec<_> = rendered
            .capabilities
            .circuits
            .iter()
            .filter(|c| c.name == "root")
            .collect();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].recorded && entries[0].observed_call);
    }
}
#[test]
fn parameterized_and_effectful_roots_remain_outside_rollout() {
    for mode in 0..2 {
        let mut contract = subject(bytes());
        let root = &mut contract.stateful_circuits[0];
        if mode == 0 {
            root.parameters.push(Parameter {
                name: "unused".into(),
                ty: Type::Field,
            });
        } else {
            root.return_value = StateReturn::Effectful {
                body: crate::ir::ReturnPlan::Value { value: bytes() },
            };
        }
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.algebra(root).is_ok());
        assert!(context.domain(root).unwrap().is_none());
    }
}
