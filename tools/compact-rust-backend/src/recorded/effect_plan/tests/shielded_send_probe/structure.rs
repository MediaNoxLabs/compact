// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old control-flow and helper rules, independent of fixture names.
use super::*;

#[test]
fn whole_old_send_admission_is_independent_of_wrapper_and_declaration_names() {
    fn rename(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(serde_json::Value::String(name)) = object.get_mut("name") {
                    *name = match name.as_str() {
                        "ShieldedSendResult" => "TransferOutcome",
                        "Maybe" => "PossibleChange",
                        "send_to_self" => "renamed_entry",
                        "sendShielded" => "transfer_helper",
                        "input" => "source_coin",
                        other => other,
                    }
                    .to_owned();
                }
                for child in object.values_mut() {
                    rename(child);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    rename(child);
                }
            }
            _ => {}
        }
    }
    let mut document = serde_json::to_value(source()).unwrap();
    rename(&mut document);
    let contract = serde_json::from_value(document).unwrap();
    probe(
        &contract,
        "renamed_entry",
        "renamed-layout-and-declarations",
        true,
    );
}

#[test]
fn whole_old_send_preserves_lexical_shadowing_and_unit_sequence() {
    let mut contract = source();
    prepend(
        &mut contract,
        vec![
            local(
                "saved_value",
                validate::uint(&u128::MAX.to_string()),
                reference("value"),
            ),
            local(
                "value",
                validate::uint(&u128::MAX.to_string()),
                reference("saved_value"),
            ),
            local("temporary", Type::Field, field_literal(3)),
            local("temporary", Type::Field, field_literal(7)),
        ],
    );
    let root = root_mut(&mut contract, "send_to_self");
    let original = expression(root).clone();
    *expression(root) = Expr::Sequence {
        steps: vec![Expr::Unit, Expr::Unit],
        value: Box::new(original),
    };
    probe(&contract, "send_to_self", "shadowing-and-sequence", true);
}

#[test]
fn whole_old_send_branch_evidence_does_not_require_effects_on_every_path() {
    for selected in [false, true] {
        let mut contract = source();
        let root = root_mut(&mut contract, "send_to_self");
        let original = expression(root).clone();
        *expression(root) = Expr::If {
            condition: Box::new(Expr::Boolean { value: selected }),
            then: Box::new(original),
            otherwise: Box::new(Expr::Default {
                ty: root.result.clone(),
            }),
        };
        probe(
            &contract,
            "send_to_self",
            &format!("branch-send-or-default-{selected}"),
            true,
        );
    }
}

#[test]
fn whole_old_send_helpers_keep_fresh_owned_vector_scopes_and_metadata() {
    let ty = vector(Type::Field, 2);
    for internal in [false, true] {
        let mut contract = source();
        contract.stateful_circuits.push(StatefulCircuit {
            source: None,
            name: "local_vector".into(),
            internal,
            parameters: vec![
                parameter("value", ty.clone()),
                parameter("input", Type::Field),
            ],
            actions: vec![],
            result: ty.clone(),
            return_value: StateReturn::Expression {
                value: Expr::Let {
                    bindings: vec![local("input", Type::Field, reference("input"))],
                    body: Box::new(reference("value")),
                },
            },
        });
        let coerced = Expr::Coerce {
            value: Box::new(Expr::Tuple {
                elements: vec![field_literal(3), field_literal(7)],
            }),
            ty: ty.clone(),
        };
        prepend(
            &mut contract,
            vec![
                local(
                    "first_vector",
                    ty.clone(),
                    call("local_vector", vec![coerced, field_literal(11)]),
                ),
                local(
                    "second_vector",
                    ty.clone(),
                    call(
                        "local_vector",
                        vec![reference("first_vector"), field_literal(13)],
                    ),
                ),
            ],
        );
        probe(
            &contract,
            "send_to_self",
            &format!("stateful-vector-helper-{internal}"),
            true,
        );
    }
}

#[test]
fn whole_old_send_admits_all_claim_variants_in_ordered_unit_steps() {
    let mut contract = source();
    let root = root_mut(&mut contract, "send_to_self");
    let original = expression(root).clone();
    *expression(root) = Expr::Sequence {
        steps: [
            KernelClaimKind::Nullifier,
            KernelClaimKind::CoinSpend,
            KernelClaimKind::CoinReceive,
        ]
        .into_iter()
        .map(|claim| Expr::KernelClaim {
            claim,
            value: Box::new(Expr::BytesLiteral { bytes: vec![1; 32] }),
        })
        .collect(),
        value: Box::new(original),
    };
    probe(&contract, "send_to_self", "all-three-claim-variants", true);
}

#[test]
fn genuine_optional_send_has_complete_old_admission_and_checked_support() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-optional-send-schema20-ir.json"
    ))
    .unwrap();
    probe(&contract, "optional_send", "genuine-optional-send", true);
}
