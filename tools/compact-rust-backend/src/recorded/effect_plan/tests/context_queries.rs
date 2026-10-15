// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent context operation invariants and ordered source lowering.
use super::super::type_environment::TypeEnvironment;
use super::*;
fn bytes_type() -> Type {
    Type::Bytes { length: 32 }
}
fn address_type() -> Type {
    Type::Struct {
        name: "ContractAddress".into(),
        fields: vec![StructField {
            name: "bytes".into(),
            ty: bytes_type(),
        }],
    }
}
fn address() -> Expr {
    Expr::KernelSelf { ty: address_type() }
}
fn project(value: Expr) -> Expr {
    Expr::StructField {
        value: Box::new(value),
        field: "bytes".into(),
        index: 0,
    }
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "query".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: bytes_type(),
        return_value: StateReturn::Expression { value },
    }
}
fn environment(ty: Type) -> declarations::Declarations {
    let mut source = root(Expr::KernelSelf { ty: ty.clone() });
    source.result = ty;
    let types = TypeEnvironment::from_sources(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&HashMap::new(), &HashMap::new()).with_type_environment(types)
}
fn value(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(id: usize, ty: Type, operation: Operation) -> Binding {
    Binding {
        value: value(id, ty),
        operation,
        path: format!("query[{id}]"),
    }
}
fn plan(bindings: Vec<Binding>, result: Value) -> Plan {
    Plan {
        parameters: vec![],
        result_type: result.ty.clone(),
        body: Block { bindings, result },
    }
}
fn checked(value: Expr) -> CheckedPlan {
    check(&root(value), &HashMap::new(), &HashMap::new()).unwrap()
}

#[test]
fn kernel_requires_exact_builtin_shape_even_when_source_registers_another_shape() {
    let good = plan(
        vec![binding(0, address_type(), Operation::KernelSelf)],
        value(0, address_type()),
    );
    validate::check(good.clone(), environment(address_type())).unwrap();
    assert!(validate::check(good, declarations::Declarations::default()).is_err());
    for mode in 0..4 {
        let mut ty = address_type();
        let Type::Struct { name, fields } = &mut ty else {
            unreachable!()
        };
        match mode {
            0 => *name = "OtherAddress".into(),
            1 => fields[0].name = "payload".into(),
            2 => fields[0].ty = Type::Bytes { length: 31 },
            3 => fields.push(StructField {
                name: "extra".into(),
                ty: Type::Field,
            }),
            _ => unreachable!(),
        }
        let forged = plan(
            vec![binding(0, ty.clone(), Operation::KernelSelf)],
            value(0, ty.clone()),
        );
        assert_eq!(
            validate::check(forged, environment(ty)).unwrap_err().path,
            "query[0]",
            "mutation {mode}"
        );
    }
    let forged = plan(
        vec![binding(0, bytes_type(), Operation::KernelSelf)],
        value(0, bytes_type()),
    );
    assert_eq!(
        validate::check(forged, environment(address_type()))
            .unwrap_err()
            .path,
        "query[0]"
    );
}
fn commitment() -> Plan {
    let left = value(0, bytes_type());
    let right = value(1, bytes_type());
    let pair = value(
        2,
        Type::Tuple {
            elements: vec![bytes_type(), bytes_type()],
        },
    );
    plan(
        vec![
            binding(
                0,
                bytes_type(),
                Operation::Literal(Literal::Bytes(vec![1; 32])),
            ),
            binding(
                1,
                bytes_type(),
                Operation::Literal(Literal::Bytes(vec![2; 32])),
            ),
            binding(
                2,
                pair.ty.clone(),
                Operation::Tuple {
                    elements: vec![left.clone(), right],
                },
            ),
            binding(
                3,
                bytes_type(),
                Operation::PersistentCommit {
                    value: pair,
                    opening: left,
                },
            ),
        ],
        value(3, bytes_type()),
    )
}
#[test]
fn forged_commit_preserves_opening_output_and_live_operands() {
    validate::check(commitment(), environment(address_type())).unwrap();
    // A live Bytes32 operand is a valid fixed FAB, not a forged pair anymore.
    let mut single = commitment();
    let Operation::PersistentCommit { value: input, .. } = &mut single.body.bindings[3].operation
    else {
        unreachable!()
    };
    *input = value(0, bytes_type());
    validate::check(single, environment(address_type())).unwrap();
    for mode in 1..6 {
        let mut forged = commitment();
        let Operation::PersistentCommit {
            value: input,
            opening,
        } = &mut forged.body.bindings[3].operation
        else {
            unreachable!()
        };
        match mode {
            1 => {
                *opening = value(
                    2,
                    Type::Tuple {
                        elements: vec![bytes_type(), bytes_type()],
                    },
                )
            }
            2 => input.id = 99,
            3 => opening.ty = Type::Bytes { length: 31 },
            4 => {
                forged.body.bindings[3].value.ty = Type::Field;
                forged.body.result.ty = Type::Field;
                forged.result_type = Type::Field;
            }
            5 => {
                forged.body.bindings.swap(2, 3);
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(forged, environment(address_type())).is_err(),
            "mutation {mode}"
        );
    }
}
#[test]
fn kernel_observations_lower_once_left_to_right_before_commit() {
    let checked = checked(Expr::PersistentCommit {
        value: Box::new(Expr::Tuple {
            elements: vec![project(address()), project(address())],
        }),
        opening: Box::new(project(address())),
    });
    let plan = checked.plan();
    let paths: Vec<_> = plan
        .body
        .bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::KernelSelf))
        .map(|b| b.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "return_value.value.value.elements[0].value",
            "return_value.value.value.elements[1].value",
            "return_value.value.opening.value"
        ]
    );
    let Operation::PersistentCommit { value, opening } =
        &plan.body.bindings.last().unwrap().operation
    else {
        panic!("commit last")
    };
    assert_eq!(value.id, 4);
    assert_eq!(opening.id, 6);
    assert_eq!(plan.body.bindings.len(), 8);
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("kernel_self").count(), 3);
    assert_eq!(tokens.matches("contract_address_bytes").count(), 3);
    assert_eq!(tokens.matches("persistent_commit").count(), 1);
}
#[test]
fn unsupported_first_commit_operand_keeps_source_path_before_opening() {
    let missing = || Expr::Parameter {
        name: "missing".into(),
    };
    let source = root(Expr::PersistentCommit {
        value: Box::new(Expr::Tuple {
            elements: vec![missing(), missing()],
        }),
        opening: Box::new(missing()),
    });
    let PlanError::Unsupported(gap) = check(&source, &HashMap::new(), &HashMap::new()).unwrap_err()
    else {
        panic!("source refusal")
    };
    assert_eq!(gap.path, "return_value.value.value.elements[0]");
}
