// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Counter declarations and operation signatures are checked independently.
use super::*;

fn operation_plan(operation: Operation, input: Value, output_type: Type) -> Plan {
    let result = Value {
        id: 1,
        ty: output_type.clone(),
    };
    Plan {
        parameters: vec![input],
        body: Block {
            bindings: vec![Binding {
                value: result.clone(),
                operation,
                path: "counter_effect".into(),
            }],
            result,
        },
        result_type: output_type,
    }
}
#[test]
fn validator_independently_checks_counter_operand_and_result_signatures() {
    let contract = contract();
    let environment = declarations(&contract);
    let slot = CounterSlot {
        declaration: environment.resolve_field("round").unwrap().0,
        index: 1,
    };
    let amount = Value {
        id: 0,
        ty: validate::uint(U16_MAX),
    };
    let threshold = Value {
        id: 0,
        ty: validate::uint(U64_MAX),
    };
    let cases = [
        (
            Operation::CounterDecrement {
                slot,
                amount: amount.clone(),
            },
            amount.clone(),
            Type::Unit,
            true,
        ),
        (
            Operation::CounterDecrement {
                slot,
                amount: threshold.clone(),
            },
            threshold.clone(),
            Type::Unit,
            false,
        ),
        (
            Operation::CounterDecrement {
                slot,
                amount: amount.clone(),
            },
            amount.clone(),
            Type::Field,
            false,
        ),
        (
            Operation::CounterReset { slot },
            amount.clone(),
            Type::Unit,
            true,
        ),
        (
            Operation::CounterReset { slot },
            amount.clone(),
            Type::Boolean,
            false,
        ),
        (
            Operation::CounterLessThan {
                slot,
                threshold: threshold.clone(),
            },
            threshold.clone(),
            Type::Boolean,
            true,
        ),
        (
            Operation::CounterLessThan {
                slot,
                threshold: amount.clone(),
            },
            amount.clone(),
            Type::Boolean,
            false,
        ),
        (
            Operation::CounterLessThan {
                slot,
                threshold: threshold.clone(),
            },
            threshold.clone(),
            Type::Field,
            false,
        ),
        (
            Operation::CounterLessThan {
                slot,
                threshold: Value {
                    id: 8,
                    ty: threshold.ty.clone(),
                },
            },
            threshold,
            Type::Boolean,
            false,
        ),
        (
            Operation::CounterDecrement {
                slot,
                amount: Value {
                    id: 8,
                    ty: amount.ty.clone(),
                },
            },
            amount,
            Type::Unit,
            false,
        ),
    ];
    for (operation, input, output, valid) in cases {
        let result = validate::check(
            operation_plan(operation, input, output),
            declarations(&contract),
        );
        assert_eq!(result.is_ok(), valid, "{result:?}");
        if let Err(gap) = result {
            assert_eq!(gap.path, "counter_effect");
        }
    }
}

#[test]
fn validator_checks_declarations_for_each_new_counter_operation() {
    let contract = contract();
    let environment = declarations(&contract);
    let round = environment.resolve_field("round").unwrap().0;
    let state = environment.resolve_field("state").unwrap().0;
    for slot in [
        CounterSlot {
            declaration: LedgerId(99),
            index: 1,
        },
        CounterSlot {
            declaration: state,
            index: 0,
        },
        CounterSlot {
            declaration: round,
            index: 9,
        },
    ] {
        let amount = Value {
            id: 0,
            ty: validate::uint(U16_MAX),
        };
        let threshold = Value {
            id: 0,
            ty: validate::uint(U64_MAX),
        };
        for (operation, input, output) in [
            (
                Operation::CounterDecrement {
                    slot,
                    amount: amount.clone(),
                },
                amount.clone(),
                Type::Unit,
            ),
            (Operation::CounterReset { slot }, amount, Type::Unit),
            (
                Operation::CounterLessThan {
                    slot,
                    threshold: threshold.clone(),
                },
                threshold,
                Type::Boolean,
            ),
        ] {
            let gap = validate::check(
                operation_plan(operation, input, output),
                declarations(&contract),
            )
            .unwrap_err();
            assert_eq!(gap.path, "counter_effect");
            assert!(
                gap.detail.contains("Counter") || gap.detail.contains("index mismatch"),
                "{}",
                gap.detail
            );
        }
    }
}
