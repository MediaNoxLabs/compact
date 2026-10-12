// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Recording preserves execution; empty public traces remain a preparation concern.
use super::*;

#[test]
fn complete_old_empty_guard_consumer_moves_to_checked_execution() {
    // Exact synthetic whole input authenticated and executed on signed597 before
    // policy changed; this is a backend IR consumer, not a fresh frontend source.
    let mut c: Contract = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests-rust-backend/empty-recording-ir/contract-ir.json"
    )))
    .unwrap();
    let i = c
        .stateful_circuits
        .iter()
        .position(|c| c.name == "recordFreshEnough")
        .unwrap();
    c.stateful_circuits.swap(0, i);
    let root = &c.stateful_circuits[0];
    let parameters = root
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                p.name.as_str(),
                (&p.ty, crate::ident(&format!("arg{i}")).unwrap()),
            )
        })
        .collect();
    let pure = c.circuits.iter().map(|c| (c.name.as_str(), c)).collect();
    let calls = c
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    assert!(
        crate::recorded::legacy::guarded_pure::closed_guarded_struct_pure_steps(
            root,
            &parameters,
            &pure,
            &calls,
        )
        .unwrap()
        .is_some(),
        "old guard premise remains explicit"
    );
    assert!(!effect_plan::has_public_program(&checked(&c)));
    assert_supported(&c);
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = output.unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "recordFreshEnough")
        .unwrap();
    assert!(cap.recorded && cap.observed_call);
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["circuit"] == "recordFreshEnough"
                && r["phase"] == "public_emission"
                && r["helper"] == false
        })
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.unit-composition");
    assert!(
        row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "guard.guarded-struct-pure"),
        "guard retirement is a separate slice"
    );
    let ProfileAttempt::Admitted(plan) = attempt_contract(&c).unwrap() else {
        unreachable!()
    };
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert!(text.find("pure_circuits").unwrap() < text.find("pure continuation").unwrap());
    assert!(!text.contains("record_increment"));
}

#[test]
fn empty_and_nested_private_execution_keep_validated_effects() {
    let mut c = subject();
    c.stateful_circuits[0].actions.clear();
    assert!(!effect_plan::has_public_program(&checked(&c)));
    assert_supported(&c);
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "private".into(),
        parameters: vec![],
        result: Type::Unit,
    });
    c.stateful_circuits[1].actions = vec![
        StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "before callback".into(),
        },
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::Expression {
                value: Expr::WitnessCall {
                    name: "private".into(),
                    arguments: vec![],
                },
            }),
            otherwise: Box::new(StateAction::Assert {
                condition: Expr::Boolean { value: false },
                message: "unselected guard".into(),
            }),
        },
    ];
    c.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "helper".into(),
        arguments: vec![],
    }];
    assert!(!effect_plan::has_public_program(&checked(&c)));
    let ProfileAttempt::Admitted(plan) = attempt_contract(&c).unwrap() else {
        panic!("valid private composition")
    };
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert_eq!(text.matches("try_witness_metered").count(), 1);
    assert!(text.find("before callback").unwrap() < text.find("try_witness_metered").unwrap());
    assert!(text.contains("unselected guard"));
    assert!(
        text.contains("if") && text.contains("else"),
        "lazy branch remains explicit"
    );
    // Both branches still require valid declarations, even without public work.
    let StateAction::If { otherwise, .. } = &mut c.stateful_circuits[1].actions[1] else {
        unreachable!()
    };
    **otherwise = StateAction::Expression {
        value: Expr::Parameter {
            name: "escaped".into(),
        },
    };
    rejected(&c);
}
