// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Qualified writes bind typed operands; runtime owns commitment allocation.
//! These compiler checks establish neither funding nor proof validity.
use super::*;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};

#[path = "qualified_coin_write/validation.rs"]
mod validation;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn write(coin: Expr, recipient: Expr) -> StateAction {
    StateAction::CellWriteCoin {
        field: "pot".into(),
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
fn source() -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "pot".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: LedgerFieldKind::Cell {
                ty: qualified_coin_type(),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            name: "write".into(),
            source: None,
            internal: false,
            parameters: vec![
                Parameter {
                    name: "coin".into(),
                    ty: shielded_coin_type(),
                },
                Parameter {
                    name: "recipient".into(),
                    ty: shielded_recipient_type(),
                },
            ],
            actions: vec![write(parameter("coin"), parameter("recipient"))],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &HashMap::new(),
        &c.stateful_circuits
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect(),
    )
}
fn declarations(c: &Contract) -> declarations::Declarations {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let calls = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    let types = type_environment::TypeEnvironment::from_sources(
        &c.stateful_circuits[0],
        &ledger,
        &witnesses,
        &HashMap::new(),
        &calls,
    )
    .unwrap();
    declarations::Declarations::with_calls(&ledger, &witnesses, &HashMap::new(), &calls)
        .with_type_environment(types)
        .with_root(&c.stateful_circuits[0])
}
fn write_index(body: &Block) -> usize {
    body.bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::CellWriteCoin { .. }))
        .unwrap()
}
fn operand(operation: &mut Operation, index: usize) -> &mut Value {
    let Operation::CellWriteCoin {
        coin, recipient, ..
    } = operation
    else {
        unreachable!()
    };
    if index == 0 { coin } else { recipient }
}
fn rendered(plan: &CheckedPlan) -> String {
    let rendered = render(plan).unwrap();
    let steps = rendered.steps;
    quote::quote!(#(#steps)*).to_string()
}
fn malformed(ty: Type, mode: usize) -> Type {
    let Type::Struct {
        mut name,
        mut fields,
    } = ty
    else {
        unreachable!()
    };
    match mode {
        0 => name.push_str("Lookalike"),
        1 => fields.swap(0, 1),
        2 => fields[0].name = "renamed".into(),
        3 => fields[0].ty = Type::Field,
        4 => {
            fields.pop();
        }
        _ => unreachable!(),
    }
    Type::Struct { name, fields }
}

#[test]
fn canonical_write_uses_declared_slot_and_official_converters_without_intents() {
    let c = source();
    let plan = checked(&c).unwrap();
    let i = write_index(&plan.plan().body);
    let binding = &plan.plan().body.bindings[i];
    let Operation::CellWriteCoin {
        slot,
        coin,
        recipient,
    } = &binding.operation
    else {
        unreachable!()
    };
    assert_eq!(
        slot.declaration,
        declarations(&c).resolve_field("pot").unwrap().0
    );
    assert_eq!(slot.index, 0);
    assert_eq!(coin, &plan.plan().parameters[0]);
    assert_eq!(recipient, &plan.plan().parameters[1]);
    assert_eq!(binding.value.ty, Type::Unit);
    let rust = rendered(&plan);
    for token in [
        "ledger_slots :: pot",
        "record_write_coin",
        "coin_info_from_compact",
        "coin_recipient_from_compact",
    ] {
        assert!(rust.contains(token), "{token}: {rust}");
    }
    assert_eq!(rust.matches("record_write_coin").count(), 1);
    assert!(!rust.contains("create_zswap"));
    assert!(!rust.contains("mt_index"));
}

#[test]
fn source_rejects_wrong_slot_identity_kind_and_canonical_carrier_layouts() {
    for mode in ["missing", "index", "ordinary", "set", "counter"] {
        let mut c = source();
        match mode {
            "missing" => {
                let StateAction::CellWriteCoin { field, .. } =
                    &mut c.stateful_circuits[0].actions[0]
                else {
                    unreachable!()
                };
                *field = "absent".into();
            }
            "index" => {
                let StateAction::CellWriteCoin { index, .. } =
                    &mut c.stateful_circuits[0].actions[0]
                else {
                    unreachable!()
                };
                *index = 1;
            }
            "ordinary" => {
                c.ledger_fields[0].declaration = LedgerFieldKind::Cell {
                    ty: shielded_coin_type(),
                }
            }
            "set" => {
                c.ledger_fields[0].declaration = LedgerFieldKind::Set {
                    ty: qualified_coin_type(),
                }
            }
            "counter" => c.ledger_fields[0].declaration = LedgerFieldKind::Counter,
            _ => unreachable!(),
        }
        assert!(
            matches!(checked(&c), Err(PlanError::Unsupported(_))),
            "{mode}"
        );
    }
    for target in 0..3 {
        for mode in 0..5 {
            let mut c = source();
            if target == 0 {
                c.ledger_fields[0].declaration = LedgerFieldKind::Cell {
                    ty: malformed(qualified_coin_type(), mode),
                };
            } else {
                let parameter = &mut c.stateful_circuits[0].parameters[target - 1];
                parameter.ty = malformed(parameter.ty.clone(), mode);
            }
            assert!(
                matches!(checked(&c), Err(PlanError::Unsupported(_))),
                "target={target}, mutation={mode}"
            );
        }
    }
    for n in 0..2 {
        let mut c = source();
        let StateAction::CellWriteCoin {
            coin, recipient, ..
        } = &mut c.stateful_circuits[0].actions[0]
        else {
            unreachable!()
        };
        *if n == 0 { coin } else { recipient } = Expr::Boolean { value: false };
        let Err(PlanError::Unsupported(gap)) = checked(&c) else {
            panic!("bad operand must be source refusal")
        };
        assert_eq!(
            gap.path,
            if n == 0 {
                "actions[0].coin"
            } else {
                "actions[0].recipient"
            }
        );
    }
}

#[test]
fn witness_operands_are_bound_once_coin_before_recipient_and_reused() {
    let mut c = source();
    c.witnesses = [
        ("nextCoin", shielded_coin_type()),
        ("nextRecipient", shielded_recipient_type()),
    ]
    .into_iter()
    .map(|(name, result)| WitnessDeclaration {
        name: name.into(),
        source: None,
        parameters: vec![],
        result,
    })
    .collect();
    c.stateful_circuits[0].parameters.clear();
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
            actions: vec![
                write(parameter("coin"), parameter("recipient")),
                write(parameter("coin"), parameter("recipient")),
            ],
        }),
    }];
    let checked = checked(&c).unwrap();
    let bindings = &checked.plan().body.bindings;
    let witnesses: Vec<_> = bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::Witness { .. }))
        .collect();
    let writes: Vec<_> = bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::CellWriteCoin { .. }))
        .collect();
    assert_eq!((witnesses.len(), writes.len()), (2, 2));
    assert_eq!(witnesses[0].value.ty, shielded_coin_type());
    assert_eq!(witnesses[1].value.ty, shielded_recipient_type());
    assert!(
        bindings
            .iter()
            .position(|b| b.value == witnesses[1].value)
            .unwrap()
            < write_index(&checked.plan().body)
    );
    for write in writes {
        let Operation::CellWriteCoin {
            coin, recipient, ..
        } = &write.operation
        else {
            unreachable!()
        };
        assert_eq!(coin, &witnesses[0].value);
        assert_eq!(recipient, &witnesses[1].value);
    }
    render(&checked).unwrap();
    // Direct expression operands retain their own source paths and evaluation order.
    c.stateful_circuits[0].actions = vec![write(witness("nextCoin"), witness("nextRecipient"))];
    let plan = super::qualified_coin_write::checked(&c).unwrap();
    assert_eq!(plan.plan().body.bindings[0].path, "actions[0].coin");
    assert_eq!(plan.plan().body.bindings[1].path, "actions[0].recipient");
    assert_eq!(write_index(&plan.plan().body), 2);
    render(&plan).unwrap();
}

#[test]
fn local_shadowing_and_helper_forwarding_use_fresh_actual_values() {
    let mut c = source();
    let mut helper = c.stateful_circuits[0].clone();
    helper.name = "helper".into();
    helper.internal = true;
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "coin".into(),
            ty: shielded_coin_type(),
            value: Expr::Default {
                ty: shielded_coin_type(),
            },
        }],
        action: Box::new(StateAction::CircuitCall {
            name: "helper".into(),
            arguments: vec![parameter("coin"), parameter("recipient")],
        }),
    }];
    c.stateful_circuits.push(helper);
    let checked = checked(&c).unwrap();
    let plan = checked.plan();
    let local = plan
        .body
        .bindings
        .iter()
        .find(|b| matches!(b.operation, Operation::Default))
        .unwrap();
    let call = plan
        .body
        .bindings
        .iter()
        .find(|b| matches!(b.operation, Operation::Call { .. }))
        .unwrap();
    let Operation::Call {
        arguments,
        parameters,
        body,
        ..
    } = &call.operation
    else {
        unreachable!()
    };
    assert_eq!(arguments[0], local.value);
    assert_ne!(arguments[0], plan.parameters[0]);
    assert_eq!(arguments[1], plan.parameters[1]);
    let Operation::CellWriteCoin {
        coin, recipient, ..
    } = &body.bindings[write_index(body)].operation
    else {
        unreachable!()
    };
    assert_eq!(coin, &parameters[0]);
    assert_eq!(recipient, &parameters[1]);
    assert_ne!(coin.id, arguments[0].id);
    validate::check(plan.clone(), declarations(&c)).unwrap();
    render(&checked).unwrap();
}

#[test]
fn fresh_frontend_nested_path_and_public_body_owners_are_current_algebra() {
    use crate::recorded::admission_probe::trace;
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-qualified-coin-source-schema20-ir.json"
    ))
    .unwrap();
    assert_eq!(c.ledger_fields.len(), 16);
    let pot = c.ledger_fields.iter().find(|f| matches!(&f.declaration, LedgerFieldKind::Cell { ty } if *ty == qualified_coin_type())).unwrap();
    assert_eq!(pot.path, vec![1, 14]);
    assert_eq!(pot.path[0], pot.index);
    for name in ["write_once", "write_read"] {
        let mut selected = c.clone();
        let i = selected
            .stateful_circuits
            .iter()
            .position(|r| r.name == name)
            .unwrap();
        selected.stateful_circuits.swap(0, i);
        let plan = checked(&selected).unwrap();
        let rust = rendered(&plan);
        assert!(rust.contains(&format!("ledger_slots :: {}", pot.id)));
    }
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = output.unwrap();
    // The zero-parameter Unit interface reaches membership composition first;
    // that route composes checked effects without requiring a membership-shaped body.
    for (name, owner) in [
        ("write_once", "algebra.membership"),
        ("write_read", "algebra.composite-values"),
    ] {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call);
        let owners: Vec<_> = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0]["body_owner"], owner);
    }
}

#[test]
fn source_requires_exact_coin_bounds_and_nested_recipient_bytes() {
    for (target, field, wrong) in [
        (
            0,
            2,
            Type::Unsigned {
                max: u64::MAX.to_string(),
            },
        ),
        (
            0,
            3,
            Type::Unsigned {
                max: u128::MAX.to_string(),
            },
        ),
        (
            1,
            2,
            Type::Unsigned {
                max: u64::MAX.to_string(),
            },
        ),
        (1, 0, Type::Bytes { length: 31 }),
        (1, 1, Type::Bytes { length: 33 }),
    ] {
        let mut c = source();
        let ty = if target == 0 {
            let LedgerFieldKind::Cell { ty } = &mut c.ledger_fields[0].declaration else {
                unreachable!()
            };
            ty
        } else {
            &mut c.stateful_circuits[0].parameters[0].ty
        };
        let Type::Struct { fields, .. } = ty else {
            unreachable!()
        };
        fields[field].ty = wrong;
        assert!(
            matches!(checked(&c), Err(PlanError::Unsupported(_))),
            "target={target}, field={field}"
        );
    }
    for branch in [1, 2] {
        for rename in [false, true] {
            let mut c = source();
            let Type::Struct { fields, .. } = &mut c.stateful_circuits[0].parameters[1].ty else {
                unreachable!()
            };
            let Type::Struct { name, fields } = &mut fields[branch].ty else {
                unreachable!()
            };
            if rename {
                name.push_str("Lookalike");
            } else {
                fields[0].ty = Type::Bytes { length: 31 };
            }
            assert!(
                matches!(checked(&c), Err(PlanError::Unsupported(_))),
                "recipient branch={branch}, rename={rename}"
            );
        }
    }
}
