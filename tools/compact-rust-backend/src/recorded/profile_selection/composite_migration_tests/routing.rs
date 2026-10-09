// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::recorded::admission_probe::trace;
#[test]
fn actual_public_and_internal_helper_owners_are_composite_algebra() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/field-observation-schema20-ir.json"
    ))
    .unwrap();
    let (rendered, rows) = trace::capture_json(|| crate::render_with_capabilities(&contract));
    rendered.unwrap();
    let rows = rows.as_array().unwrap();
    for name in ["snapshot", "via_field_helper", "via_snapshot_helper"] {
        assert!(
            rows.iter().any(|r| r["circuit"] == name
                && r["phase"] == "public_emission"
                && r["body_owner"] == "algebra.composite-values"),
            "public {name}: {rows:?}"
        );
    }
    // These expression calls are inlined into checked public roots. Shared helper
    // discovery currently selects only Unit/Field interfaces, so exercise the real
    // helper renderer separately rather than inventing a public discovery event.
    assert!(
        !rows
            .iter()
            .any(|r| r["circuit"] == "snapshot_helper" && r["phase"] == "helper_planning")
    );
    let context = Context::new(&contract);
    let helper = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == "snapshot_helper")
        .unwrap();
    assert!(context.old(helper).is_some(), "internal old-FIRST control");
    let (item, helper_rows) = trace::capture_json(|| {
        let _phase = trace::HelperPlanning::begin();
        crate::recorded::render_recorded_helper(
            helper,
            &context.ledger,
            &context.witnesses,
            &context.pure,
            &context.circuits,
            &std::collections::HashSet::new(),
        )
    });
    assert!(item.unwrap().is_supported(), "actual helper rendering");
    let helper_rows = helper_rows.as_array().unwrap();
    assert_eq!(helper_rows.len(), 1);
    let helper_row = &helper_rows[0];
    assert_eq!(helper_row["circuit"], "snapshot_helper");
    assert_eq!(helper_row["internal"], true);
    assert_eq!(helper_row["helper"], true);
    assert_eq!(helper_row["phase"], "helper_planning");
    assert_eq!(helper_row["body_owner"], "algebra.composite-values");
    assert_eq!(helper_row["outcome"]["status"], "supported");
    assert!(!rows.iter().chain(helper_rows.iter()).any(|r| {
        r["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "selector.composite-values")
    }));
}
#[test]
fn checked_domain_excludes_nonstruct_and_explicit_return_plans() {
    let result = structure("Result", vec![Type::Boolean]);
    let mut contract = subject(result.clone(), Expr::Default { ty: result });
    contract.stateful_circuits[0].internal = true;
    assert!(
        Context::new(&contract)
            .domain(&contract.stateful_circuits[0])
            .unwrap()
            .is_some(),
        "no public-only restriction"
    );
    contract.stateful_circuits[0].return_value = StateReturn::Effectful {
        body: crate::ir::ReturnPlan::Value {
            value: Expr::Boolean { value: true },
        },
    };
    assert!(
        Context::new(&contract)
            .domain(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    contract.stateful_circuits[0].result = Type::Boolean;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Boolean { value: true },
    };
    assert!(
        Context::new(&contract)
            .domain(&contract.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
}
#[test]
fn invariant_failure_is_fatal_and_unsupported_declines_without_old_fallback() {
    let gap = crate::recorded::RecordingGap {
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "return_value.value".into(),
        detail: "forged composite operation".into(),
    };
    assert!(
        resolve_checked_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    assert!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))).is_err());
}
#[test]
fn composite_selection_is_lazy_between_earlier_context_and_later_shielded_routes() {
    let context: Contract = serde_json::from_str(include_str!(
        "../../../../tests/token-query-schema20-ir.json"
    ))
    .unwrap();
    let composite = subject(
        structure("Result", vec![Type::Boolean]),
        Expr::Default {
            ty: structure("Result", vec![Type::Boolean]),
        },
    );
    for (contract, owner, attempted) in [
        (&context, "algebra.context-query", false),
        (&composite, "algebra.composite-values", true),
    ] {
        let root = &contract.stateful_circuits[0];
        let maps = Context::new(contract);
        let (_, rows) = trace::capture_json(|| {
            let scope = trace::Scope::begin(root, false);
            let selected = maps.selected(root).unwrap();
            scope.finish(&Ok::<_, RenderError>(
                crate::recorded::RecordingOutcome::Supported(()),
            ));
            selected
        });
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["body_owner"], owner);
        let attempts = rows[0]["attempts"].as_array().unwrap();
        assert_eq!(
            attempts
                .iter()
                .filter(|a| a["route"] == "algebra.composite-values")
                .count(),
            usize::from(attempted)
        );
        assert!(
            !attempts
                .iter()
                .any(|a| a["route"] == "selector.shielded-receive")
        );
    }
}
