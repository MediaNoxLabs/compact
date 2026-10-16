// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// The direct VectorFoldCall child is a native-valid recording refusal; fixed Field
// commitments themselves are supported. Do not replace this with malformed types.
// Direct legacy_selected traces below preserve the staged predecessor priority laws.
// Current public/helper emission assertions, where present, use algebra.ordinary.
use super::*;

#[test]
fn voting_owner_runs_once_and_stops_later_profiles() {
    let c = source();
    let row = legacy_selected_trace(&c, root(&c));
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
fn earlier_membership_composite_and_receive_owners_prevent_voting_attempts() {
    for (json, name, owner) in [
        (
            include_str!("../../../../tests/welcome-organizer-schema13-ir.json"),
            Some("add_participant"),
            "algebra.membership",
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
        let row = legacy_selected_trace(&c, r);
        assert_eq!(row["body_owner"], owner);
        let a = attempts(&row);
        assert_eq!(a.iter().filter(|&&route| route == owner).count(), 1);
        assert_eq!(a.last().copied(), Some(owner));
        assert!(!a.iter().any(|route| route.starts_with("organizer.")));
        assert!(!a.contains(&"algebra.voting-commit"));
    }
}

#[test]
fn unsupported_body_preserves_later_independent_attempt_order() {
    let mut c = source();
    crate::recorded::test_support::register_unsupported_fold(&mut c);
    let r = root_mut(&mut c);
    r.actions = vec![StateAction::Expression {
        value: Expr::HashToCurve {
            value: Box::new(Expr::PersistentCommit {
                value: Box::new(crate::recorded::test_support::unsupported_fixed_input()),
                opening: Box::new(Expr::FieldToBytes32 {
                    value: Box::new(Expr::FieldLiteral { value: "1".into() }),
                }),
            }),
        },
    }];
    assert!(Context::new(&c).domain(root(&c)).unwrap().is_none());
    let row = legacy_selected_trace(&c, root(&c));
    let a = attempts(&row);
    let voting = a
        .iter()
        .position(|&s| s == "algebra.voting-commit")
        .unwrap();
    assert_eq!(
        &a[voting..voting + 4],
        [
            "algebra.voting-commit",
            "algebra.guarded-deposit",
            "algebra.terminal-returns",
            "algebra.unit-actions"
        ]
    );
    assert!(!a.contains(&"selector.voting-commit"));
}

#[test]
fn unsupported_refuses_but_invalid_plan_keeps_its_diagnostic() {
    let gap = RecordingGap {
        resource: None,
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
