// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite complete-old inclusion. Raw Rust compilation and proofs are separate gates.
use super::*;
use crate::recorded::admission_probe::trace;
use serde_json::{Value as Json, json};

#[path = "reset_payout_inclusion/boundaries.rs"]
mod boundaries;
#[path = "reset_payout_inclusion/bridges.rs"]
mod bridges;
#[path = "reset_payout_inclusion/reset.rs"]
mod reset;
#[path = "reset_payout_inclusion/structure.rs"]
mod structure;

fn source() -> Json {
    serde_json::from_str(include_str!(
        "../../../../tests/micro-dao-source-schema20-ir.json"
    ))
    .unwrap()
}
fn circuit<'a>(source: &'a mut Json, name: &str) -> &'a mut Json {
    source["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn root(source: &mut Json) -> &mut Json {
    &mut circuit(source, "cash_out")["return_value"]["value"]
}
fn visit(value: &mut Json, change: &mut impl FnMut(&mut Json)) {
    change(value);
    match value {
        Json::Object(object) => {
            for child in object.values_mut() {
                visit(child, change);
            }
        }
        Json::Array(array) => {
            for child in array {
                visit(child, change);
            }
        }
        _ => {}
    }
}
fn replace(value: &mut Json, old: &str, new: &str) {
    visit(value, &mut |part| {
        if part.as_str() == Some(old) {
            *part = json!(new);
        }
    });
}
fn first(value: &mut Json, change: &mut impl FnMut(&mut Json) -> bool) -> bool {
    if change(value) {
        return true;
    }
    match value {
        Json::Object(object) => object.values_mut().any(|child| first(child, change)),
        Json::Array(array) => array.iter_mut().any(|child| first(child, change)),
        _ => false,
    }
}
fn reference(name: &str) -> Json {
    json!({"kind":"parameter","name":name})
}
fn call(name: &str, arguments: Vec<Json>) -> Json {
    json!({"kind":"call","name":name,"arguments":arguments})
}
fn binding(name: &str, ty: Json, value: Json) -> Json {
    json!({"name":name,"ty":ty,"value":value})
}
fn prepend(source: &mut Json, bindings: Vec<Json>) {
    let body = root(source).take();
    *root(source) = json!({"kind":"let","bindings":bindings,"body":body});
}
fn pure(source: &mut Json, name: &str, parameters: Vec<Json>, result: Json, body: Json) {
    source["circuits"].as_array_mut().unwrap().push(json!({
        "name":name,"internal":true,"parameters":parameters,"result":result,"body":body
    }));
}
struct Environment<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    stateful: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Environment<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            ledger: c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            witnesses: c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
            pure: c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
            stateful: c
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn old(&self, r: &StatefulCircuit) -> Option<crate::recorded::typed_plan::TypedPlan> {
        crate::recorded::typed_plan::lower_reset_payout(
            r,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.stateful,
        )
    }
    fn checked(&self, r: &StatefulCircuit) -> Result<CheckedPlan, PlanError> {
        check_with_calls(r, &self.ledger, &self.witnesses, &self.pure, &self.stateful)
    }
}
fn included(source: Json, name: &'static str, label: &'static str) {
    // Complete DAO closure can exceed the default test worker stack.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let c: Contract = serde_json::from_value(source).unwrap();
            let env = Environment::new(&c);
            let r = env.stateful[name];
            assert!(
                env.old(r).is_some(),
                "{label}: complete old lower must accept first"
            );
            render(
                &env.checked(r)
                    .unwrap_or_else(|e| panic!("{label}: checked gap {e:?}")),
            )
            .unwrap();
            let output = current(&c, name, label);
            capture(&c, name, label, &output);
        })
        .unwrap()
        .join()
        .unwrap();
}
fn current(c: &Contract, name: &str, label: &str) -> String {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap_or_else(|e| panic!("{label}: full native render {e:?}"));
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    assert!(cap.recorded && cap.observed_call, "{label}");
    let actual: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["circuit"] == name && row["phase"] == "public_emission" && row["helper"] == false
        })
        .collect();
    assert_eq!(actual.len(), 1, "{label}");
    assert_eq!(actual[0]["body_owner"], "algebra.ordinary", "{label}");
    output.source
}
fn capture(c: &Contract, name: &str, label: &str, rust: &str) {
    let Some(base) = std::env::var_os("COMPACT_RESET_PAYOUT_PROBE_OUTPUT") else {
        return;
    };
    let base = std::path::PathBuf::from(base);
    std::fs::create_dir_all(&base).unwrap();
    let directory = base.join(label);
    std::fs::create_dir(&directory).expect("reset-payout export requires a fresh destination");
    std::fs::write(
        directory.join("compact-rust-ir.json"),
        serde_json::to_vec_pretty(c).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("lib.rs"), rust).unwrap();
    std::fs::write(
        directory.join("case.json"),
        serde_json::to_vec_pretty(&json!({
            "label":label,"root":name,"old_whole_reset_accepted":true,
            "checked_plan_succeeded":true,"native_render_succeeded":true,
            "body_owner":"algebra.ordinary",
            "qualification":"IR/render only; raw consumer compilation and proofs are separate"
        }))
        .unwrap(),
    )
    .unwrap();
}
