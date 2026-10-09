// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite old-family inclusion and actual selector controls, not language coverage.
use super::*;
use crate::ir::{
    Contract, Expr, LedgerFieldKind, LocalBinding, Parameter, StateAction, StateReturn, Type,
};
use quote::quote;

#[path = "jubjub_migration_tests/inclusion.rs"]
mod inclusion;

fn fixture() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap()
}
fn subject() -> Contract {
    let mut contract = fixture();
    contract.stateful_circuits.truncate(1);
    let circuit = &mut contract.stateful_circuits[0];
    circuit.actions = vec![write(parameter("point"))];
    circuit.return_value = StateReturn::Expression {
        value: parameter("point"),
    };
    contract
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn literal(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn sequence(actions: Vec<StateAction>) -> StateAction {
    StateAction::Sequence { actions }
}
fn write(value: Expr) -> StateAction {
    StateAction::CellWrite {
        field: "result".into(),
        index: 0,
        value,
    }
}
fn bind(name: &str, ty: Type, value: Expr, action: StateAction) -> StateAction {
    StateAction::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        action: Box::new(action),
    }
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
    fn old(&self, circuit: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower_jubjub_values(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn selected(&self, circuit: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        select(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
    }
    fn algebra(&self, circuit: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower(circuit, &self.ledger, &self.witnesses)
    }
}
fn tokens(plan: &TypedPlan) -> String {
    let steps = &plan.steps;
    let result = &plan.result;
    quote!(#(#steps)* #result).to_string()
}
fn selected_tokens(selected: ProfileSelection) -> Option<String> {
    let steps = selected.organizer_steps?;
    let result = selected.typed_result?;
    Some(quote!(#(#steps)* #result).to_string())
}
fn included(contract: &Contract, circuit: &StatefulCircuit) {
    let context = Context::new(contract);
    context
        .old(circuit)
        .expect("every inclusion case must FIRST be old-admitted");
    let algebra = context
        .algebra(circuit)
        .expect("checked algebra includes old grammar case");
    assert_eq!(
        selected_tokens(context.selected(circuit).unwrap()),
        Some(tokens(&algebra)),
        "{}",
        circuit.name
    );
    let rendered = crate::render_with_capabilities(contract).unwrap();
    let capabilities: Vec<_> = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|capability| capability.name == circuit.name)
        .collect();
    assert_eq!(capabilities.len(), 1, "expected exactly one named export");
    assert!(capabilities[0].recorded);
}

#[test]
fn retained_point_family_selects_algebra_with_distinguishable_output() {
    let contract = fixture();
    let context = Context::new(&contract);
    for circuit in &contract.stateful_circuits {
        let old = context.old(circuit).unwrap();
        let algebra = context.algebra(circuit).unwrap();
        assert_ne!(tokens(&old), tokens(&algebra));
        included(&contract, circuit);
    }
}

#[test]
fn checked_point_domain_does_not_roll_out_other_ordinary_results() {
    let mut contract = subject();
    let circuit = &mut contract.stateful_circuits[0];
    circuit.result = Type::Field;
    circuit.return_value = StateReturn::Expression {
        value: literal("1"),
    };
    let context = Context::new(&contract);
    assert!(context.algebra(&contract.stateful_circuits[0]).is_ok());
    assert!(
        checked_jubjub_domain(
            &contract.stateful_circuits[0],
            &context.ledger,
            &context.witnesses,
            &context.pure,
            &context.circuits
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn new_compositional_point_domain_is_separate_from_old_inclusion() {
    for writes in [0, 2] {
        let mut contract = subject();
        contract.stateful_circuits[0].actions = vec![write(parameter("point")); writes];
        let circuit = &contract.stateful_circuits[0];
        let context = Context::new(&contract);
        assert!(context.old(circuit).is_none());
        let algebra = context.algebra(circuit).unwrap();
        assert_eq!(
            selected_tokens(context.selected(circuit).unwrap()),
            Some(tokens(&algebra))
        );
        assert!(
            crate::render_with_capabilities(&contract)
                .unwrap()
                .capabilities
                .circuits[0]
                .recorded
        );
    }
}

#[test]
fn unsupported_point_operation_does_not_reenter_the_old_family() {
    let mut contract = subject();
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::EcNeg {
            value: Box::new(parameter("point")),
        },
    };
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    assert!(context.old(circuit).is_none());
    assert!(matches!(
        context.algebra(circuit),
        Err(effect_plan::PlanError::Unsupported(_))
    ));
    assert!(context.selected(circuit).unwrap().typed_result.is_none());
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
    assert_eq!(
        rendered.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateReturn::Expression"
    );
}

#[test]
fn invalid_plan_is_a_fatal_invariant_with_its_original_diagnostic() {
    let gap = super::super::RecordingGap {
        code: super::super::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.binding".into(),
        detail: "wrong point operand".into(),
    };
    assert!(
        resolve_jubjub_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    match resolve_jubjub_plan(Err(effect_plan::PlanError::Invalid(gap))) {
        Err(RenderError::RecordingPlanInvariant { path, detail }) => {
            assert_eq!(path, "forged.binding");
            assert_eq!(detail, "wrong point operand");
        }
        _ => panic!("invalid must never become an optional later-family attempt"),
    }
}
