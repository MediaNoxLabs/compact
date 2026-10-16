// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! All forged candidates are checked against the original source declarations.
use super::*;

fn invalid(c: &Contract, plan: Plan, label: &str) -> RecordingGap {
    let error = validate::check(plan, declarations(c))
        .map_err(PlanError::Invalid)
        .unwrap_err();
    let PlanError::Invalid(gap) = error else {
        panic!("{label}: forged plans are invariants, not source refusals")
    };
    gap
}
fn with_decoys() -> Contract {
    let mut c = source();
    for (name, declaration) in [
        (
            "ordinary",
            LedgerFieldKind::Cell {
                ty: shielded_coin_type(),
            },
        ),
        (
            "set",
            LedgerFieldKind::Set {
                ty: qualified_coin_type(),
            },
        ),
        ("counter", LedgerFieldKind::Counter),
        (
            "lookalike",
            LedgerFieldKind::Cell {
                ty: malformed(qualified_coin_type(), 0),
            },
        ),
    ] {
        let index = c.ledger_fields.len() as u8;
        c.ledger_fields.push(LedgerField {
            id: name.into(),
            index,
            path: vec![index],
            source: None,
            declaration,
        });
    }
    c
}

#[test]
fn original_declarations_reject_unknown_wrong_index_and_existing_incompatible_slots() {
    let c = with_decoys();
    let original = checked(&c).unwrap().plan().clone();
    validate::check(original.clone(), declarations(&c)).unwrap();
    for mode in [
        "missing",
        "index",
        "ordinary",
        "set",
        "counter",
        "lookalike",
    ] {
        let mut forged = original.clone();
        let i = write_index(&forged.body);
        let Operation::CellWriteCoin { slot, .. } = &mut forged.body.bindings[i].operation else {
            unreachable!()
        };
        match mode {
            "missing" => slot.declaration = LedgerId(999),
            "index" => slot.index = 9,
            name => {
                let declarations = declarations(&c);
                let (id, field) = declarations.resolve_field(name).unwrap();
                slot.declaration = id;
                slot.index = field.index;
            }
        }
        assert_eq!(invalid(&c, forged, mode).path, "actions[0]");
    }
}

#[test]
fn each_operand_rejects_missing_stale_and_wrong_live_values_without_rebuilding_types() {
    let mut c = source();
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "wrong".into(),
        ty: Type::Field,
    });
    let original = checked(&c).unwrap().plan().clone();
    for n in 0..2 {
        for mode in [
            "missing",
            "stale",
            "wrong_live",
            "other_operand",
            "forged_canonical_layout",
        ] {
            let mut forged = original.clone();
            let i = write_index(&forged.body);
            let replacement = match mode {
                "wrong_live" => Some(forged.parameters[2].clone()),
                "other_operand" => Some(forged.parameters[1 - n].clone()),
                _ => None,
            };
            let value = operand(&mut forged.body.bindings[i].operation, n);
            match mode {
                "missing" => value.id = 999,
                "stale" => value.ty = Type::Field,
                "forged_canonical_layout" => value.ty = malformed(value.ty.clone(), 1),
                _ => *value = replacement.unwrap(),
            }
            assert_eq!(
                invalid(&c, forged, mode).path,
                "actions[0]",
                "operand {n}: {mode}"
            );
        }
    }
}

#[test]
fn independently_changed_definition_and_use_cannot_override_canonical_environment() {
    let mut c = source();
    c.stateful_circuits[0].parameters.clear();
    c.stateful_circuits[0].actions = vec![write(
        Expr::Default {
            ty: shielded_coin_type(),
        },
        Expr::Default {
            ty: shielded_recipient_type(),
        },
    )];
    let original = checked(&c).unwrap().plan().clone();
    for n in 0..2 {
        for mode in 0..5 {
            let mut forged = original.clone();
            let i = write_index(&forged.body);
            let mut operation = forged.body.bindings[i].operation.clone();
            let value = operand(&mut operation, n);
            let ty = malformed(value.ty.clone(), mode);
            forged
                .body
                .bindings
                .iter_mut()
                .find(|b| b.value.id == value.id)
                .unwrap()
                .value
                .ty = ty.clone();
            value.ty = ty;
            forged.body.bindings[i].operation = operation;
            invalid(&c, forged, "coordinated definition and operand forgery");
        }
    }
}

#[test]
fn write_result_is_unit_even_when_binding_body_and_root_are_coordinated() {
    let c = source();
    let original = checked(&c).unwrap().plan().clone();
    for ty in [Type::Field, Type::Boolean, qualified_coin_type()] {
        let mut forged = original.clone();
        let i = write_index(&forged.body);
        forged.body.bindings[i].value.ty = ty.clone();
        forged.body.result = forged.body.bindings[i].value.clone();
        forged.result_type = ty;
        invalid(&c, forged, "coordinated result");
    }
    // Keep the immutable root signature valid to isolate the operation's Unit rule.
    let mut forged = original.clone();
    let i = write_index(&forged.body);
    forged.body.bindings[i].value.ty = Type::Field;
    assert_eq!(
        invalid(&c, forged, "operation result only").path,
        "actions[0]"
    );
}

#[test]
fn write_cannot_precede_operand_definitions_or_reuse_a_definition_id() {
    let mut c = source();
    c.stateful_circuits[0].parameters.clear();
    c.stateful_circuits[0].actions = vec![write(
        Expr::Default {
            ty: shielded_coin_type(),
        },
        Expr::Default {
            ty: shielded_recipient_type(),
        },
    )];
    let original = checked(&c).unwrap().plan().clone();
    let i = write_index(&original.body);
    assert_eq!(i, 2);
    let mut reordered = original.clone();
    reordered.body.bindings.swap(0, i);
    assert_eq!(
        invalid(&c, reordered, "use before definition").path,
        "actions[0]"
    );
    for duplicate in [0, 1, i] {
        let mut forged = original.clone();
        let binding = forged.body.bindings[duplicate].clone();
        forged.body.bindings.insert(duplicate + 1, binding);
        invalid(&c, forged, "duplicate definition");
    }
}

#[test]
fn each_write_operand_rejects_same_typed_sibling_and_escaped_branch_values() {
    let mut c = source();
    c.stateful_circuits[0].parameters.clear();
    let branch = || {
        write(
            Expr::Default {
                ty: shielded_coin_type(),
            },
            Expr::Default {
                ty: shielded_recipient_type(),
            },
        )
    };
    c.stateful_circuits[0].actions = vec![
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(branch()),
            otherwise: Box::new(branch()),
        },
        branch(),
    ];
    let original = checked(&c).unwrap().plan().clone();
    validate::check(original.clone(), declarations(&c)).unwrap();
    render(&checked(&c).unwrap()).unwrap();
    let branch_index = original
        .body
        .bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::If { .. }))
        .unwrap();
    for n in 0..2 {
        let Operation::If { then, .. } = &original.body.bindings[branch_index].operation else {
            unreachable!()
        };
        let mut operation = then.bindings[write_index(then)].operation.clone();
        let hidden = operand(&mut operation, n).clone();
        for sibling in [false, true] {
            let mut forged = original.clone();
            if sibling {
                let Operation::If { otherwise, .. } =
                    &mut forged.body.bindings[branch_index].operation
                else {
                    unreachable!()
                };
                let i = write_index(otherwise);
                *operand(&mut otherwise.bindings[i].operation, n) = hidden.clone();
            } else {
                let i = write_index(&forged.body);
                *operand(&mut forged.body.bindings[i].operation, n) = hidden.clone();
            }
            invalid(
                &c,
                forged,
                if sibling {
                    "sibling branch"
                } else {
                    "escaped branch"
                },
            );
        }
    }
}
