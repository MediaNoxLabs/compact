// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn refusal(c: &Contract, name: &str) -> (ProfileSelection, ProfileSelection) {
    let context = Context::new(c);
    let root = context.circuits[name];
    let old = context.selected(root, true, None).0.unwrap();
    let (current, rows) = context.selected(root, false, None);
    let current = current.unwrap();
    assert_eq!(syntax(&old), (None, None));
    assert_eq!(syntax(&current), (None, None));
    assert_eq!(
        rows[0]["attempts"],
        serde_json::json!([
            {"route":"algebra.ordinary", "produced_candidate":false}
        ])
    );
    assert_eq!(old.shielded_send_rejection, current.shielded_send_rejection);
    (old, current)
}

#[test]
fn actual_unsupported_preserves_fallback_and_render_invariant_remains_fatal() {
    let mut c = identity(Type::Field);
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: test_support::unsupported_fixed_input(),
    };
    test_support::register_unsupported_fold(&mut c);
    let context = Context::new(&c);
    assert!(matches!(
        context.checked(&c.stateful_circuits[0]),
        Err(effect_plan::DiagnosticPlanError::Unsupported(_))
    ));
    let (old, current) = refusal(&c, "root");
    assert!(old.unit_composition_rejection.is_none());
    assert!(current.unit_composition_rejection.is_none());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(!output.capabilities.circuits[0].recorded);

    // Invalid Rust slot spelling is intentionally tested below native authority.
    // Independent semantic validation succeeds; emission then produces Invalid,
    // which neither selector may turn into a legacy compatibility signal.
    let c: Contract = serde_json::from_value(serde_json::json!({
        "schema_version":crate::ir::SCHEMA_VERSION,
        "type_aliases":[],"constructor":null,"witnesses":[],"circuits":[],
        "ledger_fields":[{"id":"invalid-slot","index":0,
            "declaration":{"kind":"cell","ty":{"kind":"field"}}}],
        "stateful_circuits":[{"name":"root","internal":false,"parameters":[],
            "result":{"kind":"field"},"actions":[],"return_value":{
                "kind":"expression","value":{"kind":"cell_read","field":"invalid-slot","index":0}}}]
    }))
    .unwrap();
    let context = Context::new(&c);
    let root = &c.stateful_circuits[0];
    assert!(
        effect_plan::check_with_calls_diagnostic(
            root,
            &context.ledger,
            &context.witnesses,
            &context.pure,
            &context.circuits
        )
        .is_ok()
    );
    let Err(effect_plan::DiagnosticPlanError::Invalid(gap)) = context.checked(root) else {
        panic!("expected a real renderer invariant failure");
    };
    for old in [false, true] {
        let Err(RenderError::RecordingPlanInvariant { path, detail }) =
            context.selected(root, old, None).0
        else {
            panic!("Invalid must abort both selectors");
        };
        assert_eq!((path, detail), (gap.path.clone(), gap.detail.clone()));
    }
    assert!(
        crate::render_with_capabilities(&c).is_err(),
        "native rejects before recording"
    );
}

fn layout_action() -> StateAction {
    let ty = Type::Vector {
        element: Box::new(Type::Field),
        length: crate::resource_limits::recording_fixed_layout_limit(),
    };
    StateAction::Let {
        bindings: vec![LocalBinding {
            name: "large".into(),
            ty: ty.clone(),
            value: Expr::Default { ty },
        }],
        action: Box::new(empty()),
    }
}
fn unit_refinements(
    c: &Contract,
) -> (
    effect_plan::LoweringRejection,
    effect_plan::LoweringRejection,
) {
    let (old, current) = refusal(c, "root");
    (
        old.unit_composition_rejection.unwrap(),
        current.unit_composition_rejection.unwrap(),
    )
}

#[test]
fn unit_resource_provenance_preserves_phase_ordinal_boundary_and_first_refusal() {
    let mut c = unit(vec![]);
    c.stateful_circuits[0].actions = vec![empty(), layout_action()];
    let (old, current) = unit_refinements(&c);
    let coarse = RecordingGap::action(&layout_action(), "actions[1]".into());
    let precise = current.refine_action(1, LegacyActionPrecision::BindingValue(0), coarse.clone());
    assert!(precise.resource.is_some());
    assert_eq!(precise.path, "actions[1].bindings[0].value");
    assert_eq!(
        old.refine_action(1, LegacyActionPrecision::BindingValue(0), coarse.clone()),
        precise
    );
    for (ordinal, precision, path) in [
        (0, LegacyActionPrecision::BindingValue(0), "actions[1]"),
        (1, LegacyActionPrecision::BindingValue(1), "actions[1]"),
        (1, LegacyActionPrecision::Concrete, "actions[1]"),
        (1, LegacyActionPrecision::Coarse, "actions[1]"),
        (1, LegacyActionPrecision::BindingValue(0), "actions[10]"),
        (
            1,
            LegacyActionPrecision::BindingValue(0),
            "actions[1].action",
        ),
    ] {
        let coarse = RecordingGap::action(&layout_action(), path.into());
        assert_eq!(
            old.refine_action(ordinal, precision, coarse.clone()),
            coarse
        );
        assert_eq!(
            current.refine_action(ordinal, precision, coarse.clone()),
            coarse
        );
    }
    // Signature validation is outside the root body and must not inherit action1.
    let mut signature = c.clone();
    signature.stateful_circuits[0].parameters.push(Parameter {
        name: "large".into(),
        ty: Type::Vector {
            element: Box::new(Type::Field),
            length: 32769,
        },
    });
    let (old, current) = unit_refinements(&signature);
    for rejection in [old, current] {
        assert_eq!(
            rejection.refine_action(1, LegacyActionPrecision::BindingValue(0), coarse.clone()),
            coarse
        );
    }
    // A genuinely unsupported earlier binding must remain first, even when a
    // later binding in that same Let exceeds layout limits.
    let StateAction::Let { bindings, .. } = &mut c.stateful_circuits[0].actions[1] else {
        unreachable!()
    };
    bindings.insert(
        0,
        LocalBinding {
            name: "first".into(),
            ty: Type::Field,
            value: test_support::unsupported_fixed_input(),
        },
    );
    test_support::register_unsupported_fold(&mut c);
    let (old, current) = unit_refinements(&c);
    let earlier = current.refine_action(1, LegacyActionPrecision::Coarse, coarse.clone());
    assert!(earlier.resource.is_none());
    assert_eq!(earlier.ir_node, "Expr::VectorFoldCall");
    assert_eq!(earlier.path, "actions[1].bindings[0].value");
    assert_eq!(
        old.refine_action(1, LegacyActionPrecision::Coarse, coarse),
        earlier
    );

    // Bind the same rich payload to an actual complete-source public refusal.
    let whole: Contract =
        serde_json::from_value(crate::capabilities::resource_diagnostic_tests::oversized())
            .unwrap();
    let (old, current) = refusal(&whole, "assertStoredRecordFresh");
    let old_gap = old.unit_composition_rejection.unwrap().into_gap();
    let current_gap = current.unit_composition_rejection.unwrap().into_gap();
    assert_eq!(old_gap, current_gap);
    let output = crate::render_with_capabilities(&whole).unwrap();
    let row = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == "assertStoredRecordFresh")
        .unwrap();
    assert_eq!(row.recording_unavailable.as_ref().unwrap(), &current_gap);
    assert_eq!(
        row.observed_call_unavailable.as_ref().unwrap().resource,
        current_gap.resource
    );
}

#[test]
fn shielded_gap_projection_matches_the_staged_oracle_without_global_refinement() {
    let mut c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/shielded-send-schema20-ir.json"
    ))
    .unwrap();
    test_support::register_unsupported_fold(&mut c);
    let root = &mut c.stateful_circuits[1];
    let name = root.name.clone();
    assert!(typed_plan::shielded_send_result(&root.result));
    let StateReturn::Expression { value } = &mut root.return_value else {
        unreachable!()
    };
    *value = Expr::Let {
        bindings: vec![LocalBinding {
            name: "first".into(),
            ty: Type::Field,
            value: test_support::unsupported_fixed_input(),
        }],
        body: Box::new(value.clone()),
    };
    let (_, current) = refusal(&c, &name);
    let gap = current.shielded_send_rejection.unwrap();
    assert_eq!(gap.ir_node, "Expr::VectorFoldCall");
    assert_eq!(gap.path, "return_value.value.bindings[0].value");
    assert!(current.unit_composition_rejection.is_none());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .find(|v| v.name == name)
            .unwrap()
            .recorded
    );
    let root = &mut c.stateful_circuits[1];
    root.result = Type::Struct {
        name: "OtherResult".into(),
        fields: vec![],
    };
    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut root.return_value
    else {
        unreachable!()
    };
    **body = Expr::Default {
        ty: root.result.clone(),
    };
    let (_, current) = refusal(&c, &name);
    assert!(current.shielded_send_rejection.is_none());
}
