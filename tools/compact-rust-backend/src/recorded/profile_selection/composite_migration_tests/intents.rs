// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::ir::KernelClaimKind;
fn input() -> Expr {
    Expr::CreateZswapInput {
        coin: Box::new(parameter("input")),
    }
}
fn output() -> Expr {
    Expr::CreateZswapOutput {
        coin: Box::new(parameter("coin")),
        recipient: Box::new(parameter("recipient")),
    }
}
fn intent_subject(result: Type, value: Expr) -> Contract {
    let mut contract = subject(result, value);
    contract.stateful_circuits[0].parameters = vec![
        Parameter {
            name: "input".into(),
            ty: crate::coin_shapes::qualified_coin_type(),
        },
        Parameter {
            name: "coin".into(),
            ty: crate::coin_shapes::shielded_coin_type(),
        },
        Parameter {
            name: "recipient".into(),
            ty: crate::coin_shapes::shielded_recipient_type(),
        },
        Parameter {
            name: "claim".into(),
            ty: Type::Bytes { length: 32 },
        },
        Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        },
    ];
    contract
}
#[test]
fn both_intents_all_claims_and_either_evidence_source_are_old_first() {
    for intent in [input(), output()] {
        for claim in [
            KernelClaimKind::Nullifier,
            KernelClaimKind::CoinSpend,
            KernelClaimKind::CoinReceive,
        ] {
            let result = structure("Result", vec![Type::Unit, Type::Unit]);
            let query = Expr::KernelClaim {
                claim,
                value: Box::new(parameter("claim")),
            };
            let value = record(result.clone(), vec![intent.clone(), query]);
            included(&intent_subject(result, value), "root", "intent+claim");
        }
        let result = structure(
            "Result",
            vec![Type::Unit, crate::ir::contract_address_type()],
        );
        let value = record(
            result.clone(),
            vec![
                intent,
                Expr::KernelSelf {
                    ty: crate::ir::contract_address_type(),
                },
            ],
        );
        included(&intent_subject(result, value), "root", "intent+KernelSelf");
    }
}
#[test]
fn intent_branches_unit_locals_and_repeated_noncopy_operands_are_old_first() {
    let result = structure(
        "Result",
        vec![Type::Unit, Type::Unit, crate::ir::contract_address_type()],
    );
    let arm = |value| {
        record(
            result.clone(),
            vec![
                value,
                output(),
                Expr::KernelSelf {
                    ty: crate::ir::contract_address_type(),
                },
            ],
        )
    };
    let expression = local(
        "consumed",
        Type::Unit,
        input(),
        Expr::If {
            condition: Box::new(parameter("selected")),
            then: Box::new(arm(parameter("consumed"))),
            otherwise: Box::new(arm(output())),
        },
    );
    included(
        &intent_subject(result, expression),
        "root",
        "Unit locals/branches/repeated coin and recipient",
    );
}
#[test]
fn retained_transfer_has_exact_export_and_old_intent_admission() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/composite-zswap-transfer-schema20-ir.json"
    ))
    .unwrap();
    assert_eq!(
        contract
            .stateful_circuits
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["transfer"]
    );
    included(&contract, "transfer", "retained transfer");
}
#[test]
fn absent_observation_is_expansion_and_wrong_coin_shape_remains_invalid() {
    let result = structure("Result", vec![Type::Unit]);
    let mut contract = intent_subject(result.clone(), record(result, vec![input()]));
    let context = Context::new(&contract);
    let root = &contract.stateful_circuits[0];
    assert!(
        context.old(root).is_none(),
        "old static observation evidence"
    );
    assert!(
        context.domain(root).unwrap().is_some(),
        "recordability does not promise preparation"
    );
    contract.stateful_circuits[0].parameters[0].ty = crate::coin_shapes::shielded_coin_type();
    let context = Context::new(&contract);
    assert!(context.old(&contract.stateful_circuits[0]).is_none());
    assert!(matches!(
        context.algebra(&contract.stateful_circuits[0]),
        Err(effect_plan::PlanError::Unsupported(_))
    ));
}
