// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// The direct VectorMap child is a native-valid recording refusal; fixed Field
// commitments themselves are supported. Do not replace this with malformed types.
use super::*;

#[test]
fn deposit_owner_keeps_priority_and_stops_later_families() {
    let c = source();
    let row = selected_trace(&c, root(&c));
    assert_eq!(row["body_owner"], "algebra.guarded-deposit");
    let a = attempts(&row);
    assert_eq!(
        a.iter()
            .filter(|&&s| s == "algebra.guarded-deposit")
            .count(),
        1
    );
    assert!(!a.contains(&"selector.guarded-deposit"));
    assert!(!a.contains(&"selector.reset-payout"));
    let at = a
        .iter()
        .position(|&s| s == "algebra.guarded-deposit")
        .unwrap();
    assert_eq!(a[at - 1], "algebra.voting-commit");
    assert!(!a.contains(&"selector.funded-mint"));
}

#[test]
fn earlier_composite_and_receive_owners_prevent_deposit_attempts() {
    for (json, name, owner) in [
        (
            include_str!("../../../../tests/field-observation-schema20-ir.json"),
            "snapshot",
            "algebra.composite-values",
        ),
        (
            include_str!("../../../../tests/shielded-receive-schema20-ir.json"),
            "accept",
            "algebra.shielded-receive",
        ),
    ] {
        let c: Contract = serde_json::from_str(json).unwrap();
        let r = c.stateful_circuits.iter().find(|r| r.name == name).unwrap();
        let row = selected_trace(&c, r);
        assert_eq!(row["body_owner"], owner);
        assert!(!attempts(&row).contains(&"algebra.guarded-deposit"));
    }
}

#[test]
fn unsupported_body_falls_through_without_hiding_first_action_diagnostic() {
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
    // Keep the positive default above, then exercise the unchanged commitment boundary.
    root_mut(&mut c).actions[0] = StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(crate::recorded::test_support::unsupported_fixed_input()),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    };
    let maps = Context::new(&c);
    assert!(maps.old(root(&c)).is_none());
    assert!(maps.domain(root(&c)).unwrap().is_none());
    let row = selected_trace(&c, root(&c));
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
    assert!(!a.contains(&"selector.guarded-deposit"));
    let output = crate::render_with_capabilities(&c).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == "set_topic")
        .unwrap();
    assert!(!cap.recorded && !cap.observed_call);
    assert_eq!(
        cap.recording_unavailable.as_ref().unwrap(),
        &RecordingGap::action(&root(&c).actions[0], "actions[0]".into())
    );
}
