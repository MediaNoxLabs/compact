// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn renamed(c: &Contract, names: &[&str]) -> Contract {
    fn visit(value: &mut Json, names: &[&str]) {
        match value {
            Json::Object(fields) => {
                if let Some(Json::String(name)) = fields.get_mut("name")
                    && names.contains(&name.as_str())
                {
                    *name = format!("merge_probe_{name}");
                }
                fields.values_mut().for_each(|v| visit(v, names));
            }
            Json::Array(values) => values.iter_mut().for_each(|v| visit(v, names)),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(c).unwrap();
    visit(&mut v, names);
    serde_json::from_value(v).unwrap()
}
#[test]
fn whole_merge_root_helpers_and_formals_are_not_recognized_by_name() {
    let c = renamed(
        &source(),
        &[
            "merge_qualified",
            "receive_then_merge",
            "mergeCoin",
            "mergeCoinImmediate",
            "receiveShielded",
            "upcastQualifiedCoin",
            "a",
            "b",
            "coin",
        ],
    );
    for name in [
        "merge_probe_merge_qualified",
        "merge_probe_receive_then_merge",
    ] {
        included(&c, name, &format!("renamed-{name}"));
    }
}
#[test]
fn whole_merge_internal_roots_have_real_recorded_helper_owners() {
    for name in ["merge_qualified", "receive_then_merge"] {
        let mut c = source();
        circuit_mut(&mut c, name).internal = true;
        // Keep a public forwarder so raw generated crate compilation also builds
        // the internal recorded call, not merely its independently rendered item.
        let mut forwarder = circuit_mut(&mut c, name).clone();
        forwarder.internal = false;
        forwarder.name = format!("forward_{name}");
        forwarder.actions.clear();
        forwarder.return_value = StateReturn::Expression {
            value: Expr::Call {
                name: name.into(),
                arguments: forwarder
                    .parameters
                    .iter()
                    .map(|p| parameter(&p.name))
                    .collect(),
            },
        };
        c.stateful_circuits.push(forwarder);
        included(&c, name, &format!("internal-{name}"));
    }
}
#[test]
fn whole_merge_syntactic_evidence_preserves_an_inactive_default_branch() {
    for selected in [false, true] {
        let mut c = source();
        let ty = circuit_mut(&mut c, "mergeCoin").result.clone();
        let body = returned_mut(&mut c, "mergeCoin");
        *body = Expr::If {
            condition: Box::new(Expr::Boolean { value: selected }),
            then: Box::new(body.clone()),
            otherwise: Box::new(Expr::Default { ty }),
        };
        both(&c, &format!("branch-default-{selected}"));
    }
}
#[test]
fn whole_merge_let_sequence_and_identity_coercion_preserve_effect_counts() {
    for mode in 0..3 {
        let mut c = source();
        let ty = circuit_mut(&mut c, "mergeCoin").result.clone();
        let body = returned_mut(&mut c, "mergeCoin");
        let original = body.clone();
        *body = match mode {
            0 => Expr::Let {
                bindings: vec![LocalBinding {
                    name: "merged".into(),
                    ty,
                    value: original,
                }],
                body: Box::new(parameter("merged")),
            },
            1 => Expr::Sequence {
                steps: vec![Expr::Unit, Expr::Unit],
                value: Box::new(original),
            },
            _ => Expr::Coerce {
                value: Box::new(original),
                ty,
            },
        };
        both(&c, &format!("wrapper-{mode}"));
    }
}
