// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::recorded::admission_probe::trace;
fn trace(contract: &Contract) -> serde_json::Value {
    let root = &contract.stateful_circuits[0];
    let context = Context::new(contract);
    let (_, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(root, false);
        let result = context.selected(root).unwrap();
        scope.finish(&Ok::<_, RenderError>(
            crate::recorded::RecordingOutcome::Supported(()),
        ));
        result
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
fn checked_kernel_owner_runs_once_without_legacy_or_later_fallback() {
    let mut contract = subject();
    contract.stateful_circuits[0].actions =
        vec![action(mint(parameter("domain"), parameter("amount")))];
    let row = trace(&contract);
    assert_eq!(row["body_owner"], "algebra.kernel");
    let a = attempts(&row);
    assert_eq!(a.iter().filter(|&&id| id == "algebra.kernel").count(), 1);
    assert!(!a.contains(&"selector.kernel"));
    assert!(!a.contains(&"algebra.context-query"));
}
#[test]
fn earlier_point_owner_and_later_context_owner_keep_precedence() {
    let point: Contract = serde_json::from_str(include_str!(
        "../../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap();
    let row = trace(&point);
    assert_eq!(row["body_owner"], "algebra.jubjub-values");
    assert!(!attempts(&row).contains(&"algebra.kernel"));
    let context: Contract = serde_json::from_str(include_str!(
        "../../../../tests/token-query-schema20-ir.json"
    ))
    .unwrap();
    let row = trace(&context);
    assert_eq!(row["body_owner"], "algebra.context-query");
    let a = attempts(&row);
    assert!(
        a.iter().position(|id| *id == "algebra.kernel").unwrap()
            < a.iter()
                .position(|id| *id == "algebra.context-query")
                .unwrap()
    );
}
#[test]
fn no_effect_and_literal_operands_are_separate_expansions() {
    for literal_effect in [false, true] {
        let mut contract = subject();
        if literal_effect {
            contract.stateful_circuits[0].actions = vec![action(claim(
                KernelClaimKind::Nullifier,
                Expr::BytesLiteral { bytes: vec![1; 32] },
            ))];
        }
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none());
        assert!(context.domain(root).unwrap().is_some());
        let output = crate::render_with_capabilities(&contract).unwrap();
        assert_eq!(output.capabilities.circuits.len(), 1);
        assert!(output.capabilities.circuits[0].recorded);
    }
}
#[test]
fn rollout_excludes_internal_field_parameter_and_explicit_return_plan() {
    for mode in 0..3 {
        let mut contract = subject();
        let root = &mut contract.stateful_circuits[0];
        root.actions = vec![action(claim(
            KernelClaimKind::Nullifier,
            parameter("domain"),
        ))];
        match mode {
            0 => root.internal = true,
            1 => root.parameters.push(Parameter {
                name: "field".into(),
                ty: Type::Field,
            }),
            2 => {
                root.actions.clear();
                root.return_value = StateReturn::Effectful {
                    body: crate::ir::ReturnPlan::Value { value: Expr::Unit },
                };
            }
            _ => unreachable!(),
        }
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.algebra(root).is_ok());
        assert!(context.domain(root).unwrap().is_none());
    }
}
#[test]
fn unsupported_body_continues_suffix_but_invariant_is_fatal() {
    let mut contract = subject();
    contract.stateful_circuits[0].actions = vec![action(Expr::HashToCurve {
        value: Box::new(Expr::PersistentCommit {
            value: Box::new(Expr::JubjubPointX {
                value: Box::new(Expr::HashToCurve {
                    value: Box::new(parameter("domain")),
                }),
            }),
            opening: Box::new(Expr::FieldToBytes32 {
                value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            }),
        }),
    })];
    let context = Context::new(&contract);
    assert!(
        context
            .domain(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let row = trace(&contract);
    let a = attempts(&row);
    assert!(
        a.iter().position(|id| *id == "algebra.kernel").unwrap()
            < a.iter()
                .position(|id| *id == "algebra.context-query")
                .unwrap()
    );
    let gap = crate::recorded::RecordingGap {
        resource: None,
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.kernel".into(),
        detail: "wrong claim type".into(),
    };
    assert!(
        matches!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))),Err(RenderError::RecordingPlanInvariant{path,detail}) if path=="forged.kernel"&&detail=="wrong claim type")
    );
}
