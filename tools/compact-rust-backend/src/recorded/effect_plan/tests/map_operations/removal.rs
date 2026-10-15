// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Key-only removal still validates slot identity, SSA and operation result.
use super::*;

fn nested_removal() -> Contract {
    let mut c = contract(
        Type::Field,
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    );
    let root = &mut c.stateful_circuits[0];
    root.parameters.remove(1);
    root.actions = vec![StateAction::MapRemove {
        field: "table".into(),
        index: 0,
        key: parameter("key"),
    }];
    root.result = Type::Unit;
    root.return_value = StateReturn::Unit;
    c
}

#[test]
fn nested_removal_validates_identity_key_scope_and_unit_result_independently() {
    let c = nested_removal();
    let original = checked(&c).unwrap().plan().clone();
    assert!(matches!(
        original.body.bindings[0].operation,
        Operation::MapRemove { .. }
    ));
    validate::check(original.clone(), declarations(&c)).unwrap();
    for mode in [
        "declaration",
        "index",
        "kind",
        "missing_key",
        "self_key",
        "annotation",
        "wrong_key",
        "result",
    ] {
        let mut plan = original.clone();
        let mut source = c.clone();
        let self_id = plan.body.bindings[0].value.id;
        let wrong_key = plan.parameters[1].clone();
        let binding = &mut plan.body.bindings[0];
        match mode {
            "declaration" => map_slot(&mut binding.operation).declaration = LedgerId(999),
            "index" => map_slot(&mut binding.operation).index = 99,
            "kind" => source.ledger_fields[0].declaration = LedgerFieldKind::Counter,
            "missing_key" => map_key(&mut binding.operation).id = 999,
            "self_key" => map_key(&mut binding.operation).id = self_id,
            "annotation" => map_key(&mut binding.operation).ty = Type::Boolean,
            "wrong_key" => *map_key(&mut binding.operation) = wrong_key,
            "result" => {
                binding.value.ty = Type::Boolean;
                plan.body.result = binding.value.clone();
                plan.result_type = Type::Boolean;
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, declarations(&source)).is_err(),
            "{mode}"
        );
    }
}

#[test]
fn a_checked_key_only_slot_does_not_authorize_forged_scalar_access() {
    let c = nested_removal();
    let original = checked(&c).unwrap().plan().clone();
    for insert in [false, true] {
        let mut plan = original.clone();
        let binding = &mut plan.body.bindings[0];
        let Operation::MapRemove { slot, key } = binding.operation.clone() else {
            unreachable!()
        };
        binding.operation = if insert {
            // Use valid, in-scope values: refusal must not depend on a bad SSA ID.
            Operation::MapInsert {
                slot,
                value: key.clone(),
                key,
            }
        } else {
            Operation::MapLookup { slot, key }
        };
        let Err(gap) = validate::check(plan, declarations(&c)) else {
            panic!("forged scalar operation insert={insert}");
        };
        assert_eq!(gap.detail, "type is outside checked recording value domain");
    }
}
