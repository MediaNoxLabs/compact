// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Send is subsumed by generic checked composition; diagnostics survive fallback.
// Concrete diagnostic control: opaque persistent input is native-IR valid but
// rejected by the Compact frontend. Fixed Field commitments are now supported.
use super::*;
use crate::ir::{Contract, Expr, LocalBinding, StateAction, StateReturn, Type};
use crate::recorded::admission_probe::trace;
fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/shielded-send-schema20-ir.json"
    ))
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
    fn attempt(&self, root: &StatefulCircuit) -> ProfileAttempt<TypedPlan> {
        checked_composite_attempt(
            root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
        .unwrap()
    }
    fn old(&self, root: &StatefulCircuit) -> ProfileAttempt<TypedPlan> {
        typed_plan::lower_shielded_send_checked(
            root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn selected(&self, root: &StatefulCircuit) -> ProfileSelection {
        select(
            root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
        .unwrap()
    }
}
fn no_old_attempt(rows: &serde_json::Value) {
    for row in rows.as_array().unwrap() {
        assert!(
            !row["attempts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["route"] == "selector.shielded-send")
        );
    }
}
#[test]
fn complete_old_roots_keep_generic_public_and_helper_owners_without_send_attempt() {
    let c = source();
    let maps = Context::new(&c);
    for root in &c.stateful_circuits {
        assert!(matches!(maps.old(root), ProfileAttempt::Admitted(_)));
        assert!(matches!(maps.attempt(root), ProfileAttempt::Admitted(_)));
    }
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(
        output
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    no_old_attempt(&rows);
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["internal"] == false)
    {
        assert_eq!(row["body_owner"], "algebra.composite-values");
    }
    let root = maps.circuits["sendShielded"];
    let (item, rows) = trace::capture_json(|| {
        let _phase = trace::HelperPlanning::begin();
        crate::recorded::render_recorded_helper(
            root,
            &maps.ledger,
            &maps.witnesses,
            &maps.pure,
            &maps.circuits,
            &std::collections::HashSet::new(),
        )
    });
    assert!(item.unwrap().is_supported());
    no_old_attempt(&rows);
    assert_eq!(rows[0]["body_owner"], "algebra.composite-values");
}
#[test]
fn checked_composition_does_not_require_old_send_effect_counts_or_empty_actions() {
    for actionful in [false, true] {
        let mut c = source();
        let root = &mut c.stateful_circuits[1];
        root.return_value = StateReturn::Expression {
            value: Expr::Default {
                ty: root.result.clone(),
            },
        };
        if actionful {
            root.actions
                .push(StateAction::Expression { value: Expr::Unit });
        }
        let maps = Context::new(&c);
        let root = &c.stateful_circuits[1];
        assert!(!matches!(maps.old(root), ProfileAttempt::Admitted(_)));
        assert!(matches!(maps.attempt(root), ProfileAttempt::Admitted(_)));
        let selected = maps.selected(root);
        assert!(selected.typed_result.is_some());
        assert!(selected.shielded_send_rejection.is_none());
        crate::render_with_capabilities(&c).unwrap();
    }
}
#[test]
fn actual_composite_gap_survives_only_for_send_interface_without_legacy_reanalysis() {
    let mut c = source();
    let root = &mut c.stateful_circuits[1];
    let StateReturn::Expression { value } = &mut root.return_value else {
        unreachable!()
    };
    *value = Expr::Let {
        bindings: vec![LocalBinding {
            name: "hidden".into(),
            ty: Type::Field,
            value: Expr::TransientHash {
                value: Box::new(Expr::PersistentCommit {
                    value: Box::new(Expr::Default {
                        ty: Type::OpaqueString,
                    }),
                    opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
                }),
            },
        }],
        body: Box::new(value.clone()),
    };
    let maps = Context::new(&c);
    let root = &c.stateful_circuits[1];
    let ProfileAttempt::Rejected(checked) = maps.attempt(root) else {
        panic!("unsupported persistent-commit input")
    };
    let ProfileAttempt::Rejected(old) = maps.old(root) else {
        panic!("old child refusal")
    };
    assert_eq!(old.ir_node, "Expr::PersistentCommit");
    assert_eq!(checked.path, "return_value.value.bindings[0].value.value");
    assert_eq!(checked.ir_node, "RecordedPlan");
    assert!(checked.detail.contains("persistent commitment"));
    let selected = maps.selected(root);
    assert!(selected.typed_result.is_none());
    let retained = selected.shielded_send_rejection.unwrap();
    assert_eq!(retained, checked);
    // An unrelated Struct still uses generic composition, but cannot acquire a
    // send-specific refinement of legacy return diagnostics.
    let mut c = source();
    let root = &mut c.stateful_circuits[1];
    root.result = Type::Struct {
        name: "Other".into(),
        fields: vec![],
    };
    root.return_value = StateReturn::Expression {
        value: Expr::Let {
            bindings: vec![LocalBinding {
                name: "hidden".into(),
                ty: Type::Field,
                value: Expr::TransientHash {
                    value: Box::new(Expr::PersistentCommit {
                        value: Box::new(Expr::Default {
                            ty: Type::OpaqueString,
                        }),
                        opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
                    }),
                },
            }],
            body: Box::new(Expr::Default {
                ty: root.result.clone(),
            }),
        },
    };
    assert!(
        Context::new(&c)
            .selected(&c.stateful_circuits[1])
            .shielded_send_rejection
            .is_none()
    );
}
#[test]
fn checked_composite_invariants_are_fatal_and_unsupported_is_retained() {
    let gap = RecordingGap {
        resource: None,
        code: super::super::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "return_value.value".into(),
        detail: "forged send operation".into(),
    };
    assert!(
        matches!(resolve_checked_attempt(Err(effect_plan::PlanError::Unsupported(gap.clone()))).unwrap(),ProfileAttempt::Rejected(ref retained) if retained==&gap)
    );
    assert!(matches!(
        resolve_checked_attempt(Err(effect_plan::PlanError::Invalid(gap))),
        Err(RenderError::RecordingPlanInvariant { .. })
    ));
}
