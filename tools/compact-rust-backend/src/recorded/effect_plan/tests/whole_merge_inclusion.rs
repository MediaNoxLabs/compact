// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite complete-oracle inclusion, separate from valid algebra expansions.
//! Rendering here does not establish generated-crate compilation or proof validity.
use super::*;
use crate::recorded::admission_probe::trace;
use serde_json::{Value as Json, json};

#[path = "whole_merge_inclusion/boundaries.rs"]
mod boundaries;
#[path = "whole_merge_inclusion/owned.rs"]
mod owned;
#[path = "whole_merge_inclusion/pure.rs"]
mod pure;
#[path = "whole_merge_inclusion/variants.rs"]
mod variants;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/shielded-merge-schema20-ir.json"
    ))
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
        crate::recorded::typed_plan::lower_shielded_merge(
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
    if r.internal {
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
        assert!(item.unwrap().is_supported(), "{label}");
        assert_eq!(helper_rows.as_array().unwrap().len(), 1);
        assert_eq!(helper_rows[0]["body_owner"], "algebra.ordinary");
        assert_eq!(helper_rows[0]["phase"], "helper_planning");
        assert_eq!(helper_rows[0]["helper"], true);
    } else {
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
            .filter(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect();
        assert_eq!(rows.len(), 1, "{label}");
        assert_eq!(rows[0]["body_owner"], "algebra.ordinary", "{label}");
        let attempts: Vec<_> = rows[0]["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["route"].as_str().unwrap())
            .collect();
        assert_eq!(
            attempts
                .iter()
                .filter(|&&r| r == "algebra.ordinary")
                .count(),
            1
        );
        assert!(!attempts.contains(&"selector.shielded-merge"));
    }
    capture(c, name, label, &output.source);
}
// Optional exact-output evidence for a separate native crate compilation gate.
// Each process requires a fresh destination; each positive owns one directory.
fn capture(c: &Contract, name: &str, label: &str, generated: &str) {
    let Some(root) = std::env::var_os("COMPACT_WHOLE_MERGE_PROBE_OUTPUT") else {
        return;
    };
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root).unwrap();
    let directory = root.join(format!("{index:03}"));
    std::fs::create_dir(&directory).expect("shielded-merge export requires a fresh destination");
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
            "old_whole_merge_accepted": true, "checked_plan_succeeded": true,
            "native_render_succeeded": true, "body_owner": "algebra.ordinary",
            "qualification": "IR/render evidence only; compilation and proof are separate"
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
fn retained_whole_merge_envelopes_compose() {
    let c = source();
    for name in ["merge_qualified", "receive_then_merge"] {
        included(&c, name, &format!("retained-{name}"));
    }
}
fn both(c: &Contract, label: &str) {
    for name in ["merge_qualified", "receive_then_merge"] {
        included(c, name, &format!("{label}-{name}"));
    }
}
