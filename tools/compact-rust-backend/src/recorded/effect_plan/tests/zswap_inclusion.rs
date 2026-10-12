// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite complete-oracle inclusion, separate from valid algebra expansions.
//! Rendering here does not establish generated-crate compilation or proof validity.
use super::*;
use crate::recorded::admission_probe::trace;
use serde_json::{Value as Json, json};

#[path = "zswap_inclusion/boundaries.rs"]
mod boundaries;
#[path = "zswap_inclusion/variants.rs"]
mod variants;

// Public Unit signatures now use the shared checked interface, including the
// Uint128 member of qualified coins. Expression effect returns still use Zswap.
fn expected_owner(root: &StatefulCircuit) -> &'static str {
    if root.return_value == StateReturn::Unit {
        "algebra.unit-composition"
    } else {
        "algebra.zswap"
    }
}

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/native-zswap-intents-schema20-ir.json"
    ))
    .unwrap()
}
fn circuit_mut<'a>(c: &'a mut Contract, name: &str) -> &'a mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|r| r.name == name)
        .unwrap()
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|r| r.name == "flow")
        .unwrap()
}
struct Environment<'a> {
    ledger: std::collections::HashMap<&'a str, &'a LedgerField>,
    witnesses: std::collections::HashMap<&'a str, &'a WitnessDeclaration>,
    pure: std::collections::HashMap<&'a str, &'a PureCircuit>,
    stateful: std::collections::HashMap<&'a str, &'a StatefulCircuit>,
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
        crate::recorded::zswap_plan::lower(r, &self.ledger, &self.stateful, &self.witnesses)
    }

    fn checked(&self, r: &StatefulCircuit) -> Result<CheckedPlan, PlanError> {
        check_with_calls(r, &self.ledger, &self.witnesses, &self.pure, &self.stateful)
    }
}
fn included(c: &Contract, name: &str, label: &str) {
    let env = Environment::new(c);
    let r = env.stateful[name];
    assert!(
        env.old(r).is_some(),
        "{label}: complete frozen oracle first"
    );
    let plan = env
        .checked(r)
        .unwrap_or_else(|e| panic!("{label}: checked {e:?}"));
    render(&plan).unwrap_or_else(|e| panic!("{label}: render {e:?}"));
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap_or_else(|e| panic!("{label}: full native/recorded render {e:?}"));
    let export = output
        .capabilities
        .circuits
        .iter()
        .find(|e| e.name == name)
        .unwrap();
    assert!(export.recorded && export.observed_call, "{label}");
    let rows: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .collect();
    assert_eq!(rows.len(), 1, "{label}");
    let owner = expected_owner(r);
    if owner == "algebra.unit-composition" {
        assert!(
            has_public_program(&plan),
            "Unit requires public program evidence"
        );
    }
    assert_eq!(rows[0]["body_owner"], owner, "{label}");
    let attempts: Vec<_> = rows[0]["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect();
    assert_eq!(attempts.iter().filter(|&&r| r == owner).count(), 1);
    if owner == "algebra.unit-composition" {
        assert!(
            !attempts.contains(&"algebra.zswap"),
            "earlier Unit admission stops the later adapter"
        );
    }

    capture(c, name, label, &output.source, owner);
}
// Optional exact-output evidence for a separate native crate compilation gate.
// Each process requires a fresh destination; each positive owns one directory.
fn capture(c: &Contract, name: &str, label: &str, generated: &str, owner: &str) {
    let Some(root) = std::env::var_os("COMPACT_ZSWAP_PROBE_OUTPUT") else {
        return;
    };
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root).unwrap();
    let directory = root.join(format!("{index:03}"));
    std::fs::create_dir(&directory).expect("Zswap export requires a fresh destination");
    std::fs::write(
        directory.join("compact-rust-ir.json"),
        serde_json::to_vec_pretty(c).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("lib.rs"), generated).unwrap();
    std::fs::write(
        directory.join("case.json"),
        serde_json::to_vec_pretty(&json!({
            "index": index, "label": label, "root": name,
            "old_whole_zswap_accepted": true, "checked_plan_succeeded": true,
            "native_render_succeeded": true, "body_owner": owner,
            "qualification": "IR/render evidence only; compilation and proof are separate"
        }))
        .unwrap(),
    )
    .unwrap();
}

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn expression(value: Expr) -> StateAction {
    StateAction::Expression { value }
}
fn output() -> Expr {
    Expr::CreateZswapOutput {
        coin: Box::new(parameter("coin")),
        recipient: Box::new(parameter("recipient")),
    }
}
fn substitute(value: &mut Json, from: &str, replacement: &Expr) {
    if value["kind"] == "parameter" && value["name"] == from {
        *value = serde_json::to_value(replacement).unwrap();
        return;
    }
    match value {
        Json::Object(fields) => fields
            .values_mut()
            .for_each(|v| substitute(v, from, replacement)),
        Json::Array(values) => values
            .iter_mut()
            .for_each(|v| substitute(v, from, replacement)),
        _ => {}
    }
}
fn replace_uses(c: &mut Contract, name: &str, from: &str, replacement: Expr) {
    let r = circuit_mut(c, name);
    let mut value = serde_json::to_value(&*r).unwrap();
    substitute(&mut value, from, &replacement);
    *r = serde_json::from_value(value).unwrap();
}
#[test]
fn whole_old_zswap_flow_source_composes() {
    included(&source(), "flow", "retained conditional flow");
}

#[test]
fn whole_old_zswap_other_retained_sources_compose() {
    for (text, name) in [
        (
            include_str!("../../../../tests/canonical-output-order-schema20-ir.json"),
            "distribute",
        ),
        (
            include_str!("../../../../tests/zswap-transfer-schema20-ir.json"),
            "transfer",
        ),
    ] {
        let c: Contract = serde_json::from_str(text).unwrap();
        included(&c, name, &format!("retained {name}"));
    }
}
