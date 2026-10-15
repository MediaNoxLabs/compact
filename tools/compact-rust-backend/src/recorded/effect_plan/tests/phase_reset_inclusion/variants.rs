// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn whole_old_phase_reset_declaration_names_are_data() {
    fn rename(v: &mut Json) {
        match v {
            Json::Object(fields) => {
                for key in ["name", "field", "id"] {
                    if let Some(Json::String(name)) = fields.get_mut(key)
                        && [
                            "advance",
                            "reset_state",
                            "reset_pot",
                            "local_secret_key",
                            "public_key",
                            "successor",
                            "committed_votes",
                            "yes",
                            "no",
                            "round",
                        ]
                        .contains(&name.as_str())
                    {
                        *name = format!("phase_probe_{name}");
                    }
                }
                fields.values_mut().for_each(rename);
            }
            Json::Array(values) => values.iter_mut().for_each(rename),
            _ => {}
        }
    }
    let mut json = serde_json::to_value(source()).unwrap();
    rename(&mut json);
    let c: Contract = serde_json::from_value(json).unwrap();
    included(
        &c,
        "phase_probe_advance",
        "renamed source declarations and references",
    );
}

#[test]
fn whole_old_phase_reset_root_and_helper_wrappers_compose() {
    for name in ["advance", "reset_state"] {
        for empty_let in [false, true] {
            let mut c = source();
            let r = circuit_mut(&mut c, name);
            if name == "advance" && !empty_let {
                // Preserve the old mandatory outer Let while wrapping its body.
                let StateAction::Let { action, .. } = &mut r.actions[0] else {
                    unreachable!()
                };
                let original =
                    std::mem::replace(action.as_mut(), StateAction::Sequence { actions: vec![] });
                **action = StateAction::Sequence {
                    actions: vec![original],
                };
            } else {
                let body = StateAction::Sequence {
                    actions: std::mem::take(&mut r.actions),
                };
                r.actions = vec![if empty_let {
                    StateAction::Let {
                        bindings: vec![],
                        action: Box::new(body),
                    }
                } else {
                    body
                }];
            }
            included(&c, "advance", &format!("{name}/empty-let={empty_let}"));
        }
    }
}

#[test]
fn whole_old_phase_reset_pure_callees_keep_native_lexical_semantics() {
    for name in ["successor", "public_key"] {
        for local in [false, true] {
            let mut c = source();
            let helper = c.circuits.iter_mut().find(|c| c.name == name).unwrap();
            let original = helper.body.clone();
            helper.body = if local {
                Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "phase_pure_result".into(),
                        ty: helper.result.clone(),
                        value: original,
                    }],
                    body: Box::new(Expr::Parameter {
                        name: "phase_pure_result".into(),
                    }),
                }
            } else {
                Expr::Coerce {
                    value: Box::new(original),
                    ty: helper.result.clone(),
                }
            };
            included(&c, "advance", &format!("pure {name}/local={local}"));
        }
    }
}

#[test]
fn whole_old_phase_reset_false_identity_and_increment_values_compose() {
    let mut c = source();
    edit_circuit(&mut c, "advance", "circuit_call", |v| {
        let original = v["arguments"][0].take();
        v["arguments"][0] = json!({"kind":"coerce","ty":{"kind":"boolean"},"value":original});
    });
    included(&c, "advance", "nested literal-false identity coercion");
    for amount in [0, 2, u16::MAX] {
        let mut c = source();
        edit_circuit(&mut c, "reset_state", "unsigned_literal", |v| {
            assert_eq!(v["max"], "65535");
            v["value"] = json!(amount.to_string());
        });
        included(&c, "advance", &format!("Uint16 increment {amount}"));
    }
}
