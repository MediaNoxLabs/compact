// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Laws for complete checked outcomes and unchanged compatibility projection.
use super::*;

fn general(
    c: &Contract,
    prechecked: Option<TypedPlan>,
) -> (Result<CheckedSelection, RenderError>, Value) {
    let ctx = Context::new(c);
    let root = &c.stateful_circuits[0];
    trace::capture_json(|| {
        let _scope = trace::Scope::begin(root, false);
        select_checked(
            root,
            &ctx.ledger,
            &ctx.witnesses,
            &ctx.pure,
            &ctx.circuits,
            prechecked,
        )
    })
}

#[test]
fn nonunit_refusal_is_retained_before_unchanged_compatibility_projection() {
    let mut c = identity(Type::Field);
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: test_support::unsupported_fixed_input(),
    };
    test_support::register_unsupported_fold(&mut c);
    let ctx = Context::new(&c);
    let expected = match ctx.checked(&c.stateful_circuits[0]) {
        Err(effect_plan::DiagnosticPlanError::Unsupported(r)) => format!("{r:?}"),
        _ => panic!("expected checked refusal"),
    };
    let (actual, trace) = general(&c, None);
    let Ok(CheckedSelection::Refused(rejection)) = actual else {
        panic!("general carrier must retain the complete non-Unit refusal");
    };
    assert_eq!(format!("{rejection:?}"), expected);
    let (wrapped, wrapper_trace) = ctx.selected(&c.stateful_circuits[0], false, None);
    let wrapped = wrapped.unwrap();
    assert_eq!(syntax(&wrapped), (None, None));
    assert!(wrapped.unit_composition_rejection.is_none());
    assert!(wrapped.shielded_send_rejection.is_none());
    assert_eq!(trace[0]["attempts"], wrapper_trace[0]["attempts"]);
    assert_eq!(
        trace[0]["attempts"],
        serde_json::json!([
            {"route":"algebra.ordinary", "produced_candidate":false}
        ])
    );
}

#[test]
fn admitted_plan_is_moved_unchanged_and_ordinary_attempt_occurs_once() {
    let c = identity(Type::Field);
    let (actual, trace) = general(&c, None);
    let Ok(CheckedSelection::Admitted(plan)) = actual else {
        panic!("admitted");
    };
    let ctx = Context::new(&c);
    let (wrapped, wrapper_trace) = ctx.selected(&c.stateful_circuits[0], false, None);
    assert_eq!(syntax(&admitted(plan)), syntax(&wrapped.unwrap()));
    assert_eq!(trace[0]["attempts"], wrapper_trace[0]["attempts"]);
    assert_eq!(
        trace[0]["attempts"],
        serde_json::json!([
            {"route":"algebra.ordinary", "produced_candidate":true}
        ])
    );
}

#[test]
fn supplied_plan_bypasses_checker_and_new_carrier_does_not_exceed_old_descriptor() {
    let mut c = identity(Type::Field);
    // Intentionally below native authority: a supplied plan must bypass lowering.
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Parameter {
            name: "missing".into(),
        },
    };
    let plan = TypedPlan {
        steps: vec![],
        result: syn::parse_quote!(sentinel),
    };
    let (actual, trace) = general(&c, Some(plan));
    let Ok(CheckedSelection::Admitted(plan)) = actual else {
        panic!("prechecked");
    };
    assert!(plan.steps.is_empty());
    assert_eq!(plan.result.to_token_stream().to_string(), "sentinel");
    assert!(trace[0]["attempts"].as_array().unwrap().is_empty());
    // Keep the moved carrier within the existing descriptor footprint.
    assert!(std::mem::size_of::<CheckedSelection>() <= std::mem::size_of::<ProfileSelection>());
}

#[test]
fn renderer_invalid_is_fatal_before_any_compatibility_projection() {
    let mut c = identity(Type::Field);
    c.stateful_circuits[0].parameters.clear();
    c.ledger_fields.push(LedgerField {
        id: "invalid-slot".into(),
        index: 0,
        path: vec![],
        source: None,
        declaration: crate::ir::LedgerFieldKind::Cell { ty: Type::Field },
    });
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::CellRead {
            field: "invalid-slot".into(),
            index: 0,
        },
    };
    let ctx = Context::new(&c);
    let expected = match ctx.checked(&c.stateful_circuits[0]) {
        Err(effect_plan::DiagnosticPlanError::Invalid(gap)) => (gap.path, gap.detail),
        _ => panic!("real renderer invariant must be Invalid"),
    };
    let Err(RenderError::RecordingPlanInvariant { path, detail }) = general(&c, None).0 else {
        panic!("Invalid must escape general carrier");
    };
    assert_eq!((path, detail), expected);
    let Err(RenderError::RecordingPlanInvariant { path, detail }) =
        ctx.selected(&c.stateful_circuits[0], false, None).0
    else {
        panic!("wrapper fatal");
    };
    assert_eq!((path, detail), expected);
}
