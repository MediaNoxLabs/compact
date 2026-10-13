// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Shielded payout is subsumed by existing generic composition; no new route is added.
// The direct VectorFoldCall child is a native-valid recording refusal; fixed Field
// commitments themselves are supported. Do not replace this with malformed types.
use super::*;
use crate::ir::{Contract, Expr, StateAction, StateReturn, Type};
use crate::recorded::admission_probe::trace;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/coracle-source-schema20-ir.json"
    ))
    .unwrap()
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "withdraw")
        .unwrap()
}
fn root_mut(c: &mut Contract) -> &mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == "withdraw")
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
        typed_plan::lower_shielded_payout(
            c,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn domain(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_composite_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
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
fn both_complete_old_payout_domains_keep_actual_composite_owners_without_fallback() {
    let c = source();
    let maps = Context::new(&c);
    for name in ["withdraw", "concede"] {
        let r = c.stateful_circuits.iter().find(|r| r.name == name).unwrap();
        assert!(maps.old(r).is_some(), "whole frozen oracle first: {name}");
        assert!(maps.domain(r).unwrap().is_some());
        public_owner(&c, name);
        let row = legacy_selected_trace(&c, r);
        let a = attempts(&row);
        assert!(!a.contains(&"selector.shielded-payout"));
        assert!(!a.contains(&"selector.funded-mint"));
    }
}

#[test]
fn generic_composition_does_not_require_old_payout_effect_counts() {
    let mut c = source();
    let r = root_mut(&mut c);
    r.return_value = StateReturn::Expression {
        value: Expr::Default {
            ty: r.result.clone(),
        },
    };
    let maps = Context::new(&c);
    assert!(maps.old(root(&c)).is_none());
    assert!(maps.domain(root(&c)).unwrap().is_some());
    public_owner(&c, "withdraw");
    assert!(!attempts(&legacy_selected_trace(&c, root(&c))).contains(&"selector.shielded-payout"));
}

#[test]
fn unsupported_action_keeps_fallback_order_and_first_action_diagnostic() {
    let mut c = source();
    root_mut(&mut c).actions.insert(
        0,
        StateAction::Expression {
            value: Expr::Default {
                ty: Type::OpaqueString,
            },
        },
    );
    assert!(Context::new(&c).old(root(&c)).is_none());
    assert!(Context::new(&c).domain(root(&c)).unwrap().is_some());
    crate::recorded::test_support::register_unsupported_fold(&mut c);
    // Keep the positive default above, then exercise the direct fold boundary.
    root_mut(&mut c).actions[0] = StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(crate::recorded::test_support::unsupported_fixed_input()),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    };
    let maps = Context::new(&c);
    assert!(maps.old(root(&c)).is_none());
    assert!(maps.domain(root(&c)).unwrap().is_none());
    let row = legacy_selected_trace(&c, root(&c));
    let a = attempts(&row);
    let at = a
        .iter()
        .position(|&s| s == "algebra.guarded-deposit")
        .unwrap();
    assert_eq!(
        &a[at..at + 3],
        [
            "algebra.guarded-deposit",
            "algebra.terminal-returns",
            "algebra.unit-actions"
        ]
    );
    assert!(!a.contains(&"selector.shielded-payout"));
    let output = crate::render_with_capabilities(&c).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == "withdraw")
        .unwrap();
    assert!(!cap.recorded && !cap.observed_call);
    crate::recorded::test_support::assert_checked_fold_refusal(
        &c,
        root(&c),
        RecordingGap::action(&root(&c).actions[0], "actions[0]".into()),
        "actions[0].value.value",
    );
}
