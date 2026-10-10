// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn voting_owner_runs_once_and_stops_later_profiles() {
    let c = source();
    let row = selected_trace(&c, root(&c));
    assert_eq!(row["body_owner"], "algebra.voting-commit");
    let a = attempts(&row);
    assert_eq!(
        a.iter().filter(|&&s| s == "algebra.voting-commit").count(),
        1
    );
    assert!(!a.contains(&"selector.voting-commit"));
    assert!(!a.contains(&"selector.funded-mint"));
}

#[test]
fn earlier_organizer_composite_and_receive_owners_prevent_voting_attempts() {
    for (json, name, owner) in [
        (
            include_str!("../../../../tests/welcome-organizer-schema13-ir.json"),
            None,
            "organizer.organizer",
        ),
        (
            include_str!("../../../../tests/field-observation-schema20-ir.json"),
            Some("snapshot"),
            "algebra.composite-values",
        ),
        (
            include_str!("../../../../tests/shielded-receive-schema20-ir.json"),
            Some("accept"),
            "algebra.shielded-receive",
        ),
    ] {
        let mut c: Contract = serde_json::from_str(json).unwrap();
        c.schema_version = crate::ir::SCHEMA_VERSION;
        let r = c
            .stateful_circuits
            .iter()
            .find(|r| name.map_or(!r.internal, |name| r.name == name))
            .unwrap();
        let row = selected_trace(&c, r);
        assert_eq!(row["body_owner"], owner);
        assert!(!attempts(&row).contains(&"algebra.voting-commit"));
    }
}

#[test]
fn unsupported_body_preserves_later_independent_attempt_order() {
    let mut c = source();
    let r = root_mut(&mut c);
    let coin = r.parameters[1].name.clone();
    r.actions = vec![StateAction::Expression {
        value: Expr::PersistentHash {
            value: Box::new(Expr::Parameter { name: coin }),
        },
    }];
    assert!(Context::new(&c).domain(root(&c)).unwrap().is_none());
    let row = selected_trace(&c, root(&c));
    let a = attempts(&row);
    let voting = a
        .iter()
        .position(|&s| s == "algebra.voting-commit")
        .unwrap();
    assert_eq!(
        &a[voting..voting + 4],
        [
            "algebra.voting-commit",
            "selector.funded-mint",
            "selector.guarded-deposit",
            "selector.reset-payout"
        ]
    );
    assert!(!a.contains(&"selector.voting-commit"));
}

#[test]
fn unsupported_refuses_but_invalid_plan_keeps_its_diagnostic() {
    let gap = RecordingGap {
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.voting.operand".into(),
        detail: "wrong output operand".into(),
    };
    assert!(
        resolve_checked_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    assert!(
        matches!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))),
        Err(RenderError::RecordingPlanInvariant { path, detail })
        if path == "forged.voting.operand" && detail == "wrong output operand")
    );
}
