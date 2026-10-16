// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn harmless_root_sequences_empty_lets_and_topic_aliases_preserve_whole_admission() {
    for mode in 0..3 {
        let mut c = source(true);
        let r = circuit_mut(&mut c, "set_topic");
        let original = StateAction::Sequence {
            actions: std::mem::take(&mut r.actions),
        };
        r.actions = vec![match mode {
            0 => StateAction::Sequence {
                actions: vec![original],
            },
            1 => StateAction::Let {
                bindings: vec![],
                action: Box::new(original),
            },
            _ => StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "unused_topic_alias".into(),
                    ty: Type::OpaqueString,
                    value: parameter(&r.parameters[0].name),
                }],
                action: Box::new(original),
            },
        }];
        included(&c, "set_topic", &format!("root-wrapper-{mode}"));
    }
}
#[test]
fn root_helper_formal_and_ledger_names_are_not_the_old_mechanism() {
    fn visit(v: &mut Json) {
        match v {
            Json::Object(map) => {
                for (key, value) in map.iter_mut() {
                    if ["name", "id", "field"].contains(&key.as_str())
                        && let Json::String(name) = value
                        && [
                            "set_topic",
                            "mergeCoin",
                            "mergeCoinImmediate",
                            "receiveShielded",
                            "upcastQualifiedCoin",
                            "topic_param",
                            "beneficiary_param",
                            "seedCoin",
                            "pot",
                            "pot_has_coin",
                        ]
                        .contains(&name.as_str())
                    {
                        *name = format!("deposit_probe_{name}");
                    }
                    visit(value);
                }
            }
            Json::Array(items) => items.iter_mut().for_each(visit),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(source(true)).unwrap();
    visit(&mut v);
    included(
        &serde_json::from_value(v).unwrap(),
        "deposit_probe_set_topic",
        "renamed-full-source",
    );
}
#[test]
fn internal_root_is_native_valid_via_a_public_forwarding_call() {
    let mut c = source(true);
    let r = circuit_mut(&mut c, "set_topic");
    r.internal = true;
    let mut forward = r.clone();
    forward.internal = false;
    forward.name = "forward_topic".into();
    forward.actions = vec![StateAction::CircuitCall {
        name: r.name.clone(),
        arguments: r.parameters.iter().map(|p| parameter(&p.name)).collect(),
    }];
    c.stateful_circuits.push(forward);
    included(&c, "set_topic", "internal-full-source");
}
#[test]
fn arbitrary_one_bytes32_product_is_not_limited_to_the_wallet_key_name() {
    let mut c = source(true);
    let r = circuit_mut(&mut c, "set_topic");
    let canonical = r.parameters[1].ty.clone();
    let argument = r.parameters[1].name.clone();
    let custom = Type::Struct {
        name: "DepositBeneficiary".into(),
        fields: vec![StructField {
            name: "payload".into(),
            ty: Type::Bytes { length: 32 },
        }],
    };
    r.parameters[1].ty = custom.clone();
    fn visit(v: &mut Json, name: &str) {
        if v["kind"] == "parameter" && v["name"] == name {
            *v = json!({"kind":"call","name":"beneficiary_key","arguments":[v.clone()]});
            return;
        }
        match v {
            Json::Object(map) => map.values_mut().for_each(|v| visit(v, name)),
            Json::Array(items) => items.iter_mut().for_each(|v| visit(v, name)),
            _ => {}
        }
    }
    let mut actions = serde_json::to_value(&r.actions).unwrap();
    visit(&mut actions, &argument);
    r.actions = serde_json::from_value(actions).unwrap();
    c.circuits.push(PureCircuit {
        source: None,
        name: "beneficiary_key".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "beneficiary".into(),
            ty: custom,
        }],
        result: canonical.clone(),
        body: Expr::StructLiteral {
            ty: canonical,
            fields: vec![Expr::StructField {
                value: Box::new(parameter("beneficiary")),
                field: "payload".into(),
                index: 0,
            }],
        },
    });
    included(&c, "set_topic", "arbitrary-beneficiary-wrapper");
}
#[test]
fn merge_helper_wrappers_preserve_required_received_bridge_and_root_flow() {
    for mode in 0..2 {
        let mut c = source(true);
        let result = circuit_mut(&mut c, "mergeCoin").result.clone();
        let body = returned_mut(&mut c, "mergeCoin");
        let original = body.clone();
        *body = if mode == 0 {
            Expr::Sequence {
                steps: vec![Expr::Unit],
                value: Box::new(original),
            }
        } else {
            Expr::Let {
                bindings: vec![LocalBinding {
                    name: "merged_alias".into(),
                    ty: result,
                    value: original,
                }],
                body: Box::new(parameter("merged_alias")),
            }
        };
        included(&c, "set_topic", &format!("merge-wrapper-{mode}"));
    }
}
