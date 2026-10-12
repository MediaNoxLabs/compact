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
    assert!(!attempted.contains(&"algebra.unit-composition"));
}

#[test]
fn earlier_field_interface_owner_keeps_its_priority() {
    let mut contract = subject();
    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: parameter("seed"),
    };
    let (_, row) = selected_trace(&contract);
    assert_eq!(row["body_owner"], "algebra.membership");
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
    assert!(at("algebra.unit-actions") < at("algebra.unit-composition"));
    assert!(at("algebra.unit-composition") < at("algebra.zswap"));
    assert!(!attempted.contains(&"selector.unit-actions"));
    let gap = crate::recorded::RecordingGap {
        resource: None,
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
    for mode in 0..2 {
        let mut contract = subject();
        let root = &mut contract.stateful_circuits[0];
        match mode {
            0 => {
                root.parameters[0].ty = Type::Unsigned { max: "255".into() };
                root.return_value = StateReturn::Expression { value: Expr::Unit };
            }
            1 => {
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

#[test]
fn ordinary_unit_return_enters_composition_without_an_operation_shape_gate() {
    let mut contract = subject();
    let root = &mut contract.stateful_circuits[0];
    root.actions = vec![write("amount", 0, parameter("seed"))];
    root.return_value = StateReturn::Unit;
    let root = &contract.stateful_circuits[0];
    let context = Context::new(&contract);
    assert!(
        context.old(root).is_none(),
        "explicit expansion beyond old expression admission"
    );
    assert_eq!(
        tokens(&context.unit(root).unwrap().unwrap()),
        tokens(&context.algebra(root).unwrap())
    );
    let (_, row) = selected_trace(&contract);
    assert_eq!(row["body_owner"], "algebra.unit-actions");
    assert!(!attempts(&row).contains(&"algebra.unit-composition"));
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    let capability = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == root.name)
        .unwrap();
    assert!(capability.recorded && capability.observed_call);
}

#[test]
fn real_set_lifecycle_has_public_unit_composition_and_existing_direct_queries_stay_available() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-set-schema20-ir.json"
    ))
    .unwrap();
    let index = contract
        .stateful_circuits
        .iter()
        .position(|c| c.name == "lifecycle")
        .unwrap();
    contract.stateful_circuits.swap(0, index);
    let (_, row) = selected_trace(&contract);
    assert_eq!(row["body_owner"], "algebra.unit-actions");
    assert!(!attempts(&row).contains(&"algebra.unit-composition"));

    // Member's specialized return is not emitted by this source; exercise it as
    // an explicitly IR-only public control alongside the real size/empty roots.
    let mut member = contract.stateful_circuits[0].clone();
    member.name = "direct_member".into();
    member.result = Type::Boolean;
    member.actions.clear();
    member.return_value = StateReturn::SetMember {
        field: "scalars".into(),
        index: 0,
        value: parameter("seed"),
    };
    contract.stateful_circuits.push(member);
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    let capabilities: Vec<_> = rendered
        .capabilities
        .circuits
        .iter()
        .map(|c| {
            assert!(c.recorded && c.observed_call, "{}: {c:?}", c.name);
            c.name.as_str()
        })
        .collect();
    assert_eq!(capabilities.len(), 5);
    for name in [
        "run",
        "lifecycle",
        "scalarSize",
        "scalarEmpty",
        "direct_member",
    ] {
        assert!(capabilities.contains(&name));
    }
}
