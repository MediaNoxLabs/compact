// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Routing controls for the first effect-algebra migration. Token comparisons
//! identify the selected renderer; generated-runtime tests check its behavior.

use super::*;
use crate::ir::{Contract, ReturnPlan};
use quote::quote;

mod inclusion;
mod resources;

fn effectful_fixture() -> Contract {
    serde_json::from_str(include_str!(
        "../../tests/effectful-return-schema20-ir.json"
    ))
    .unwrap()
}

struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}

impl<'a> Context<'a> {
    fn new(contract: &'a Contract) -> Self {
        Self {
            ledger: contract
                .ledger_fields
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect(),
            witnesses: contract
                .witnesses
                .iter()
                .map(|w| (w.name.as_str(), w))
                .collect(),
            pure: contract
                .circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect(),
            circuits: contract
                .stateful_circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect(),
        }
    }

    fn selected(&self, circuit: &StatefulCircuit) -> RecordingOutcome<typed_plan::TypedPlan> {
        lower_effectful_return(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
        .unwrap()
    }

    fn legacy(&self, circuit: &StatefulCircuit) -> Option<typed_plan::TypedPlan> {
        typed_plan::lower_effectful(circuit, &self.ledger, &self.witnesses, &self.pure)
    }

    fn algebra(
        &self,
        circuit: &StatefulCircuit,
    ) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
        effect_plan::lower(circuit, &self.ledger, &self.witnesses)
    }
}

fn tokens(plan: &typed_plan::TypedPlan) -> String {
    let steps = &plan.steps;
    let result = &plan.result;
    quote! { #(#steps)* #result }.to_string()
}

fn supported(outcome: RecordingOutcome<typed_plan::TypedPlan>) -> typed_plan::TypedPlan {
    match outcome {
        RecordingOutcome::Supported(plan) => plan,
        RecordingOutcome::Unsupported(gap) => panic!("expected recording support: {gap:?}"),
    }
}

#[test]
fn legacy_admitted_intersection_uses_the_algebra_renderer() {
    let contract = effectful_fixture();
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    let legacy = context.legacy(circuit).expect("legacy admission control");
    let algebra = context
        .algebra(circuit)
        .expect("algebra intersection control");
    assert_ne!(
        tokens(&legacy),
        tokens(&algebra),
        "fixture must distinguish the renderers"
    );
    assert_eq!(
        tokens(&supported(context.selected(circuit))),
        tokens(&algebra)
    );
    assert!(
        crate::render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn wide_scalar_in_legacy_family_uses_checked_algebra() {
    let mut contract = effectful_fixture();
    let StateReturn::Effectful { body } = &mut contract.stateful_circuits[0].return_value else {
        panic!("fixture effectful return")
    };
    let max = "340282366920938463463374607431768211456";
    *body = ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "unused_wide".into(),
            ty: Type::Unsigned { max: max.into() },
            value: Expr::UnsignedLiteral {
                value: "1".into(),
                max: max.into(),
            },
        }],
        result: Box::new(body.clone()),
    };
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    let legacy = context
        .legacy(circuit)
        .expect("wide scalar remains legacy-supported");
    let algebra = context
        .algebra(circuit)
        .expect("wide values are checked algebra operations");
    assert_ne!(tokens(&algebra), tokens(&legacy));
    assert_eq!(
        tokens(&supported(context.selected(circuit))),
        tokens(&algebra)
    );
}

#[test]
fn real_source_wide_equality_uses_algebra_with_legacy_admission() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../tests/effect-plan-wide-schema20-ir.json"
    ))
    .unwrap();
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    let legacy = context
        .legacy(circuit)
        .expect("real source remains in legacy family");
    let algebra = context
        .algebra(circuit)
        .expect("real source wide values lower through algebra");
    assert_ne!(tokens(&legacy), tokens(&algebra));
    assert_eq!(
        tokens(&supported(context.selected(circuit))),
        tokens(&algebra)
    );
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
}

#[test]
fn start_funding_still_uses_its_separate_legacy_profile() {
    let contract: Contract =
        serde_json::from_str(include_str!("../../tests/coracle-start-schema20-ir.json")).unwrap();
    let context = Context::new(&contract);
    let circuit = context.circuits["start"];
    assert!(
        context.legacy(circuit).is_none(),
        "start is outside the migrating family"
    );
    assert!(matches!(
        context.algebra(circuit),
        Err(effect_plan::PlanError::Unsupported(_))
    ));
    let start = typed_plan::lower_start_funding(
        circuit,
        &context.ledger,
        &context.witnesses,
        &context.pure,
        &context.circuits,
    )
    .expect("retained start-funding admission");
    assert_eq!(
        tokens(&supported(context.selected(circuit))),
        tokens(&start)
    );
}

#[test]
fn broader_effectful_compositions_use_default_algebra_admission() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    let context = Context::new(&contract);
    for circuit in &contract.stateful_circuits {
        assert!(context.legacy(circuit).is_none());
        let algebra = context
            .algebra(circuit)
            .expect("checked composition control");
        assert_eq!(
            tokens(&supported(context.selected(circuit))),
            tokens(&algebra)
        );
    }
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
}

#[test]
fn pure_effectful_recording_api_does_not_promise_a_nonempty_transcript() {
    let mut contract = effectful_fixture();
    let circuit = &mut contract.stateful_circuits[0];
    circuit.source = None;
    circuit.return_value = StateReturn::Effectful {
        body: ReturnPlan::Value {
            value: Expr::FieldLiteral { value: "9".into() },
        },
    };
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    assert!(context.legacy(circuit).is_none());
    let algebra = context
        .algebra(circuit)
        .expect("literal is algebra-supported");
    assert!(
        !algebra.steps.is_empty(),
        "literal binding is not a VM effect"
    );
    assert_eq!(
        tokens(&supported(context.selected(circuit))),
        tokens(&algebra)
    );
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    let capability = &rendered.capabilities.circuits[0];
    // Availability describes the recording/observed-call API, not successful
    // preparation. The runtime still refuses an actual empty public transcript.
    assert!(capability.recorded && capability.observed_call);
    assert!(capability.recording_unavailable.is_none());
}

#[test]
fn unsupported_effectful_nodes_keep_the_exact_algebra_refusal() {
    let mut contract = effectful_fixture();
    contract.stateful_circuits[0].return_value = StateReturn::Effectful {
        body: ReturnPlan::Value {
            value: Expr::Call {
                name: "unsupported_helper".into(),
                arguments: vec![],
            },
        },
    };
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    let Err(effect_plan::PlanError::Unsupported(expected)) = context.algebra(circuit) else {
        panic!("expected unsupported helper");
    };
    let RecordingOutcome::Unsupported(actual) = context.selected(circuit) else {
        panic!("missing helper cannot be recorded");
    };
    assert_eq!(actual, expected);
    assert_eq!(actual.path, "return_value.body.value");
    assert_eq!(actual.ir_node, "Expr::Call");
}
