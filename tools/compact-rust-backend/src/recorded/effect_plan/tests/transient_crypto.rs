// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Crypto signature controls; runtime/proof qualification is separate.
use super::*;
fn field() -> Expr {
    Expr::FieldLiteral { value: "7".into() }
}
fn bytes(n: usize) -> Expr {
    Expr::BytesLiteral { bytes: vec![3; n] }
}
fn hash(value: Expr) -> Expr {
    Expr::TransientHash {
        value: Box::new(value),
    }
}
fn commit(value: Expr, opening: Expr) -> Expr {
    Expr::TransientCommit {
        value: Box::new(value),
        opening: Box::new(opening),
    }
}
fn checked(value: Expr) -> Result<CheckedPlan, PlanError> {
    let circuit = StatefulCircuit {
        source: None,
        name: "crypto".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::Expression { value }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    check(&circuit, &HashMap::new(), &HashMap::new())
}
fn accepted(value: Expr) {
    render(&checked(value).unwrap()).unwrap();
}
fn recheck(plan: Plan) -> Result<CheckedPlan, RecordingGap> {
    validate::check(
        plan,
        declarations::Declarations::new(&HashMap::new(), &HashMap::new()),
    )
}
#[test]
fn fixed_fab_products_include_primitive_leaves_and_exact_tuple_boundary() {
    let point = Expr::EcMulGenerator {
        scalar: Box::new(Expr::JubjubScalarFromNative {
            value: Box::new(field()),
        }),
    };
    for input in [
        Expr::Unit,
        field(),
        Expr::Boolean { value: true },
        bytes(0),
        bytes(32),
        point,
        Expr::UnsignedLiteral {
            value: "1".into(),
            max: "255".into(),
        },
        Expr::UnsignedLiteral {
            value: "1".into(),
            max: "340282366920938463463374607431768211456".into(),
        },
    ] {
        accepted(hash(input.clone()));
        accepted(commit(input, field()));
    }
    for length in [0, 1, 2, 11] {
        accepted(hash(Expr::Tuple {
            elements: vec![field(); length],
        }));
        accepted(commit(
            Expr::Tuple {
                elements: vec![field(); length],
            },
            field(),
        ));
    }
    accepted(hash(Expr::Vector {
        element: Type::Field,
        elements: vec![field(); 3],
    }));
    accepted(commit(
        Expr::StructLiteral {
            ty: Type::Struct {
                name: "FieldPair".into(),
                fields: vec![
                    StructField {
                        name: "left".into(),
                        ty: Type::Field,
                    },
                    StructField {
                        name: "right".into(),
                        ty: Type::Field,
                    },
                ],
            },
            fields: vec![field(), field()],
        },
        field(),
    ));
    assert!(matches!(
        checked(hash(Expr::Tuple {
            elements: vec![field(); 12]
        })),
        Err(PlanError::Unsupported(_))
    ));
    // Dynamic hash inputs and their owned defaults are independently qualified.
    for ty in [Type::OpaqueString, Type::OpaqueBytes] {
        assert!(validate::aligned_fab_type(&ty));
        assert!(validate::aligned_fab_type(&Type::Vector {
            element: Box::new(ty.clone()),
            length: 0
        }));
        accepted(hash(Expr::Default { ty }));
    }
}
#[test]
fn conversion_signatures_are_distinct_from_field_byte_encoding() {
    accepted(Expr::UpgradeFromTransient {
        value: Box::new(field()),
    });
    accepted(Expr::DegradeToTransient {
        value: Box::new(bytes(32)),
    });
    for n in [0, 31, 33] {
        assert!(matches!(
            checked(Expr::DegradeToTransient {
                value: Box::new(bytes(n))
            }),
            Err(PlanError::Unsupported(_))
        ));
    }
    assert!(matches!(
        checked(Expr::UpgradeFromTransient {
            value: Box::new(bytes(32))
        }),
        Err(PlanError::Unsupported(_))
    ));
    assert!(matches!(
        checked(commit(field(), bytes(32))),
        Err(PlanError::Unsupported(_))
    ));
    let plan = checked(Expr::UpgradeFromTransient {
        value: Box::new(field()),
    })
    .unwrap();
    assert!(
        plan.plan()
            .body
            .bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::UpgradeFromTransient(_)))
    );
    assert!(
        !plan
            .plan()
            .body
            .bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::FieldToBytes32(_)))
    );
}
fn observed_field() -> Expr {
    Expr::DegradeToTransient {
        value: Box::new(Expr::StructField {
            value: Box::new(Expr::KernelSelf {
                ty: crate::ir::contract_address_type(),
            }),
            field: "bytes".into(),
            index: 0,
        }),
    }
}
#[test]
fn commitment_evaluates_value_then_opening_once_in_distinct_ssa_values() {
    let plan = checked(commit(observed_field(), observed_field())).unwrap();
    let bindings = &plan.plan().body.bindings;
    let observations: Vec<_> = bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::KernelSelf))
        .collect();
    assert_eq!(observations.len(), 2);
    assert!(observations[0].path.contains(".value.value.value"));
    assert!(observations[1].path.contains(".opening.value.value"));
    let Operation::TransientCommit { value, opening } = &bindings
        .iter()
        .find(|b| matches!(b.operation, Operation::TransientCommit { .. }))
        .unwrap()
        .operation
    else {
        unreachable!()
    };
    assert_ne!(value.id, opening.id);
    assert!(value.id < opening.id);
}
#[test]
fn independent_validator_rejects_wrong_signature_results_and_live_operand_references() {
    for expr in [
        hash(field()),
        commit(field(), field()),
        Expr::DegradeToTransient {
            value: Box::new(bytes(32)),
        },
        Expr::UpgradeFromTransient {
            value: Box::new(field()),
        },
    ] {
        let original = checked(expr).unwrap().plan().clone();
        recheck(original.clone()).unwrap();
        let position = original
            .body
            .bindings
            .iter()
            .position(|b| {
                matches!(
                    b.operation,
                    Operation::TransientHash(_)
                        | Operation::TransientCommit { .. }
                        | Operation::DegradeToTransient(_)
                        | Operation::UpgradeFromTransient(_)
                )
            })
            .unwrap();
        for mode in 0..3 {
            let mut plan = original.clone();
            if mode == 0 {
                plan.body.bindings[position].value.ty = Type::Boolean;
            } else {
                let operand = match &mut plan.body.bindings[position].operation {
                    Operation::TransientHash(v)
                    | Operation::DegradeToTransient(v)
                    | Operation::UpgradeFromTransient(v) => v,
                    Operation::TransientCommit { opening, .. } => opening,
                    _ => unreachable!(),
                };
                if mode == 1 {
                    operand.id = 999;
                } else {
                    operand.ty = Type::Boolean;
                }
            }
            assert!(recheck(plan).is_err());
        }
        // Change the actual definition and use together so the unary conversion
        // and commitment opening checks cannot pass merely by trusting annotations.
        if !matches!(
            original.body.bindings[position].operation,
            Operation::TransientHash(_)
        ) {
            let mut plan = original.clone();
            let mut operation = plan.body.bindings[position].operation.clone();
            let operand = match &mut operation {
                Operation::DegradeToTransient(v) | Operation::UpgradeFromTransient(v) => v,
                Operation::TransientCommit { opening, .. } => opening,
                _ => unreachable!(),
            };
            let definition = plan
                .body
                .bindings
                .iter_mut()
                .find(|b| b.value.id == operand.id)
                .unwrap();
            definition.value.ty = Type::Boolean;
            definition.operation = Operation::Literal(Literal::Boolean(false));
            operand.ty = Type::Boolean;
            plan.body.bindings[position].operation = operation;
            assert!(recheck(plan).is_err());
        }
    }
}
#[test]
fn independent_validator_accepts_opaque_defaults_but_rejects_missing_traits_and_scope_escape() {
    for expression in [hash(field()), commit(field(), field())] {
        let mut plan = checked(expression).unwrap().plan().clone();
        let position = plan
            .body
            .bindings
            .iter()
            .position(|b| {
                matches!(
                    b.operation,
                    Operation::TransientHash(_) | Operation::TransientCommit { .. }
                )
            })
            .unwrap();
        let mut operation = plan.body.bindings[position].operation.clone();
        let operand = match &mut operation {
            Operation::TransientHash(value) | Operation::TransientCommit { value, .. } => value,
            _ => unreachable!(),
        };
        let definition = plan
            .body
            .bindings
            .iter_mut()
            .find(|b| b.value.id == operand.id)
            .unwrap();
        definition.value.ty = Type::OpaqueString;
        definition.operation = Operation::Default;
        operand.ty = Type::OpaqueString;
        plan.body.bindings[position].operation = operation;
        recheck(plan.clone()).unwrap();
        // A matching annotation cannot fabricate a tuple13 Default implementation.
        let unsupported = Type::Tuple {
            elements: vec![Type::Field; 13],
        };
        let definition = plan
            .body
            .bindings
            .iter_mut()
            .find(|b| matches!(b.operation, Operation::Default))
            .unwrap();
        definition.value.ty = unsupported;
        assert!(recheck(plan).is_err());
    }
    let mut plan = checked(Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(hash(field())),
        otherwise: Box::new(hash(field())),
    })
    .unwrap()
    .plan()
    .clone();
    let branch = plan
        .body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::If { .. }))
        .unwrap();
    let Operation::If {
        then, otherwise, ..
    } = &mut branch.operation
    else {
        unreachable!()
    };
    let escaped = then
        .bindings
        .iter()
        .find(|b| matches!(b.operation, Operation::Literal(Literal::Field(_))))
        .unwrap()
        .value
        .clone();
    let sibling_hash = otherwise
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::TransientHash(_)))
        .unwrap();
    sibling_hash.operation = Operation::TransientHash(escaped);
    assert!(recheck(plan).is_err());
}
