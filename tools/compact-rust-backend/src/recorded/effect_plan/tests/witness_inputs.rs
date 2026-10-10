// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Callback input ownership is independent of public ABI and witness outputs.
use super::*;
#[path = "witness_inputs/inclusion.rs"]
mod inclusion;
#[path = "witness_inputs/validation.rs"]
mod validation;

fn text() -> Expr {
    Expr::Parameter {
        name: "text".into(),
    }
}
fn structure(member: Type) -> Type {
    Type::Struct {
        name: "Argument".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: member,
        }],
    }
}
fn tuple(count: usize) -> (Type, Expr) {
    (
        Type::Tuple {
            elements: vec![Type::OpaqueString; count],
        },
        Expr::Tuple {
            elements: vec![text(); count],
        },
    )
}
fn consumer(ty: Type) -> WitnessDeclaration {
    WitnessDeclaration {
        source: None,
        name: "consume".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty,
        }],
        result: Type::Boolean,
    }
}
fn root(argument: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "text".into(),
            ty: Type::OpaqueString,
        }],
        actions: vec![],
        result: Type::Boolean,
        return_value: StateReturn::Expression {
            value: Expr::WitnessCall {
                name: "consume".into(),
                arguments: vec![argument],
            },
        },
    }
}
fn checked(root: &StatefulCircuit, witness: &WitnessDeclaration) -> Result<CheckedPlan, PlanError> {
    check(
        root,
        &HashMap::new(),
        &HashMap::from([(witness.name.as_str(), witness)]),
    )
}
fn authority(root: &StatefulCircuit, witness: &WitnessDeclaration) -> declarations::Declarations {
    let witnesses = HashMap::from([(witness.name.as_str(), witness)]);
    let types = type_environment::TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &witnesses,
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&HashMap::new(), &witnesses)
        .with_type_environment(types)
        .with_root(root)
}
fn operation(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::Witness { .. }))
        .unwrap()
}

#[test]
fn owned_callback_inputs_include_standalone_tuple9_without_changing_other_domains() {
    let vector = Type::Vector {
        element: Box::new(Type::OpaqueString),
        length: 2,
    };
    let product = structure(vector.clone());
    let vector_value = Expr::Coerce {
        ty: vector.clone(),
        value: Box::new(tuple(2).1),
    };
    let bytes = structure(Type::OpaqueBytes);
    let cases = [
        (Type::OpaqueString, text()),
        (
            Type::OpaqueBytes,
            Expr::StructField {
                value: Box::new(Expr::Default { ty: bytes }),
                field: "value".into(),
                index: 0,
            },
        ),
        tuple(2),
        tuple(9),
        (vector.clone(), vector_value.clone()),
        (
            product.clone(),
            Expr::StructLiteral {
                ty: product,
                fields: vec![vector_value],
            },
        ),
    ];
    for (ty, argument) in cases {
        assert!(validate::witness_parameter_type(&ty), "{ty:?}");
        let root = root(argument);
        let witness = consumer(ty);
        let plan = checked(&root, &witness).unwrap();
        validate::check(plan.plan().clone(), authority(&root, &witness)).unwrap();
        render(&plan).unwrap();
    }
    assert!(!validate::public_interface_type(&tuple(9).0));
    assert!(!validate::witness_result_type(&tuple(9).0));
    assert!(!validate::storage_value_type(&tuple(9).0));
    assert!(!validate::witness_result_type(&Type::OpaqueString));
    // Named structs still acquire ledger representation derives with tuple8 bound.
    assert!(validate::witness_parameter_type(&structure(tuple(8).0)));
    assert!(!validate::witness_parameter_type(&structure(tuple(9).0)));
    let bad = structure(tuple(9).0);
    assert!(matches!(
        checked(&root(Expr::Default { ty: bad.clone() }), &consumer(bad)),
        Err(PlanError::Unsupported(_))
    ));
}

#[test]
fn witness_operand_expressions_remain_ordered_and_retained_once() {
    let producer = WitnessDeclaration {
        source: None,
        name: "produce".into(),
        parameters: vec![],
        result: Type::OpaqueBytes,
    };
    let mut consumer = consumer(Type::OpaqueBytes);
    consumer.parameters.push(Parameter {
        name: "second".into(),
        ty: Type::OpaqueBytes,
    });
    let value = || Expr::WitnessCall {
        name: "produce".into(),
        arguments: vec![],
    };
    let mut root = root(text());
    root.return_value = StateReturn::Expression {
        value: Expr::WitnessCall {
            name: "consume".into(),
            arguments: vec![value(), value()],
        },
    };
    let plan = check(
        &root,
        &HashMap::new(),
        &HashMap::from([("produce", &producer), ("consume", &consumer)]),
    )
    .unwrap();
    let calls = plan
        .plan()
        .body
        .bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::Witness { .. }))
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 3);
    let Operation::Witness { arguments, .. } = &calls[2].operation else {
        unreachable!()
    };
    assert_eq!(arguments, &[calls[0].value.clone(), calls[1].value.clone()]);
    assert!(calls[0].path.ends_with("arguments[0]"));
    assert!(calls[1].path.ends_with("arguments[1]"));
    render(&plan).unwrap();
}
