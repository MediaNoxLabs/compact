// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite complete-oracle inclusion, separate from valid algebra expansions.
//! Rendering here does not establish generated-crate compilation or proof validity.
use super::*;
use crate::recorded::admission_probe::trace;
use serde_json::{Value as Json, json};

#[path = "guarded_deposit_inclusion/boundaries.rs"]
mod boundaries;
#[path = "guarded_deposit_inclusion/grammar.rs"]
mod grammar;
#[path = "guarded_deposit_inclusion/owned.rs"]
mod owned;
#[path = "guarded_deposit_inclusion/pure.rs"]
mod pure;
#[path = "guarded_deposit_inclusion/variants.rs"]
mod variants;
fn source(full: bool) -> Contract {
    serde_json::from_str(if full {
        include_str!("../../../../tests/micro-dao-source-schema20-ir.json")
    } else {
        include_str!("../../../../tests/micro-dao-set-topic-schema20-ir.json")
    })
    .unwrap()
}
fn circuit_mut<'a>(c: &'a mut Contract, name: &str) -> &'a mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|r| r.name == name)
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
        crate::recorded::typed_plan::lower_guarded_deposit(
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
fn included(c: &Contract, name: &str, label: &str) -> Json {
    let env = Environment::new(c);
    let r = env.stateful[name];
    assert!(
        env.old(r).is_some(),
        "{label}: mandatory complete old admission"
    );
    let checked = env
        .checked(r)
        .unwrap_or_else(|e| panic!("{label}: candidate gap {e:?}"));
    render(&checked).unwrap_or_else(|e| panic!("{label}: checked render {e:?}"));
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap_or_else(|e| panic!("{label}: full native rendering {e:?}"));
    let actual = if r.internal {
        let (item, helper_rows) = trace::capture_json(|| {
            let _phase = trace::HelperPlanning::begin();
            crate::recorded::render_recorded_helper(
                r,
                &env.ledger,
                &env.witnesses,
                &env.pure,
                &env.stateful,
                &std::collections::HashSet::new(),
            )
        });
        assert!(
            item.unwrap().is_supported(),
            "{label}: current helper supported"
        );
        helper_rows
    } else {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|v| v.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call, "{label}");
        Json::Array(
            rows.as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["circuit"] == name
                        && row["phase"] == "public_emission"
                        && row["helper"] == false
                })
                .cloned()
                .collect(),
        )
    };
    assert_eq!(actual.as_array().unwrap().len(), 1, "{label}");
    assert_eq!(
        actual[0]["body_owner"], "algebra.guarded-deposit",
        "{label}"
    );
    capture(c, name, label, &output.source, &actual[0]);
    actual[0]["body_owner"].clone()
}
// Optional exact-output evidence for a separate native crate compilation gate.
// Each process requires a fresh destination; each positive owns one directory.
fn capture(c: &Contract, name: &str, label: &str, generated: &str, owner: &Json) {
    let Some(root) = std::env::var_os("COMPACT_GUARDED_DEPOSIT_PROBE_OUTPUT") else {
        return;
    };
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root).unwrap();
    let directory = root.join(format!("{index:03}"));
    std::fs::create_dir(&directory).expect("guarded-deposit export requires a fresh destination");
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
            "old_whole_guarded_deposit_accepted": true, "checked_plan_succeeded": true,
            "native_render_succeeded": true, "selected_owner": owner,
            "qualification": "Candidate IR/render plus current selected full source; before cutover current owner may remain legacy. Raw compilation and proof separate."
        }))
        .unwrap(),
    )
    .unwrap();
}

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn returned_mut<'a>(c: &'a mut Contract, name: &str) -> &'a mut Expr {
    let StateReturn::Expression { value } = &mut circuit_mut(c, name).return_value else {
        unreachable!()
    };
    value
}

#[test]
fn both_retained_sources_use_checked_guarded_deposit_owner() {
    for (full, owner) in [
        (true, "algebra.guarded-deposit"),
        (false, "algebra.guarded-deposit"),
    ] {
        assert_eq!(
            included(&source(full), "set_topic", &format!("baseline-full-{full}")),
            owner
        );
    }
}
