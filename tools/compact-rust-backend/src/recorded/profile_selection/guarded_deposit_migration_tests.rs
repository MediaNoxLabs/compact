// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Guarded deposit admission follows its declared interface; composition validates the body.
use super::*;
use crate::ir::{Contract, Expr, Parameter, ReturnPlan, StateAction, StateReturn, Type};
use crate::recorded::admission_probe::trace;

#[path = "guarded_deposit_migration_tests/domain.rs"]
mod domain;
#[path = "guarded_deposit_migration_tests/routing.rs"]
mod routing;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/micro-dao-source-schema20-ir.json"
    ))
    .unwrap()
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "set_topic")
        .unwrap()
}
fn root_mut(c: &mut Contract) -> &mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == "set_topic")
        .unwrap()
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
    fn old(&self, c: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower_guarded_deposit(
            c,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn domain(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_guarded_deposit_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    // Historical staged-priority oracle; production emission is checked separately.
    fn legacy_selected(&self, c: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        legacy_select(
            c,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
    }
}
fn legacy_selected_trace(c: &Contract, r: &StatefulCircuit) -> serde_json::Value {
    let maps = Context::new(c);
    let (_, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(r, false);
        let result = maps.legacy_selected(r).unwrap();
        // This captures selector ownership only; public_owner checks actual emission.
        scope.finish(&Ok::<_, RenderError>(
            crate::recorded::RecordingOutcome::Supported(()),
        ));
        result
    });
    assert_eq!(rows.as_array().unwrap().len(), 1);
    rows[0].clone()
}
fn attempts(row: &serde_json::Value) -> Vec<&str> {
    row["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect()
}
fn public_owner(c: &Contract, name: &str) {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    let exports = output
        .capabilities
        .circuits
        .iter()
        .filter(|e| e.name == name)
        .collect::<Vec<_>>();
    assert_eq!(exports.len(), 1);
    assert!(exports[0].recorded && exports[0].observed_call);
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "algebra.ordinary");
    assert_eq!(rows[0]["outcome"]["status"], "supported");
    assert_eq!(
        attempts(rows[0])
            .iter()
            .filter(|&&s| s == "algebra.ordinary")
            .count(),
        1
    );
}

#[test]
fn retained_full_source_is_old_first_and_public_emission_uses_checked_owner() {
    let c = source();
    let maps = Context::new(&c);
    assert!(maps.old(root(&c)).is_some(), "whole frozen oracle first");
    assert!(maps.domain(root(&c)).unwrap().is_some());
    public_owner(&c, "set_topic");
}

#[test]
fn renamed_and_internal_declarations_keep_the_declared_domain() {
    for internal in [false, true] {
        let mut c = source();
        let r = root_mut(&mut c);
        r.name = "renamed_deposit".into();
        r.internal = internal;
        let r = c
            .stateful_circuits
            .iter()
            .find(|r| r.name == "renamed_deposit")
            .unwrap();
        let maps = Context::new(&c);
        assert!(maps.old(r).is_some(), "whole frozen oracle first");
        assert!(maps.domain(r).unwrap().is_some());
        assert_eq!(
            legacy_selected_trace(&c, r)["body_owner"],
            "algebra.guarded-deposit"
        );
        if internal {
            let (item, rows) = trace::capture_json(|| {
                let _phase = trace::HelperPlanning::begin();
                crate::recorded::render_recorded_helper(
                    r,
                    &maps.ledger,
                    &maps.witnesses,
                    &maps.pure,
                    &maps.circuits,
                    &std::collections::HashSet::new(),
                )
            });
            assert!(item.unwrap().is_supported());
            assert_eq!(rows.as_array().unwrap().len(), 1);
            assert_eq!(rows[0]["phase"], "helper_planning");
            assert_eq!(rows[0]["helper"], true);
            assert_eq!(rows[0]["body_owner"], "algebra.ordinary");
        } else {
            public_owner(&c, &r.name);
        }
    }
}
