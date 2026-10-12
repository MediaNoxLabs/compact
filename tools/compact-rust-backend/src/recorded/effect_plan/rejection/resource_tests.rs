// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::{RecordingResourceKind, RecordingResourcePhase};

fn layout_type() -> Type {
    Type::Vector {
        element: Box::new(Type::Field),
        length: crate::resource_limits::recording_fixed_layout_limit(),
    }
}
fn binding_action() -> StateAction {
    StateAction::Let {
        bindings: vec![LocalBinding {
            name: "large".into(),
            ty: layout_type(),
            value: Expr::Default { ty: layout_type() },
        }],
        action: Box::new(empty()),
    }
}
fn layout_refusal() -> LoweringRejection {
    refusal(&[circuit("root", vec![empty(), binding_action()])])
}

#[test]
fn typed_resource_refinement_requires_exact_failed_binding_and_body_origin() {
    let mut rejection = layout_refusal();
    assert_eq!(rejection.root_action, Some(1));
    assert_eq!(rejection.phase, FailurePhase::CircuitBody);
    assert_eq!(rejection.gap.path, "actions[1].bindings[0].value");
    let payload = rejection.gap.resource.as_ref().unwrap();
    assert_eq!(payload.kind, RecordingResourceKind::FixedLayoutSlots);
    assert_eq!(payload.phase, RecordingResourcePhase::FixedLayoutValidation);
    let gap = RecordingGap::action(&binding_action(), "actions[1]".into());
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::BindingValue(0), gap.clone()),
        rejection.gap
    );
    for precision in [
        LegacyActionPrecision::Concrete,
        LegacyActionPrecision::Coarse,
        LegacyActionPrecision::BindingValue(1),
    ] {
        assert_eq!(rejection.refine_action(1, precision, gap.clone()), gap);
    }
    assert_eq!(
        rejection.refine_action(0, LegacyActionPrecision::BindingValue(0), gap.clone()),
        gap
    );
    for wrong_path in ["actions[10]", "actions[1].action", "actions[1].actions[0]"] {
        let wrong = RecordingGap::action(&binding_action(), wrong_path.into());
        assert_eq!(
            rejection.refine_action(1, LegacyActionPrecision::BindingValue(0), wrong.clone()),
            wrong
        );
    }
    assert!(!encloses("actions[1]", "actions[10].value"));
    assert!(!encloses("actions[1].action", "actions[1].actions[0]"));
    assert!(!encloses(
        "actions[1].bindings[0].value",
        "actions[1].bindings[0].value_other"
    ));
    rejection.phase = FailurePhase::OutsideBody;
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::BindingValue(0), gap.clone()),
        gap
    );
    rejection.phase = FailurePhase::CircuitBody;
    rejection.gap.resource = None;
    // Display text alone retains the precise-looking resource wording but is not authority.
    assert_eq!(
        rejection.refine_action(1, LegacyActionPrecision::BindingValue(0), gap.clone()),
        gap
    );
}

#[test]
fn signature_return_and_invalid_phases_do_not_gain_action_authority() {
    let mut root = circuit("root", vec![binding_action()]);
    root.parameters.push(Parameter {
        name: "input".into(),
        ty: layout_type(),
    });
    let parameter = refusal(&[root]);
    assert_eq!(parameter.root_action, None);
    assert_eq!(parameter.phase, FailurePhase::OutsideBody);
    assert!(parameter.gap.resource.is_some());
    let mut root = circuit("root", vec![empty(), empty()]);
    root.return_value = StateReturn::Expression {
        value: Expr::Default { ty: layout_type() },
    };
    let returned = refusal(&[root]);
    assert_eq!(returned.root_action, None);
    assert_eq!(returned.phase, FailurePhase::CircuitBody);
    assert!(returned.gap.resource.is_some());
    let coarse = RecordingGap::action(&binding_action(), "actions[0]".into());
    for rejection in [parameter, returned] {
        assert_eq!(
            rejection.refine_action(0, LegacyActionPrecision::BindingValue(0), coarse.clone()),
            coarse
        );
        let invalid: PlanError = DiagnosticPlanError::Invalid(rejection.gap.clone()).into();
        assert!(matches!(invalid, PlanError::Invalid(gap) if gap == rejection.gap));
    }
}

fn whole_contract(value: serde_json::Value) -> Contract {
    serde_json::from_value(value).unwrap()
}
fn whole_refusal(contract: &Contract) -> LoweringRejection {
    let root = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == "assertStoredRecordFresh")
        .unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let calls = contract
        .stateful_circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    match effect_plan::check_with_calls_diagnostic(root, &ledger, &witnesses, &pure, &calls)
        .unwrap_err()
    {
        DiagnosticPlanError::Unsupported(rejection) => rejection,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn whole_resource_cause_matches_candidate_public_and_dependency() {
    let c = whole_contract(crate::capabilities::resource_diagnostic_tests::oversized());
    let rejection = whole_refusal(&c);
    assert_eq!(rejection.root_action, Some(0));
    assert_eq!(rejection.phase, FailurePhase::CircuitBody);
    let output = crate::render_with_capabilities(&c).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == "assertStoredRecordFresh")
        .unwrap();
    assert_eq!(cap.recording_unavailable.as_ref().unwrap(), &rejection.gap);
    let dependency = cap.observed_call_unavailable.as_ref().unwrap();
    assert_eq!(dependency.resource, rejection.gap.resource);
    assert_eq!(dependency.path, rejection.gap.path);
    assert_eq!(dependency.ir_node, rejection.gap.ir_node);
}

#[test]
fn earlier_legacy_binding_failure_is_not_replaced_by_later_checked_resource() {
    let mut value = crate::capabilities::resource_diagnostic_tests::oversized();
    let root = value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["name"] == "assertStoredRecordFresh")
        .unwrap();
    let tuple = serde_json::json!({"kind":"tuple","elements":[{"kind":"field"},{"kind":"field"}]});
    let binding = serde_json::json!({"name":"earlier_default","ty":tuple,"value":{"kind":"default","ty":tuple}});
    // Authenticate the earlier legacy refusal directly; scalar Boolean defaults
    // are supported and would not be a meaningful precedence control.
    let earlier = StateAction::Let {
        bindings: vec![serde_json::from_value(binding.clone()).unwrap()],
        action: Box::new(empty()),
    };
    let mut precision = LegacyActionPrecision::Concrete;
    let old = crate::recorded::legacy::actions::append_steps(
        &earlier,
        "actions[0]",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &std::collections::HashSet::new(),
        &mut Vec::new(),
        &mut 0,
        &mut std::collections::HashSet::new(),
        &mut precision,
    )
    .unwrap();
    assert!(
        matches!(old, crate::recorded::RecordingOutcome::Unsupported(gap)
        if gap.path == "actions[0]" && gap.resource.is_none())
    );
    assert!(matches!(precision, LegacyActionPrecision::BindingValue(0)));
    root["actions"][0]["action"]["actions"][1]["bindings"]
        .as_array_mut()
        .unwrap()
        .insert(0, binding);
    let c = whole_contract(value);
    let rejection = whole_refusal(&c);
    assert_eq!(
        rejection.gap.path,
        "actions[0].action.actions[1].bindings[1].value"
    );
    assert!(rejection.gap.resource.is_some());
    let output = crate::render_with_capabilities(&c).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == "assertStoredRecordFresh")
        .unwrap();
    let gap = cap.recording_unavailable.as_ref().unwrap();
    assert!(
        gap.resource.is_none(),
        "earlier legacy binding owns refusal"
    );
    assert_eq!(gap.path, "actions[0].action.actions[1]");
    assert_eq!(gap.ir_node, "StateAction::Let");
}

#[test]
fn diagnostic_error_carriers_remain_compact() {
    assert!(std::mem::size_of::<RecordingGap>() <= 88);
    assert!(std::mem::size_of::<LoweringRejection>() < 128);
}
