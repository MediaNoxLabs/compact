// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Kernel invariants verified independently of source lowering.
use super::*;
fn bytes_type() -> Type {
    Type::Bytes { length: 32 }
}
fn amount_type() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn value(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(id: usize, ty: Type, operation: Operation) -> Binding {
    Binding {
        value: value(id, ty),
        operation,
        path: format!("kernel[{id}]"),
    }
}
fn plan(operation: Operation) -> Plan {
    Plan {
        parameters: vec![value(0, bytes_type()), value(1, amount_type())],
        result_type: Type::Unit,
        body: Block {
            bindings: vec![binding(2, Type::Unit, operation)],
            result: value(2, Type::Unit),
        },
    }
}
fn validate(plan: Plan) -> Result<CheckedPlan, RecordingGap> {
    validate::check(plan, declarations::Declarations::default())
}
#[test]
fn all_closed_claim_kinds_require_bytes32_live_input_and_unit_output() {
    for kind in [
        KernelClaimKind::Nullifier,
        KernelClaimKind::CoinSpend,
        KernelClaimKind::CoinReceive,
    ] {
        let original = plan(Operation::KernelClaim {
            kind,
            value: value(0, bytes_type()),
        });
        validate(original.clone()).unwrap();
        for mode in 0..4 {
            let mut bad = original.clone();
            match mode {
                0 => {
                    if let Operation::KernelClaim { value, .. } =
                        &mut bad.body.bindings[0].operation
                    {
                        *value = Value {
                            id: 1,
                            ty: amount_type(),
                        };
                    }
                }
                1 => {
                    if let Operation::KernelClaim { value, .. } =
                        &mut bad.body.bindings[0].operation
                    {
                        value.id = 99;
                    }
                }
                2 => {
                    if let Operation::KernelClaim { value, .. } =
                        &mut bad.body.bindings[0].operation
                    {
                        value.ty = Type::Bytes { length: 31 };
                    }
                }
                3 => {
                    bad.body.bindings[0].value.ty = Type::Boolean;
                    bad.body.result.ty = Type::Boolean;
                    bad.result_type = Type::Boolean;
                }
                _ => unreachable!(),
            }
            assert!(validate(bad).is_err(), "claim mutation {mode}");
        }
    }
}
#[test]
fn mint_requires_exact_uint64_domain_order_and_unit_result() {
    let original = plan(Operation::KernelMintShielded {
        domain: value(0, bytes_type()),
        amount: value(1, amount_type()),
    });
    validate(original.clone()).unwrap();
    for mode in 0..7 {
        let mut bad = original.clone();
        match mode {
            0 | 1 => {
                let ty = Type::Unsigned {
                    max: if mode == 0 {
                        "255".into()
                    } else {
                        (u64::MAX as u128 + 1).to_string()
                    },
                };
                bad.parameters[1].ty = ty.clone();
                if let Operation::KernelMintShielded { amount, .. } =
                    &mut bad.body.bindings[0].operation
                {
                    amount.ty = ty;
                }
            }
            2 => {
                if let Operation::KernelMintShielded { domain, amount } =
                    &mut bad.body.bindings[0].operation
                {
                    std::mem::swap(domain, amount);
                }
            }
            3 => {
                if let Operation::KernelMintShielded { amount, .. } =
                    &mut bad.body.bindings[0].operation
                {
                    amount.id = 99;
                }
            }
            4 => {
                if let Operation::KernelMintShielded { amount, .. } =
                    &mut bad.body.bindings[0].operation
                {
                    amount.ty = Type::Field;
                }
            }
            5 => {
                bad.body.bindings[0].value.ty = Type::Boolean;
                bad.body.result.ty = Type::Boolean;
                bad.result_type = Type::Boolean;
            }
            6 => {
                bad.parameters.remove(0);
            }
            _ => unreachable!(),
        }
        assert!(validate(bad).is_err(), "mint mutation {mode}");
    }
}
#[test]
fn kernel_cannot_use_a_value_from_a_sibling_branch_scope() {
    let unit = |id| binding(id, Type::Unit, Operation::Literal(Literal::Unit));
    let bad = Plan {
        parameters: vec![value(0, Type::Boolean)],
        result_type: Type::Unit,
        body: Block {
            bindings: vec![
                binding(
                    1,
                    Type::Unit,
                    Operation::If {
                        condition: value(0, Type::Boolean),
                        then: Block {
                            bindings: vec![
                                binding(
                                    2,
                                    bytes_type(),
                                    Operation::Literal(Literal::Bytes(vec![0; 32])),
                                ),
                                unit(3),
                            ],
                            result: value(3, Type::Unit),
                        },
                        otherwise: Block {
                            bindings: vec![unit(4)],
                            result: value(4, Type::Unit),
                        },
                    },
                ),
                binding(
                    5,
                    Type::Unit,
                    Operation::KernelClaim {
                        kind: KernelClaimKind::Nullifier,
                        value: value(2, bytes_type()),
                    },
                ),
            ],
            result: value(5, Type::Unit),
        },
    };
    assert_eq!(validate(bad).unwrap_err().path, "kernel[5]");
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Expression { value },
    }
}
#[test]
fn mint_witness_operands_evaluate_once_in_domain_then_amount_order() {
    let witnesses = [
        WitnessDeclaration {
            source: None,
            name: "domain".into(),
            parameters: vec![],
            result: bytes_type(),
        },
        WitnessDeclaration {
            source: None,
            name: "amount".into(),
            parameters: vec![],
            result: amount_type(),
        },
    ];
    let maps = witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let witness = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    let checked = check(
        &root(Expr::KernelMintShielded {
            domain: Box::new(witness("domain")),
            amount: Box::new(witness("amount")),
        }),
        &HashMap::new(),
        &maps,
    )
    .unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 3);
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(bindings[1].operation, Operation::Witness { .. }));
    assert_eq!(bindings[0].path, "return_value.value.domain");
    assert_eq!(bindings[1].path, "return_value.value.amount");
    let Operation::KernelMintShielded { domain, amount } = &bindings[2].operation else {
        panic!("mint follows both witnesses")
    };
    assert_eq!(domain, &bindings[0].value);
    assert_eq!(amount, &bindings[1].value);
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    let source = quote::quote!(#(#steps)*).to_string();
    assert_eq!(source.matches("try_witness_metered").count(), 2);
    assert_eq!(source.matches("kernel_mint_shielded").count(), 1);
    let error = check(
        &root(Expr::KernelMintShielded {
            domain: Box::new(witness("missing_domain")),
            amount: Box::new(witness("missing_amount")),
        }),
        &HashMap::new(),
        &maps,
    )
    .unwrap_err();
    let PlanError::Unsupported(gap) = error else {
        panic!("source refusal")
    };
    assert_eq!(gap.path, "return_value.value.domain");
}
