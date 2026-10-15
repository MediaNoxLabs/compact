// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn storage_codecs_independent_validation_rechecks_each_operand_and_result() {
    let c = contract(product(vector(Type::Field, 2)));
    let original = checked(&c).unwrap().plan().clone();
    for position in 0..original.body.bindings.len() {
        if input(&mut original.body.bindings[position].operation.clone()).is_none() {
            continue;
        }
        for mutation in ["missing", "self", "annotation"] {
            let mut plan = original.clone();
            let binding = &mut plan.body.bindings[position];
            let self_id = binding.value.id;
            let operand = input(&mut binding.operation).unwrap();
            match mutation {
                "missing" => operand.id = usize::MAX,
                "self" => operand.id = self_id,
                _ => operand.ty = product(vector(Type::Boolean, 2)),
            }
            assert!(
                validate::check(plan, declarations(&c)).is_err(),
                "{position} {mutation}"
            );
        }
    }
    let mut forged = original.clone();
    let read = forged
        .body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::CellRead { .. }))
        .unwrap();
    read.value.ty = product(vector(Type::Boolean, 2));
    assert!(validate::check(forged, declarations(&c)).is_err());
    let mut forged = original;
    forged.parameters[0].ty = product(vector(Type::Boolean, 2));
    assert!(
        validate::check(forged, declarations(&c)).is_err(),
        "canonical named definition cannot be forged"
    );
}

#[test]
fn storage_codecs_keep_declaration_kind_index_and_lexical_authority() {
    let c = contract(vector(Type::Field, 2));
    let original = checked(&c).unwrap().plan().clone();
    for family in ["cell", "set", "map"] {
        let mut plan = original.clone();
        let operation = plan
            .body
            .bindings
            .iter_mut()
            .find(|b| match family {
                "cell" => matches!(b.operation, Operation::CellWrite { .. }),
                "set" => matches!(b.operation, Operation::SetInsert { .. }),
                _ => matches!(b.operation, Operation::MapInsert { .. }),
            })
            .unwrap();
        match &mut operation.operation {
            Operation::CellWrite { slot, .. } => slot.index = 9,
            Operation::SetInsert { slot, .. } => slot.index = 9,
            Operation::MapInsert { slot, .. } => slot.index = 9,
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, declarations(&c)).is_err(),
            "{family} index"
        );
    }
    let mut wrong_kind = c.clone();
    wrong_kind.ledger_fields[0].declaration = LedgerFieldKind::Set {
        ty: vector(Type::Field, 2),
    };
    assert!(checked(&wrong_kind).is_err());
    let mut wrong_source = c;
    let StateAction::CellWrite { value, .. } = &mut wrong_source.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *value = Expr::Parameter {
        name: "undeclared".into(),
    };
    assert!(checked(&wrong_source).is_err());
    assert!(crate::render_with_capabilities(&wrong_source).is_err());
}

#[test]
fn storage_codecs_preserve_canonical_and_fixed_layout_resource_refusals() {
    for ty in [
        validate::uint("01"),
        validate::uint("not unsigned"),
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662656",
        ),
        vector(
            Type::Field,
            crate::resource_limits::recording_fixed_layout_limit() + 1,
        ),
        Type::Bytes {
            length: crate::resource_limits::recording_byte_payload_limit() + 1,
        },
    ] {
        assert!(checked(&contract(ty)).is_err());
    }
    let c = contract(vector(
        Type::Field,
        crate::resource_limits::recording_fixed_layout_limit() + 1,
    ));
    let Err(PlanError::Unsupported(gap)) = checked(&c) else {
        panic!("fixed layout must be refused before plan creation")
    };
    assert!(gap.resource.is_some(), "retain typed resource diagnostics");
    let mut malformed = contract(product(Type::Field));
    let Type::Struct { fields, .. } = &mut malformed.stateful_circuits[0].parameters[0].ty else {
        unreachable!()
    };
    fields[0].ty = Type::Boolean;
    assert!(
        checked(&malformed).is_err(),
        "root and ledger named definitions must agree"
    );
}
