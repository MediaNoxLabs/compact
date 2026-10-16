// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite structural inclusion controls, always admitted by the complete old root
//! before checked lowering and complete native/recorded rendering are asserted.
use super::*;

fn complete_source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/micro-dao-source-schema20-ir.json"
    ))
    .unwrap()
}

fn included(contract: &Contract, root: &str, label: &str) {
    let env = Environment::new(contract);
    assert!(
        crate::recorded::typed_plan::lower_voting_commit(
            env.stateful[root],
            &env.ledger,
            &env.witnesses,
            &env.pure,
            &env.stateful,
        )
        .is_some(),
        "{label}: complete old root must accept"
    );
    let checked = env
        .checked(root)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    render(&checked).unwrap_or_else(|e| panic!("{label}: {e:?}"));
    crate::render_with_capabilities(contract)
        .unwrap_or_else(|e| panic!("{label}: complete native/recorded render: {e:?}"));
}

#[test]
fn whole_old_voting_root_and_receive_sequence_let_wrappers_compose() {
    for name in ["vote_commit", "receiveShielded"] {
        for empty_let in [false, true] {
            let mut contract = complete_source();
            let circuit = contract
                .stateful_circuits
                .iter_mut()
                .find(|c| c.name == name)
                .unwrap();
            let body = StateAction::Sequence {
                actions: std::mem::take(&mut circuit.actions),
            };
            circuit.actions = vec![if empty_let {
                StateAction::Let {
                    bindings: vec![],
                    action: Box::new(body),
                }
            } else {
                StateAction::Sequence {
                    actions: vec![body],
                }
            }];
            included(
                &contract,
                "vote_commit",
                &format!("{name}/empty-let={empty_let}"),
            );
        }
    }
}

#[test]
fn receive_indirection_composes_beyond_the_old_direct_forwarding_guard() {
    let mut contract = complete_source();
    let receive = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "receiveShielded")
        .unwrap();
    let mut implementation = receive.clone();
    implementation.name = "receive_implementation".into();
    implementation.internal = true;
    receive.actions = vec![StateAction::CircuitCall {
        name: implementation.name.clone(),
        arguments: vec![parameter(&receive.parameters[0].name)],
    }];
    contract.stateful_circuits.push(implementation);
    // The old direct-forwarding guard refuses an intermediate CircuitCall,
    // even when it forwards the exact same coin to the unchanged implementation.
    // This case establishes expansion, not inclusion of old admission.
    let env = Environment::new(&contract);
    assert!(
        crate::recorded::typed_plan::lower_voting_commit(
            env.stateful["vote_commit"],
            &env.ledger,
            &env.witnesses,
            &env.pure,
            &env.stateful,
        )
        .is_none()
    );
    let checked = env
        .checked("vote_commit")
        .expect("same-coin indirection composes");
    render(&checked).unwrap();
    crate::render_with_capabilities(&contract).unwrap();
}

#[test]
fn whole_old_voting_context_identity_and_let_values_compose() {
    for local in [false, true] {
        let mut contract = complete_source();
        let helper = contract
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "dao_voting_token")
            .unwrap();
        let StateReturn::Expression { value } = &mut helper.return_value else {
            unreachable!()
        };
        let original = value.clone();
        *value = if local {
            Expr::Let {
                bindings: vec![LocalBinding {
                    name: "context_result".into(),
                    ty: helper.result.clone(),
                    value: original,
                }],
                body: Box::new(parameter("context_result")),
            }
        } else {
            Expr::Coerce {
                value: Box::new(original),
                ty: helper.result.clone(),
            }
        };
        included(&contract, "vote_commit", &format!("context let={local}"));
    }
}

#[test]
fn whole_old_voting_renamed_root_helpers_and_formals_compose() {
    // Rename only declaration/reference names; canonical type and field identities
    // remain untouched. No string rewriting of emitted Rust is involved.
    fn rename(value: &mut Json) {
        match value {
            Json::Object(fields) => {
                if let Some(Json::String(name)) = fields.get_mut("name")
                    && [
                        "vote_commit",
                        "receiveShielded",
                        "sendImmediateShielded",
                        "dao_voting_token",
                        "commit_with_sk",
                        "ballot",
                    ]
                    .contains(&name.as_str())
                {
                    *name = format!("renamed_{name}");
                }
                for value in fields.values_mut() {
                    rename(value);
                }
            }
            Json::Array(values) => values.iter_mut().for_each(rename),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(complete_source()).unwrap();
    rename(&mut value);
    included(
        &serde_json::from_value(value).unwrap(),
        "renamed_vote_commit",
        "renamed declarations and formals",
    );
}
