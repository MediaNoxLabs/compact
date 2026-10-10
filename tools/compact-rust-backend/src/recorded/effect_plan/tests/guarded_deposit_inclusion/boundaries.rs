// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn edit_first(v: &mut Json, kind: &str, f: &mut impl FnMut(&mut Json)) -> bool {
    if v["kind"] == kind {
        f(v);
        return true;
    }
    match v {
        Json::Object(map) => map.values_mut().any(|v| edit_first(v, kind, f)),
        Json::Array(items) => items.iter_mut().any(|v| edit_first(v, kind, f)),
        _ => false,
    }
}
#[test]
fn source_seed_shadowing_is_old_refusal_not_an_automatic_generalized_error() {
    let mut c = source(true);
    let r = circuit_mut(&mut c, "set_topic");
    let seed = r.parameters[2].clone();
    r.actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: seed.name.clone(),
            ty: seed.ty,
            value: parameter(&seed.name),
        }],
        action: Box::new(StateAction::Sequence {
            actions: std::mem::take(&mut r.actions),
        }),
    }];
    let env = Environment::new(&c);
    assert!(env.old(env.stateful["set_topic"]).is_none());
}
#[test]
fn immutable_coin_slot_identity_and_escaped_binding_fail_both_analyzers() {
    for index in [false, true] {
        let mut c = source(true);
        let r = circuit_mut(&mut c, "set_topic");
        let mut actions = serde_json::to_value(&r.actions).unwrap();
        assert!(edit_first(&mut actions, "cell_write_coin", &mut |v| {
            if index {
                v["index"] = json!(254)
            } else {
                v["coin"] = json!({"kind":"parameter","name":"escaped_branch_value"})
            }
        }));
        r.actions = serde_json::from_value(actions).unwrap();
        let env = Environment::new(&c);
        assert!(env.old(env.stateful["set_topic"]).is_none());
        assert!(env.checked(env.stateful["set_topic"]).is_err());
    }
}
#[test]
fn wrong_receive_operand_is_not_a_whole_old_admitted_control() {
    let mut c = source(true);
    let r = circuit_mut(&mut c, "set_topic");
    let mut actions = serde_json::to_value(&r.actions).unwrap();
    assert!(edit_first(
        &mut actions,
        "circuit_call",
        &mut |v| v["arguments"][0] = json!({"kind":"parameter","name":"beneficiary_param"})
    ));
    r.actions = serde_json::from_value(actions).unwrap();
    let env = Environment::new(&c);
    assert!(env.old(env.stateful["set_topic"]).is_none());
    assert!(env.checked(env.stateful["set_topic"]).is_err());
}

#[test]
fn missing_or_duplicated_path_store_is_an_old_shape_refusal_only() {
    for duplicate in [false, true] {
        let mut c = source(true);
        let r = circuit_mut(&mut c, "set_topic");
        let mut actions = serde_json::to_value(&r.actions).unwrap();
        assert!(edit_first(&mut actions, "cell_write_coin", &mut |v| {
            *v = if duplicate {
                json!({"kind":"sequence","actions":[v.clone(),v.clone()]})
            } else {
                json!({"kind":"sequence","actions":[]})
            };
        }));
        r.actions = serde_json::from_value(actions).unwrap();
        let env = Environment::new(&c);
        assert!(env.old(env.stateful["set_topic"]).is_none());
        // The future checked route may intentionally admit either composition.
    }
}
#[test]
fn a_pure_callee_cannot_capture_the_callers_sibling_binding() {
    let mut c = source(true);
    let helper = c
        .circuits
        .iter_mut()
        .find(|p| p.name == "public_key")
        .unwrap();
    // public_key returns Bytes32; caller's apk has the same type but is not in
    // this pure helper's formal scope. Type coincidence does not grant access.
    helper.body = parameter("apk");
    let env = Environment::new(&c);
    assert!(env.old(env.stateful["set_topic"]).is_none());
    assert!(env.checked(env.stateful["set_topic"]).is_err());
}
