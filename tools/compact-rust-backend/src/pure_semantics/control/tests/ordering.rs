// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn control_first_errors_precede_later_children_and_output_callbacks() {
    let unknown = reference("missing");
    let pairs = [
        assertion(field("1")),
        branch(field("1"), unknown.clone(), unknown.clone()),
        Expr::Sequence {
            steps: vec![field("1")],
            value: Box::new(unknown.clone()),
        },
        local("bad-name", Type::Field, unknown.clone(), unknown.clone()),
        local("x", Type::Boolean, field("1"), unknown.clone()),
    ];
    for (index, source) in pairs.iter().enumerate() {
        let mut consumer = Consumer::default();
        let refusal = check(source, &HashMap::new(), &HashMap::new(), &mut consumer)
            .err()
            .unwrap();
        assert!(!consumer.outputs.contains(&"control"));
        assert!(consumer.locals.is_empty());
        if index == 3 {
            assert!(format!("{refusal:?}").contains("InvalidIdentifier"));
        } else {
            assert!(matches!(refusal, RenderError::TypeMismatch { .. }));
        }
    }
    let mismatch = local(
        "x",
        Type::Unsigned { max: "bad".into() },
        field("1"),
        unknown.clone(),
    );
    assert!(error(&mismatch, &HashMap::new()).contains("TypeMismatch"));
    let bad_annotation = Type::Unsigned { max: "bad".into() };
    let bindings = HashMap::from([("u", (&bad_annotation, ()))]);
    let matched = local("x", bad_annotation.clone(), reference("u"), unknown);
    assert!(error(&matched, &bindings).contains("InvalidUnsignedMaximum"));
}

#[test]
fn equal_arm_occurrences_visit_once_before_join_and_coalescing() {
    let source = branch(
        boolean(true),
        assertion(boolean(false)),
        assertion(boolean(false)),
    );
    let Expr::If {
        then, otherwise, ..
    } = &source
    else {
        unreachable!()
    };
    let mut consumer = Consumer::default();
    assert_eq!(
        check(&source, &HashMap::new(), &HashMap::new(), &mut consumer).unwrap(),
        Type::Unit
    );
    for node in [&source, then.as_ref(), otherwise.as_ref()] {
        assert_eq!(
            consumer
                .visits
                .iter()
                .filter(|&&key| key == std::ptr::from_ref(node) as usize)
                .count(),
            1
        );
    }
    assert_eq!(
        consumer
            .outputs
            .iter()
            .filter(|&&kind| kind == "control")
            .count(),
        3
    );
    let failure = branch(boolean(true), field("1"), reference("missing"));
    assert!(error(&failure, &HashMap::new()).contains("UnknownParameter"));
}
