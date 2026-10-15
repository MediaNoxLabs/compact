// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole generic admission, native validity and checked ownership are separate premises.
use super::*;
use crate::ir::{Contract, CounterAmount, LocalBinding, Parameter, StructField};
use admission_probe::trace;
use serde_json::json;

#[path = "generic_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "generic_migration_tests/calls.rs"]
mod calls;
#[path = "generic_migration_tests/diagnostics.rs"]
mod diagnostics;
#[path = "generic_migration_tests/execution_results.rs"]
mod execution_results;
#[path = "generic_migration_tests/leaves.rs"]
mod leaves;
#[path = "generic_migration_tests/lexical.rs"]
mod lexical;
#[path = "generic_migration_tests/merkle.rs"]
mod merkle;
#[path = "generic_migration_tests/storage.rs"]
mod storage;

fn uint64() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn field(n: u64) -> Expr {
    Expr::FieldLiteral {
        value: n.to_string(),
    }
}
fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}
fn var(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn binding(name: &str, ty: Type, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty,
        value,
    }
}
fn root(result: Type, return_value: StateReturn, actions: Vec<StateAction>) -> Contract {
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
            parameters: vec![],
            result,
            actions,
            return_value,
        }],
    }
}
fn unit(actions: Vec<StateAction>) -> Contract {
    root(Type::Unit, StateReturn::Unit, actions)
}
fn slot(c: &mut Contract, id: &str, declaration: LedgerFieldKind) {
    c.ledger_fields.push(LedgerField {
        source: None,
        id: id.into(),
        index: c.ledger_fields.len() as u8,
        path: vec![],
        declaration,
    });
}
fn assertion() -> StateAction {
    StateAction::Assert {
        condition: Expr::Boolean { value: true },
        message: "guard".into(),
    }
}
fn write() -> StateAction {
    StateAction::CellWrite {
        field: "cell".into(),
        index: 0,
        value: field(1),
    }
}
fn cell_contract() -> Contract {
    let mut c = unit(vec![write()]);
    slot(&mut c, "cell", LedgerFieldKind::Cell { ty: Type::Field });
    c
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
    fn old(&self, c: &Contract, name: &str) -> Result<RecordingOutcome<syn::Item>, RenderError> {
        let (shared, _) = generic_reference::plan_recorded_helpers(
            &c.stateful_circuits,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )?;
        generic_reference::render_recorded_circuit(
            self.circuits[name],
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            &shared,
        )
    }
    fn checked(&self, name: &str) -> Result<recording_plan::TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(
            self.circuits[name],
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
}
fn included(c: &Contract, name: &str, label: &str) -> recording_plan::TypedPlan {
    let cx = Context::new(c);
    assert!(
        matches!(cx.old(c, name), Ok(RecordingOutcome::Supported(_))),
        "{label}: complete old renderer refused"
    );
    let (rendered, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let rendered = rendered.unwrap_or_else(|e| panic!("{label}: native/public render {e:?}"));
    let plan = cx
        .checked(name)
        .unwrap_or_else(|e| panic!("{label}: checked {e:?}"));
    let cap = rendered
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == name)
        .unwrap();
    assert!(cap.recorded && cap.observed_call, "{label}");
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["circuit"] == name && v["phase"] == "public_emission" && v["helper"] == false)
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.ordinary", "{label}");
    assert!(
        row["action_dispatch"].is_null() && row["return_dispatch"].is_null(),
        "{label}"
    );
    plan
}
fn methods(plan: &recording_plan::TypedPlan) -> Vec<String> {
    struct Visitor(Vec<String>);
    impl syn::visit_mut::VisitMut for Visitor {
        fn visit_expr_method_call_mut(&mut self, n: &mut syn::ExprMethodCall) {
            self.0.push(n.method.to_string());
            syn::visit_mut::visit_expr_method_call_mut(self, n);
        }
    }
    let mut v = Visitor(vec![]);
    for mut s in plan.steps.clone() {
        syn::visit_mut::VisitMut::visit_stmt_mut(&mut v, &mut s);
    }
    v.0
}
fn parse_fixture(s: &str) -> Contract {
    let mut c: Contract = serde_json::from_str(s).unwrap();
    c.schema_version = crate::ir::SCHEMA_VERSION;
    c
}
