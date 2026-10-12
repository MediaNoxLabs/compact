// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Merge is subsumed by earlier composition; ordinary fallback diagnostics stay intact.
use super::*;
use crate::ir::{Contract, Expr, LocalBinding, StateAction, StateReturn, Type};
use crate::recorded::{RecordingGapCode, admission_probe::trace};
fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/shielded-merge-schema20-ir.json"
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
    fn old(&self, root: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower_shielded_merge(
            root,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
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
fn root_mut(c: &mut Contract) -> &mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == "merge_qualified")
        .unwrap()
}
fn no_old_attempt(rows: &serde_json::Value) {
    for row in rows.as_array().unwrap() {
        assert!(
            !row["attempts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["route"] == "selector.shielded-merge")
        );
    }
}
#[test]
fn whole_old_merge_exports_and_internal_helper_keep_existing_composite_owner() {
    let c = source();
    let maps = Context::new(&c);
    for name in ["mergeCoin", "merge_qualified", "receive_then_merge"] {
        let root = maps.circuits[name];
        assert!(
            maps.old(root).is_some(),
            "{name}: complete old merge control"
        );
        assert!(
            matches!(maps.attempt(root), ProfileAttempt::Admitted(_)),
            "{name}"
        );
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
    for name in ["merge_qualified", "receive_then_merge"] {
        let owners: Vec<_> = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["circuit"] == name
                    && row["phase"] == "public_emission"
                    && row["helper"] == false
            })
            .collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0]["body_owner"], "algebra.composite-values");
        assert!(
            !owners[0]["attempts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["route"] == "selector.funded-mint"),
            "earlier composite winner must short circuit later families"
        );
    }
    let (item, rows) = trace::capture_json(|| {
        let _phase = trace::HelperPlanning::begin();
        crate::recorded::render_recorded_helper(
            maps.circuits["mergeCoin"],
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
fn composable_coin_result_does_not_require_old_merge_effect_counts() {
    for actionful in [false, true] {
        let mut c = source();
        let root = root_mut(&mut c);
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
        let root = maps.circuits["merge_qualified"];
        assert!(
            maps.old(root).is_none(),
            "explicit general-composition expansion"
        );
        assert!(matches!(maps.attempt(root), ProfileAttempt::Admitted(_)));
        assert!(maps.selected(root).typed_result.is_some());
        crate::render_with_capabilities(&c).unwrap();
    }
}
fn unsupported_hash() -> Expr {
    Expr::TransientHash {
        value: Box::new(Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "0".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        }),
    }
}

#[test]
fn rejected_merge_keeps_action_precedence_and_generic_return_diagnostic() {
    for actionful in [false, true] {
        let mut c = source();
        let root = root_mut(&mut c);
        let StateReturn::Expression { value } = &mut root.return_value else {
            unreachable!()
        };
        *value = Expr::Let {
            bindings: vec![LocalBinding {
                name: "hidden".into(),
                ty: Type::Field,
                value: unsupported_hash(),
            }],
            body: Box::new(value.clone()),
        };
        if actionful {
            root.actions.push(StateAction::Expression {
                value: unsupported_hash(),
            });
        }
        // Preserve the previous default-hash case as an explicit composition gain.
        let mut supported = c.clone();
        let supported_root = root_mut(&mut supported);
        let StateReturn::Expression {
            value: Expr::Let { bindings, .. },
        } = &mut supported_root.return_value
        else {
            unreachable!()
        };
        let default_hash = Expr::TransientHash {
            value: Box::new(Expr::Default {
                ty: Type::OpaqueString,
            }),
        };
        bindings[0].value = default_hash.clone();
        if actionful {
            *supported_root.actions.last_mut().unwrap() = StateAction::Expression {
                value: default_hash,
            };
        }
        let positive_maps = Context::new(&supported);
        let positive_root = positive_maps.circuits["merge_qualified"];
        assert!(positive_maps.old(positive_root).is_none());
        assert!(matches!(
            positive_maps.attempt(positive_root),
            ProfileAttempt::Admitted(_)
        ));
        let maps = Context::new(&c);
        let root = maps.circuits["merge_qualified"];
        assert!(maps.old(root).is_none());
        assert!(matches!(maps.attempt(root), ProfileAttempt::Rejected(_)));
        let selected = maps.selected(root);
        assert!(selected.typed_result.is_none());
        assert!(
            selected.shielded_send_rejection.is_none(),
            "a coin result must not acquire send-result diagnostics"
        );
        let expected = if actionful {
            RecordingGap::action(&root.actions[0], "actions[0]".into())
        } else {
            RecordingGap::returned(&root.return_value)
        };
        let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        let output = output.unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "merge_qualified")
            .unwrap();
        assert!(!capability.recorded && !capability.observed_call);
        let gap = capability.recording_unavailable.as_ref().unwrap();
        assert_eq!(
            gap.code,
            if actionful {
                RecordingGapCode::UnsupportedAction
            } else {
                RecordingGapCode::UnsupportedReturn
            }
        );
        assert_eq!(gap, &expected);
        no_old_attempt(&rows);
    }
}
#[test]
fn native_validation_still_precedes_merge_recording_diagnostics() {
    let mut c = source();
    root_mut(&mut c).name = "bad-name".into();
    assert!(
        matches!(crate::render_with_capabilities(&c).err(), Some(RenderError::Located { error, .. }) if *error == RenderError::InvalidIdentifier("bad-name".into()))
    );
}
