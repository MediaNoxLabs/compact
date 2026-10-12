// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Forged plans must pass declaration, SSA and point operation signatures independently.
use super::*;
fn point(id: usize) -> Value {
    Value {
        id,
        ty: Type::JubjubPoint,
    }
}
fn scalar(id: usize) -> Value {
    Value {
        id,
        ty: Type::Field,
    }
}
fn validate_operation(
    operation: Operation,
    result_type: Type,
) -> Result<CheckedPlan, RecordingGap> {
    let result = Value {
        id: 4,
        ty: result_type.clone(),
    };
    let contract = contract();
    let declarations = declarations::Declarations::new(
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &HashMap::new(),
    );
    validate::check(
        Plan {
            parameters: vec![point(0), scalar(1), point(2), scalar(3)],
            body: Block {
                bindings: vec![Binding {
                    value: result.clone(),
                    operation,
                    path: "point_operation".into(),
                }],
                result,
            },
            result_type,
        },
        declarations,
    )
}
#[test]
fn validator_checks_each_jubjub_operand_result_and_scope_independently() {
    let valid = [
        (Operation::EcNeg(point(0)), Type::JubjubPoint),
        (Operation::JubjubScalarFromNative(scalar(1)), Type::Field),
        (Operation::EcMulGenerator(scalar(1)), Type::JubjubPoint),
        (
            Operation::EcMul {
                point: point(0),
                scalar: scalar(1),
            },
            Type::JubjubPoint,
        ),
        (
            Operation::EcAdd {
                left: point(0),
                right: point(2),
            },
            Type::JubjubPoint,
        ),
    ];
    for (operation, ty) in valid {
        assert!(validate_operation(operation.clone(), ty).is_ok());
        assert!(validate_operation(operation, Type::Boolean).is_err());
    }
    for input in [point(0), scalar(9)] {
        assert!(validate_operation(Operation::JubjubScalarFromNative(input), Type::Field).is_err());
    }
    for operation in [
        Operation::EcNeg(scalar(1)),
        Operation::EcNeg(point(1)),
        Operation::EcNeg(point(4)),
        Operation::EcNeg(point(9)),
        Operation::JubjubScalarFromNative(point(0)),
        Operation::EcMulGenerator(point(0)),
        Operation::EcMul {
            point: scalar(1),
            scalar: scalar(3),
        },
        Operation::EcMul {
            point: point(0),
            scalar: point(2),
        },
        Operation::EcAdd {
            left: scalar(1),
            right: point(0),
        },
        Operation::EcAdd {
            left: point(0),
            right: scalar(1),
        },
        Operation::JubjubScalarFromNative(scalar(9)),
        Operation::EcMulGenerator(scalar(9)),
        Operation::EcMul {
            point: point(9),
            scalar: scalar(1),
        },
        Operation::EcMul {
            point: point(0),
            scalar: scalar(9),
        },
        Operation::EcAdd {
            left: point(9),
            right: point(0),
        },
        Operation::EcAdd {
            left: point(0),
            right: point(9),
        },
        Operation::EcMul {
            point: point(1),
            scalar: scalar(3),
        },
    ] {
        let gap = validate_operation(operation, Type::JubjubPoint).unwrap_err();
        assert_eq!(gap.path, "point_operation");
    }
}
#[test]
fn validator_accepts_point_equality_only_with_live_matching_operands_and_boolean_result() {
    validate_operation(Operation::Equal(point(0), point(2)), Type::Boolean).unwrap();
    for (operation, result) in [
        (Operation::Equal(point(0), scalar(1)), Type::Boolean),
        (Operation::Equal(point(0), point(1)), Type::Boolean),
        (Operation::Equal(point(0), point(9)), Type::Boolean),
        (Operation::Equal(point(0), point(2)), Type::JubjubPoint),
    ] {
        let gap = validate_operation(operation, result).unwrap_err();
        assert_eq!(gap.path, "point_operation");
    }
}

#[test]
fn validator_keeps_existing_scalar_operation_domains_closed() {
    for (operation, ty) in [
        (Operation::Add(point(0), point(2)), Type::Field),
        (
            Operation::UnsignedAdd(point(0), point(2)),
            validate::uint("65535"),
        ),
        (
            Operation::Convert {
                value: point(0),
                kind: Conversion::UnsignedCast,
            },
            validate::uint("65535"),
        ),
        (
            Operation::Convert {
                value: point(0),
                kind: Conversion::Coerce,
            },
            Type::Field,
        ),
    ] {
        assert!(validate_operation(operation, ty).is_err());
    }
    assert!(
        validate_operation(
            Operation::Convert {
                value: point(0),
                kind: Conversion::Coerce
            },
            Type::JubjubPoint
        )
        .is_ok()
    );
}
#[test]
fn validator_checks_point_cell_value_result_and_declaration_identity() {
    let slot = CellSlot {
        declaration: LedgerId(0),
        index: 0,
    };
    assert!(validate_operation(Operation::CellRead(slot), Type::JubjubPoint).is_ok());
    assert!(
        validate_operation(
            Operation::CellWrite {
                slot,
                value: point(0)
            },
            Type::Unit
        )
        .is_ok()
    );
    for (operation, ty) in [
        (Operation::CellRead(slot), Type::Field),
        (
            Operation::CellWrite {
                slot,
                value: scalar(1),
            },
            Type::Unit,
        ),
        (
            Operation::CellWrite {
                slot,
                value: point(0),
            },
            Type::JubjubPoint,
        ),
        (
            Operation::CellRead(CellSlot {
                declaration: LedgerId(99),
                index: 0,
            }),
            Type::JubjubPoint,
        ),
        (
            Operation::CellRead(CellSlot {
                declaration: LedgerId(0),
                index: 9,
            }),
            Type::JubjubPoint,
        ),
    ] {
        assert!(validate_operation(operation, ty).is_err());
    }
}
