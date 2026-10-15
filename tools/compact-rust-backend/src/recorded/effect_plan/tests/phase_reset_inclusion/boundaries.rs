// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn broader_unit_interfaces_are_expansion_not_old_phase_inclusion() {
    for mode in 0..3 {
        let mut c = source();
        let r = circuit_mut(&mut c, "advance");
        match mode {
            0 => r.internal = true,
            1 => r.parameters.push(Parameter {
                name: "unused_phase_flag".into(),
                ty: Type::Boolean,
            }),
            2 => r.return_value = StateReturn::Expression { value: Expr::Unit },
            _ => unreachable!(),
        }
        expansion(&c, &format!("broader interface {mode}"));
    }
}

#[test]
fn valid_depth_and_true_reset_arguments_are_expansion_not_inclusion() {
    let mut c = source();
    let tree = c
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "committed_votes")
        .unwrap();
    let LedgerFieldKind::MerkleTree { depth, .. } = &mut tree.declaration else {
        unreachable!()
    };
    *depth = 9;
    expansion(&c, "valid alternate declared tree depth");
    let mut c = source();
    edit_circuit(&mut c, "advance", "circuit_call", |v| {
        v["arguments"] = json!([{"kind":"boolean","value":true}]);
    });
    expansion(&c, "true reset call");
}

#[test]
fn source_scope_operand_slot_and_recursive_helper_errors_are_refused() {
    for mode in 0..6 {
        let mut c = source();
        match mode {
            0 => edit_circuit(&mut c, "advance", "circuit_call", |v| {
                v["arguments"] = json!([{"kind":"parameter","name":"escaping_phase_local"}]);
            }),
            1 => edit_circuit(&mut c, "advance", "unsigned_add", |v| {
                v["right"] = json!({"kind":"boolean","value":true});
            }),
            2 => edit_circuit(&mut c, "reset_state", "counter_reset", |v| {
                v["index"] = json!(0);
            }),
            3 => circuit_mut(&mut c, "reset_state")
                .actions
                .push(StateAction::CircuitCall {
                    name: "reset_state".into(),
                    arguments: vec![Expr::Boolean { value: false }],
                }),
            4 => {
                let helper = c
                    .circuits
                    .iter_mut()
                    .find(|r| r.name == "successor")
                    .unwrap();
                helper.body = Expr::Call {
                    name: "successor".into(),
                    arguments: vec![Expr::Parameter {
                        name: helper.parameters[0].name.clone(),
                    }],
                };
            }
            5 => edit_circuit(&mut c, "advance", "enum_variant", |v| {
                v["variant"] = json!("missing_phase");
            }),
            _ => unreachable!(),
        }
        let env = Environment::new(&c);
        assert!(
            env.old(root(&c)).is_none(),
            "malformed source {mode}: old refusal"
        );
        assert!(
            matches!(env.checked(root(&c)), Err(PlanError::Unsupported(_))),
            "malformed source {mode}: source refusal"
        );
    }
}

#[test]
fn malformed_nominal_enum_layout_is_rejected_against_complete_declarations() {
    let mut c = source();
    edit_circuit(&mut c, "advance", "enum_variant", |v| {
        v["ty"]["variants"] = json!(["setup", "commit"]);
    });
    let env = Environment::new(&c);
    assert!(env.old(root(&c)).is_none());
    assert!(
        matches!(env.checked(root(&c)), Err(PlanError::Unsupported(gap)) if gap.detail.contains("conflict") || gap.detail.contains("canonical"))
    );
}
