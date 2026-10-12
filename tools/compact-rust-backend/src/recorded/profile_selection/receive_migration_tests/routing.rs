// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn selected_trace(c: &Contract, r: &StatefulCircuit) -> serde_json::Value {
    let maps = Context::new(c);
    let (_, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(r, false);
        let result = maps.selected(r).unwrap();
        // Selector evidence only; full capability is checked separately by public_owner.
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
#[test]
fn actual_receive_owner_is_once_and_stops_later_profiles() {
    let c = source();
    let row = selected_trace(&c, root(&c));
    assert_eq!(row["body_owner"], "algebra.shielded-receive");
    let a = attempts(&row);
    assert_eq!(
        a.iter()
            .filter(|&&s| s == "algebra.shielded-receive")
            .count(),
        1
    );
    assert!(!a.contains(&"selector.shielded-receive"));
    assert!(!a.contains(&"algebra.voting-commit"));
}
#[test]
fn earlier_organizer_and_composite_owner_prevent_receive_attempts() {
    let mut organizer: Contract = serde_json::from_str(include_str!(
        "../../../../tests/welcome-organizer-schema13-ir.json"
    ))
    .unwrap();
    organizer.schema_version = crate::ir::SCHEMA_VERSION;
    let root = organizer
        .stateful_circuits
        .iter()
        .find(|r| !r.internal)
        .unwrap();
    let row = selected_trace(&organizer, root);
    assert_eq!(row["body_owner"], "organizer.organizer");
    assert!(!attempts(&row).contains(&"algebra.shielded-receive"));
    let composite: Contract = serde_json::from_str(include_str!(
        "../../../../tests/field-observation-schema20-ir.json"
    ))
    .unwrap();
    let root = composite
        .stateful_circuits
        .iter()
        .find(|r| r.name == "snapshot")
        .unwrap();
    let row = selected_trace(&composite, root);
    assert_eq!(row["body_owner"], "algebra.composite-values");
    assert!(!attempts(&row).contains(&"algebra.shielded-receive"));
}
#[test]
fn unsupported_body_continues_independent_suffix_and_invariant_stays_fatal() {
    let mut c = source();
    root_mut(&mut c).actions = vec![StateAction::Expression {
        value: Expr::ConstructJubjubPoint {
            x: Box::new(Expr::JubjubPointX {
                value: Box::new(Expr::HashToCurve {
                    value: Box::new(parameter("coin")),
                }),
            }),
            y: Box::new(Expr::FieldLiteral { value: "1".into() }),
        },
    }];
    assert!(Context::new(&c).domain(root(&c)).unwrap().is_none());
    let row = selected_trace(&c, root(&c));
    let a = attempts(&row);
    assert!(
        a.iter()
            .position(|&s| s == "algebra.shielded-receive")
            .unwrap()
            < a.iter()
                .position(|&s| s == "algebra.voting-commit")
                .unwrap()
    );
    assert!(!a.contains(&"selector.shielded-receive"));
    let gap = RecordingGap {
        code: super::super::super::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "forged.receive".into(),
        detail: "wrong output operand".into(),
    };
    assert!(
        resolve_checked_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    assert!(
        matches!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))),Err(RenderError::RecordingPlanInvariant { path,detail }) if path=="forged.receive"&&detail=="wrong output operand")
    );
}
#[test]
fn rollout_boundary_keeps_only_declared_interface_constraints() {
    for mode in 0..4 {
        let mut c = source();
        let r = root_mut(&mut c);
        match mode {
            0 => r.parameters.clear(),
            1 => r.parameters.push(Parameter {
                name: "other".into(),
                ty: Type::Boolean,
            }),
            2 => r.return_value = StateReturn::Expression { value: Expr::Unit },
            3 => {
                r.parameters[0].ty = structure("DifferentCoin", Type::Field);
            }
            _ => unreachable!(),
        }
        assert!(
            Context::new(&c).domain(root(&c)).unwrap().is_none(),
            "interface {mode}"
        );
    }
    for no_effects in [false, true] {
        let mut c = source();
        root_mut(&mut c).actions = if no_effects {
            vec![]
        } else {
            helper_mut(&mut c).actions.clone()
        };
        assert!(
            Context::new(&c).old(root(&c)).is_none(),
            "explicit expansion"
        );
        assert!(Context::new(&c).domain(root(&c)).unwrap().is_some());
        public_owner(&c, "accept");
    }
}
#[test]
fn actual_source_witness_and_cell_composition_is_a_separate_expansion() {
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-receive-composition-schema20-ir.json"
    ))
    .unwrap();
    let r = c
        .stateful_circuits
        .iter()
        .find(|r| r.name == "acceptSelected")
        .unwrap();
    let maps = Context::new(&c);
    assert!(maps.old(r).is_none());
    assert!(maps.domain(r).unwrap().is_some());
    public_owner(&c, "acceptSelected");
}
