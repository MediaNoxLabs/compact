// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn rename(c: &Contract, names: &[&str]) -> Contract {
    fn visit(value: &mut Json, names: &[&str]) {
        match value {
            Json::Object(fields) => {
                if let Some(Json::String(name)) = fields.get_mut("name")
                    && names.contains(&name.as_str())
                {
                    *name = format!("immediate_probe_{name}");
                }
                fields.values_mut().for_each(|v| visit(v, names));
            }
            Json::Array(values) => values.iter_mut().for_each(|v| visit(v, names)),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(c).unwrap();
    visit(&mut value, names);
    serde_json::from_value(value).unwrap()
}

#[test]
fn whole_old_immediate_send_names_nominal_products_and_internal_root_compose() {
    let c = rename(
        &source(),
        &[
            "receive_then_send",
            "receiveShielded",
            "sendImmediateShielded",
            "sendShielded",
            "upcastQualifiedCoin",
            "coin",
            "recipient",
            "input",
            "target",
        ],
    );
    included(
        &c,
        "immediate_probe_receive_then_send",
        "renamed declarations and formals",
    );
    let c = rename(&source(), &["ShieldedSendResult", "Maybe"]);
    included(
        &c,
        "receive_then_send",
        "consistently renamed result and optional products",
    );
    let mut c = source();
    circuit_mut(&mut c, "receive_then_send").internal = true;
    included(&c, "receive_then_send", "internal root");
}

#[test]
fn whole_old_immediate_send_root_argument_identities_compose() {
    let mut c = source();
    let r = circuit_mut(&mut c, "receive_then_send");
    let StateAction::CircuitCall { arguments, .. } = &mut r.actions[0] else {
        unreachable!()
    };
    arguments[0] = Expr::Coerce {
        value: Box::new(arguments[0].clone()),
        ty: r.parameters[0].ty.clone(),
    };
    let StateReturn::Expression {
        value: Expr::Call { arguments, .. },
    } = &mut r.return_value
    else {
        unreachable!()
    };
    for (value, ty) in arguments.iter_mut().zip([
        r.parameters[0].ty.clone(),
        r.parameters[1].ty.clone(),
        Type::Unsigned {
            max: u128::MAX.to_string(),
        },
    ]) {
        *value = Expr::Coerce {
            value: Box::new(value.clone()),
            ty,
        };
    }
    included(&c, "receive_then_send", "extra source identity coercions");
}

#[test]
fn whole_old_immediate_send_receive_wrappers_and_indirection_compose() {
    for empty_let in [false, true] {
        let mut c = source();
        let receive = circuit_mut(&mut c, "receiveShielded");
        let body = StateAction::Sequence {
            actions: std::mem::take(&mut receive.actions),
        };
        receive.actions = vec![if empty_let {
            StateAction::Let {
                bindings: vec![],
                action: Box::new(body),
            }
        } else {
            body
        }];
        included(
            &c,
            "receive_then_send",
            &format!("receive empty-let={empty_let}"),
        );
    }
    let mut c = source();
    let receive = circuit_mut(&mut c, "receiveShielded");
    let mut implementation = receive.clone();
    implementation.name = "receive_implementation".into();
    receive.actions = vec![StateAction::CircuitCall {
        name: implementation.name.clone(),
        arguments: vec![parameter(&receive.parameters[0].name)],
    }];
    c.stateful_circuits.push(implementation);
    // This whole oracle permits indirection; voting's additional forwarding guard does not.
    included(&c, "receive_then_send", "receive helper indirection");
}

#[test]
fn whole_old_immediate_send_ordinary_pure_callees_compose() {
    for name in ["coinCommitment", "coinNullifier"] {
        for local in [false, true] {
            let mut c = source();
            let helper = c.circuits.iter_mut().find(|c| c.name == name).unwrap();
            let body = helper.body.clone();
            helper.body = if local {
                Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "pure_output".into(),
                        ty: helper.result.clone(),
                        value: body,
                    }],
                    body: Box::new(parameter("pure_output")),
                }
            } else {
                Expr::Coerce {
                    value: Box::new(body),
                    ty: helper.result.clone(),
                }
            };
            included(
                &c,
                "receive_then_send",
                &format!("pure {name}/local={local}"),
            );
        }
    }
}
