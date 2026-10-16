// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Coin Set operands compose; the runtime owns recipient-bound qualification.
use super::*;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
#[path = "qualified_coin_set/inclusion.rs"]
mod inclusion;
#[path = "qualified_coin_set/validation.rs"]
mod validation;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/qualified-coin-set-schema20-ir.json"
    ))
    .unwrap()
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn insert(coin: Expr, recipient: Expr) -> StateAction {
    StateAction::SetInsertCoin {
        field: "coins".into(),
        index: 0,
        coin,
        recipient,
    }
}
fn witness(name: &str) -> Expr {
    Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|r| (r.name.as_str(), r)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect(),
    )
}
fn declarations(c: &Contract) -> declarations::Declarations {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|r| (r.name.as_str(), r)).collect();
    let calls = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    let types = type_environment::TypeEnvironment::from_sources(
        &c.stateful_circuits[0],
        &ledger,
        &witnesses,
        &pure,
        &calls,
    )
    .unwrap();
    declarations::Declarations::with_calls(&ledger, &witnesses, &pure, &calls)
        .with_type_environment(types)
        .with_root(&c.stateful_circuits[0])
}
fn insertion(body: &Block) -> usize {
    body.bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::SetInsertCoin { .. }))
        .unwrap()
}
fn operand(operation: &mut Operation, n: usize) -> &mut Value {
    let Operation::SetInsertCoin {
        coin, recipient, ..
    } = operation
    else {
        unreachable!()
    };
    if n == 0 { coin } else { recipient }
}

#[test]
fn qualified_set_insertion_uses_official_slot_without_fabricating_intent_or_index() {
    let c = source();
    let plan = checked(&c).unwrap();
    let binding = &plan.plan().body.bindings[insertion(&plan.plan().body)];
    let Operation::SetInsertCoin {
        slot,
        coin,
        recipient,
    } = &binding.operation
    else {
        unreachable!()
    };
    assert_eq!(
        slot.declaration,
        declarations(&c).resolve_field("coins").unwrap().0
    );
    assert_eq!(slot.index, 0);
    assert_eq!(coin.ty, shielded_coin_type());
    assert_eq!(recipient.ty, shielded_recipient_type());
    assert_eq!(binding.value.ty, Type::Unit);
    let steps = render(&plan).unwrap().steps;
    let rust = quote::quote!(#(#steps)*).to_string();
    for token in [
        "ledger_slots :: coins",
        "record_insert_coin",
        "coin_info_from_compact",
        "coin_recipient_from_compact",
    ] {
        assert!(rust.contains(token), "{token}: {rust}");
    }
    assert_eq!(rust.matches("record_insert_coin").count(), 1);
    assert!(!rust.contains("create_zswap") && !rust.contains("mt_index"));
}

#[test]
fn direct_witness_operands_are_once_only_and_coin_precedes_recipient() {
    let mut c = source();
    c.witnesses.extend([
        WitnessDeclaration {
            name: "nextCoin".into(),
            source: None,
            parameters: vec![],
            result: shielded_coin_type(),
        },
        WitnessDeclaration {
            name: "nextRecipient".into(),
            source: None,
            parameters: vec![],
            result: shielded_recipient_type(),
        },
    ]);
    c.stateful_circuits[0].actions = vec![insert(witness("nextCoin"), witness("nextRecipient"))];
    let plan = checked(&c).unwrap();
    let bindings = &plan.plan().body.bindings;
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(bindings[1].operation, Operation::Witness { .. }));
    assert_eq!(bindings[0].path, "actions[0].coin");
    assert_eq!(bindings[1].path, "actions[0].recipient");
    assert_eq!(insertion(&plan.plan().body), 2);
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![
            LocalBinding {
                name: "coin".into(),
                ty: shielded_coin_type(),
                value: witness("nextCoin"),
            },
            LocalBinding {
                name: "recipient".into(),
                ty: shielded_recipient_type(),
                value: witness("nextRecipient"),
            },
        ],
        action: Box::new(StateAction::Sequence {
            actions: vec![insert(parameter("coin"), parameter("recipient")); 2],
        }),
    }];
    let plan = checked(&c).unwrap();
    let bindings = &plan.plan().body.bindings;
    assert_eq!(
        bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::Witness { .. }))
            .count(),
        2
    );
    let mut count = 0;
    for b in bindings {
        if let Operation::SetInsertCoin {
            coin, recipient, ..
        } = &b.operation
        {
            count += 1;
            assert_eq!(coin, &bindings[0].value);
            assert_eq!(recipient, &bindings[1].value);
            assert_ne!(coin, &plan.plan().parameters[0]);
        }
    }
    assert_eq!(count, 2);
    render(&plan).unwrap();
}

#[test]
fn source_canonical_layouts_and_bounds_are_exact_for_slot_and_both_operands() {
    for target in 0..3 {
        for mode in 0..4 {
            let mut c = source();
            let ty = if target == 0 {
                let LedgerFieldKind::Set { ty } = &mut c.ledger_fields[0].declaration else {
                    unreachable!()
                };
                ty
            } else {
                &mut c.stateful_circuits[0].parameters[target - 1].ty
            };
            let Type::Struct { name, fields } = ty else {
                unreachable!()
            };
            match mode {
                0 => name.push_str("Lookalike"),
                1 => fields.swap(0, 1),
                2 => fields[0].ty = Type::Bytes { length: 31 },
                _ => {
                    if target == 2 {
                        let Type::Struct { fields, .. } = &mut fields[1].ty else {
                            unreachable!()
                        };
                        fields[0].ty = Type::Bytes { length: 33 };
                    } else {
                        fields[2].ty = Type::Unsigned {
                            max: u64::MAX.to_string(),
                        };
                    }
                }
            }
            assert!(
                matches!(checked(&c), Err(PlanError::Unsupported(_))),
                "target={target}, mode={mode}"
            );
        }
    }
}
