// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Independent declaration validation and shared candidate-checking fixture.
use super::*;

// The validator receives declaration truth independently of candidate operations.
fn declarations() -> declarations::Declarations {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    contract.witnesses[0].parameters.push(Parameter {
        name: "input".into(),
        ty: Type::Field,
    });
    declarations_for(&contract)
}
fn declarations_for(contract: &Contract) -> declarations::Declarations {
    declarations::Declarations::new(
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect(),
    )
}
pub(super) fn check_plan(plan: Plan) -> Result<CheckedPlan, RecordingGap> {
    validate::check(plan, declarations())
}
fn operation_plan(operation: Operation, ty: Type) -> Plan {
    let result = Value {
        id: 1,
        ty: ty.clone(),
    };
    Plan {
        parameters: vec![field(0)],
        body: Block {
            bindings: vec![Binding {
                value: result.clone(),
                operation,
                path: "effect".into(),
            }],
            result,
        },
        result_type: ty,
    }
}
#[test]
fn declaration_validator_rejects_missing_wrong_kind_and_wrong_index_slots() {
    let cases = [
        (
            Operation::CellRead(CellSlot {
                declaration: LedgerId(99),
                index: 0,
            }),
            Type::Field,
            "unknown Cell",
        ),
        (
            Operation::CellRead(CellSlot {
                declaration: LedgerId(0),
                index: 1,
            }),
            Type::Field,
            "expected Cell",
        ),
        (
            Operation::CellRead(CellSlot {
                declaration: LedgerId(1),
                index: 9,
            }),
            Type::Field,
            "index mismatch",
        ),
        (
            Operation::CounterRead {
                slot: CounterSlot {
                    declaration: LedgerId(99),
                    index: 1,
                },
            },
            validate::uint("18446744073709551615"),
            "unknown Counter",
        ),
        (
            Operation::CounterRead {
                slot: CounterSlot {
                    declaration: LedgerId(1),
                    index: 0,
                },
            },
            validate::uint("18446744073709551615"),
            "expected Counter",
        ),
        (
            Operation::CounterRead {
                slot: CounterSlot {
                    declaration: LedgerId(0),
                    index: 9,
                },
            },
            validate::uint("18446744073709551615"),
            "index mismatch",
        ),
    ];
    for (operation, ty, reason) in cases {
        let gap = check_plan(operation_plan(operation, ty)).unwrap_err();
        assert_eq!(gap.path, "effect");
        assert!(gap.detail.contains(reason), "{}", gap.detail);
    }
}
#[test]
fn declaration_validator_checks_cell_read_and_write_against_contract_type() {
    let cell = CellSlot {
        declaration: LedgerId(1),
        index: 0,
    };
    assert!(check_plan(operation_plan(Operation::CellRead(cell), Type::Field)).is_ok());
    assert!(
        check_plan(operation_plan(
            Operation::CellWrite {
                slot: cell,
                value: field(0)
            },
            Type::Unit
        ))
        .is_ok()
    );
    assert!(check_plan(operation_plan(Operation::CellRead(cell), Type::Boolean)).is_err());
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    contract.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean };
    for (operation, ty) in [
        (Operation::CellRead(cell), Type::Field),
        (
            Operation::CellWrite {
                slot: cell,
                value: field(0),
            },
            Type::Unit,
        ),
    ] {
        let gap = validate::check(operation_plan(operation, ty), declarations_for(&contract))
            .unwrap_err();
        assert_eq!(gap.path, "effect");
        assert_eq!(gap.detail, "operation input/output types are incompatible");
    }
}
#[test]
fn declaration_validator_checks_witness_existence_signature_and_result() {
    let witness = || Operation::Witness {
        declaration: WitnessId(0),
        arguments: vec![field(0)],
    };
    assert!(check_plan(operation_plan(witness(), Type::Field)).is_ok());
    let missing = Operation::Witness {
        declaration: WitnessId(99),
        arguments: vec![field(0)],
    };
    assert!(
        check_plan(operation_plan(missing, Type::Field))
            .unwrap_err()
            .detail
            .contains("unknown witness")
    );
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    // The contract witness has no arguments, despite the candidate's Field argument.
    assert!(
        validate::check(
            operation_plan(witness(), Type::Field),
            declarations_for(&contract)
        )
        .is_err()
    );
    contract.witnesses[0].parameters.push(Parameter {
        name: "input".into(),
        ty: Type::Boolean,
    });
    assert!(
        validate::check(
            operation_plan(witness(), Type::Field),
            declarations_for(&contract)
        )
        .is_err()
    );
    contract.witnesses[0].parameters[0].ty = Type::Field;
    contract.witnesses[0].result = Type::Boolean;
    assert!(
        validate::check(
            operation_plan(witness(), Type::Field),
            declarations_for(&contract)
        )
        .is_err()
    );
}
#[test]
fn declaration_environment_retains_full_path_and_deterministic_identities() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    contract.ledger_fields[0].path = vec![0, 3];
    let first = declarations_for(&contract);
    contract.ledger_fields.reverse();
    let second = declarations_for(&contract);
    let (a, declaration) = first.resolve_field("state").unwrap();
    let (b, _) = second.resolve_field("state").unwrap();
    assert_eq!(a, b);
    assert_eq!(declaration.physical_path(), vec![0, 3]);
    assert_eq!(
        first.resolve_witness("mark").unwrap().0,
        second.resolve_witness("mark").unwrap().0
    );
}
