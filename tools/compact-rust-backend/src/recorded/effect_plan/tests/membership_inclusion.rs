// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite inclusion across every final membership acceptance alternative.
//! Whole old admission precedes checked lowering and complete native rendering.
use super::*;
use crate::recorded::admission_probe::trace;
use serde_json::{Value as Json, json};

#[path = "membership_inclusion/families.rs"]
mod families;
#[path = "membership_inclusion/variants.rs"]
mod variants;

fn source(text: &str) -> Contract {
    let mut c: Contract = serde_json::from_str(text).unwrap();
    // Historical typed snapshots are controls, not new frontend provenance.
    c.schema_version = crate::ir::SCHEMA_VERSION;
    c
}
fn bboard() -> Contract {
    source(include_str!("../../../../tests/bboard-schema20-ir.json"))
}
fn counter() -> Contract {
    source(include_str!(
        "../../../../tests/counter-less-than-schema15-ir.json"
    ))
}
fn assertions() -> Contract {
    source(include_str!(
        "../../../../tests/stateful-assert-schema20-ir.json"
    ))
}
fn root_mut<'a>(c: &'a mut Contract, name: &str) -> &'a mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|r| r.name == name)
        .unwrap()
}
fn rename(value: &mut Json, before: &str, after: &str) {
    match value {
        Json::String(text) if text == before => *text = after.into(),
        Json::Array(items) => {
            for child in items {
                rename(child, before, after);
            }
        }
        Json::Object(fields) => {
            for child in fields.values_mut() {
                rename(child, before, after);
            }
        }
        _ => {}
    }
}
fn renamed(c: &Contract, names: &[(&str, &str)]) -> Contract {
    let mut json = serde_json::to_value(c).unwrap();
    for (before, after) in names {
        rename(&mut json, before, after);
    }
    serde_json::from_value(json).unwrap()
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
    fn old(&self, root: &StatefulCircuit) -> Option<crate::recorded::typed_plan::TypedPlan> {
        crate::recorded::typed_plan::lower(
            root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.stateful,
        )
    }
}
fn included(c: &Contract, name: &str, label: &str, family: &str) {
    let env = Environment::new(c);
    let root = env.stateful[name];
    assert!(
        env.old(root).is_some(),
        "{label}: complete old {family} admission is mandatory"
    );
    let checked = check_with_calls(root, &env.ledger, &env.witnesses, &env.pure, &env.stateful)
        .unwrap_or_else(|e| panic!("{label}: checked gap {e:?}"));
    render(&checked).unwrap_or_else(|e| panic!("{label}: checked render {e:?}"));
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap_or_else(|e| panic!("{label}: complete native rendering {e:?}"));
    let owner = if root.internal {
        let (item, rows) = trace::capture_json(|| {
            let _phase = trace::HelperPlanning::begin();
            crate::recorded::render_recorded_helper(
                root,
                &env.ledger,
                &env.witnesses,
                &env.pure,
                &env.stateful,
                &std::collections::HashSet::new(),
            )
        });
        assert!(item.unwrap().is_supported(), "{label}: internal recording");
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), 1, "{label}");
        assert_eq!(rows[0]["phase"], "helper_planning");
        rows[0]["body_owner"].clone()
    } else {
        let cap = output
            .capabilities
            .circuits
            .iter()
            .find(|v| v.name == name)
            .unwrap();
        assert!(
            cap.recorded && cap.observed_call,
            "{label}: actual export recording"
        );
        let owners = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["circuit"] == name
                    && row["phase"] == "public_emission"
                    && row["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(owners.len(), 1, "{label}");
        owners[0]["body_owner"].clone()
    };
    assert!(
        owner.is_string(),
        "{label}: actual routing must be attributed"
    );
    // The owner may be an earlier organizer. Whole-old admission is independently
    // checked above; a first accepted trace is never substituted for that premise.
    if let Some(path) = std::env::var_os("COMPACT_MEMBERSHIP_PROBE_OUTPUT") {
        let base = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join(label);
        std::fs::create_dir(&dir).expect("use a fresh membership export directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(c).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
        std::fs::write(dir.join("case.json"), serde_json::to_vec_pretty(&json!({"family":family,"root":name,"internal":root.internal,"old_admitted":true,"checked":true,"actual_owner":owner,"scope":"complete typed-IR rendering; compile/runtime/proof separate"})).unwrap()).unwrap();
    }
}
