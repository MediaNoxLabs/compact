// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole opaque-Set admission includes actual shared-helper discovery.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
use admission_probe::trace;
use legacy::opaque_set::closed_guarded_opaque_set_mutation_steps;
use serde_json::{Value, json};
#[path = "opaque_set_migration_tests/boundaries.rs"]
mod boundaries;
#[path = "opaque_set_migration_tests/inclusion.rs"]
mod inclusion;
fn fixture() -> Value {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../tests/asset-watch-write-schema13-ir.json"
    ))
    .unwrap();
    v["schema_version"] = json!(SCHEMA_VERSION);
    v
}
fn full_fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/asset-registry-schema20-ir.json")).unwrap()
}
fn contract(v: Value) -> Contract {
    serde_json::from_value(v).unwrap()
}
fn named<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn actions(v: &mut Value) -> &mut Value {
    &mut named(v, "setWatch")["actions"][0]["action"]["action"]["actions"]
}
fn rename(v: &mut Value, pairs: &[(&str, &str)]) {
    match v {
        Value::String(s) => {
            if let Some((_, to)) = pairs.iter().find(|(from, _)| *from == s.as_str()) {
                *s = (*to).into();
            }
        }
        Value::Object(m) => m.values_mut().for_each(|v| rename(v, pairs)),
        Value::Array(a) => a.iter_mut().for_each(|v| rename(v, pairs)),
        _ => {}
    }
}
fn edit(v: &mut Value, f: &mut impl FnMut(&mut serde_json::Map<String, Value>)) {
    match v {
        Value::Object(m) => {
            f(m);
            m.values_mut().for_each(|v| edit(v, f));
        }
        Value::Array(a) => a.iter_mut().for_each(|v| edit(v, f)),
        _ => {}
    }
}
struct Context<'a> {
    c: &'a Contract,
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            c,
            ledger: c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect(),
            witnesses: c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect(),
            pure: c.circuits.iter().map(|x| (x.name.as_str(), x)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|x| (x.name.as_str(), x))
                .collect(),
        }
    }
    fn root(&self) -> &StatefulCircuit {
        self.circuits["setWatch"]
    }
    fn discovered(&self) -> Result<HashSet<String>, RenderError> {
        let (names, _) = plan_recorded_helpers(
            &self.c.stateful_circuits,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )?;
        // Each shared name must actually emit independently with empty sharing.
        // A hand-written set of helper names would bypass this old obligation.
        for name in &names {
            assert!(
                render_recorded_helper(
                    self.circuits[name.as_str()],
                    &self.ledger,
                    &self.witnesses,
                    &self.pure,
                    &self.circuits,
                    &HashSet::new()
                )?
                .is_supported()
            );
        }
        Ok(names)
    }
    fn old(&self) -> Result<Option<Vec<syn::Stmt>>, RenderError> {
        let names = self.discovered()?;
        let parameters = self
            .root()
            .parameters
            .iter()
            .map(|p| Ok((p.name.as_str(), (&p.ty, ident(&p.name)?))))
            .collect::<Result<HashMap<_, _>, RenderError>>()?;
        closed_guarded_opaque_set_mutation_steps(
            self.root(),
            &parameters,
            &self.ledger,
            &self.circuits,
            &names,
        )
    }
    fn checked(&self) -> Result<typed_plan::TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(
            self.root(),
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
}
fn assert_current(c: &Contract) {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|c| c.name == "setWatch" && c.recorded && c.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["circuit"] == "setWatch" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.unit-composition");
    let attempts = row["attempts"].as_array().unwrap();
    assert!(
        !attempts
            .iter()
            .any(|a| a["route"] == "guard.opaque-set-mutation")
    );
    assert!(
        !attempts
            .iter()
            .any(|a| a["route"] == "guard.guarded-map-read"),
        "Map-read admission is retired independently of this old Set oracle"
    );
    assert!(
        !attempts
            .iter()
            .any(|a| a["route"] == "guard.opaque-map-write"),
        "Map-write admission is retired independently of this old oracle"
    );
    assert!(
        attempts
            .iter()
            .any(|a| a["route"] == "guard.guarded-struct-pure")
    );
}
fn assert_old_positive(c: &Contract, label: &str) {
    let cx = Context::new(c);
    assert!(cx.old().unwrap().is_some(), "complete old {label}");
    let plan = cx.checked().unwrap();
    let steps = &plan.steps;
    let t = quote::quote!(#(#steps)*).to_string();
    assert!(
        t.contains("record_insert") && t.contains("record_remove"),
        "both mutation branches survive"
    );
    assert!(
        t.find("watch mutation must be Add or Drop").unwrap()
            < t.find("record does not exist").unwrap()
    );
    assert!(t.find("record does not exist").unwrap() < t.find("record_insert").unwrap());
    assert_current(c);
}
