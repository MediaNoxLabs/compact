// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Differential laws for the single checked attempt and the frozen staged oracle.
use super::*;
use crate::ir::{
    Contract, Expr, LocalBinding, Parameter, StateAction, StateReturn, StructField, Type,
};
use crate::recorded::admission_probe::trace;
use crate::recorded::{LegacyActionPrecision, test_support};
use quote::ToTokens;
use serde_json::Value;

#[path = "single_attempt_tests/diagnostics.rs"]
mod diagnostics;
#[path = "single_attempt_tests/general_selection.rs"]
mod general_selection;
#[path = "single_attempt_tests/interfaces.rs"]
mod interfaces;
#[path = "single_attempt_tests/pipeline.rs"]
mod pipeline;

fn contract(parameters: Vec<Type>, result: Type, returned: StateReturn) -> Contract {
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        ledger_fields: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "root".into(),
            parameters: parameters
                .into_iter()
                .enumerate()
                .map(|(i, ty)| Parameter {
                    name: format!("p{i}"),
                    ty,
                })
                .collect(),
            result,
            actions: vec![],
            return_value: returned,
        }],
    }
}
fn identity(ty: Type) -> Contract {
    contract(
        vec![ty.clone()],
        ty,
        StateReturn::Expression {
            value: Expr::Parameter { name: "p0".into() },
        },
    )
}
fn unit(parameters: Vec<Type>) -> Contract {
    contract(parameters, Type::Unit, StateReturn::Unit)
}
fn empty() -> StateAction {
    StateAction::Sequence { actions: vec![] }
}
struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            ledger: c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            witnesses: c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
            pure: c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn selected(
        &self,
        root: &StatefulCircuit,
        old: bool,
        prechecked: Option<TypedPlan>,
    ) -> (Result<ProfileSelection, RenderError>, Value) {
        trace::capture_json(|| {
            let _scope = trace::Scope::begin(root, false);
            let selector = if old { legacy_select } else { select };
            selector(
                root,
                &self.ledger,
                &self.witnesses,
                &self.pure,
                &self.circuits,
                prechecked,
            )
        })
    }
    fn checked(
        &self,
        root: &StatefulCircuit,
    ) -> Result<TypedPlan, effect_plan::DiagnosticPlanError> {
        effect_plan::lower_with_calls_diagnostic(
            root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
}
fn syntax(selection: &ProfileSelection) -> (Option<String>, Option<String>) {
    (
        selection
            .organizer_steps
            .as_ref()
            .map(|steps| quote::quote!(#(#steps)*).to_string()),
        selection
            .typed_result
            .as_ref()
            .map(|value| value.to_token_stream().to_string()),
    )
}
fn equivalent(c: &Contract, name: &str) -> (ProfileSelection, Value) {
    let context = Context::new(c);
    let root = context.circuits[name];
    let (old, old_trace) = context.selected(root, true, None);
    let (current, current_trace) = context.selected(root, false, None);
    let old = old.unwrap();
    let current = current.unwrap();
    assert!(
        old.organizer_steps.is_some(),
        "old complete selector: {name}"
    );
    assert_eq!(syntax(&old), syntax(&current), "{name}");
    assert!(current.typed_result.is_some());
    assert!(old.shielded_send_rejection.is_none());
    assert!(current.shielded_send_rejection.is_none());
    assert!(old.unit_composition_rejection.is_none());
    assert!(current.unit_composition_rejection.is_none());
    assert_eq!(
        current_trace[0]["attempts"],
        serde_json::json!([
            {"route":"algebra.ordinary", "produced_candidate":true}
        ])
    );
    (current, old_trace)
}
fn public(c: &Contract, name: &str) {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == name)
        .unwrap();
    assert!(capability.recorded && capability.observed_call);
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["circuit"] == name && v["phase"] == "public_emission" && v["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.ordinary");
    assert!(row["action_dispatch"].is_null());
    assert!(row["return_dispatch"].is_null());
}
