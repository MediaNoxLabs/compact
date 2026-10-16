// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Same-type comparison preserves Rust representation and evaluation boundaries.
use super::*;
#[path = "inequality/source.rs"]
mod source;
#[path = "inequality/validation.rs"]
mod validation;
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn different(left: Expr, right: Expr) -> Expr {
    Expr::NotEqual {
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn tuple(arity: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::OpaqueBytes; arity],
    }
}
fn product(ty: Type) -> Type {
    Type::Struct {
        name: "Product".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn vector(ty: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(ty),
        length,
    }
}
fn root(left: Type, right: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "compare".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "left".into(),
                ty: left,
            },
            Parameter {
                name: "right".into(),
                ty: right,
            },
        ],
        result: Type::Boolean,
        actions: vec![StateAction::Expression {
            value: different(reference("left"), reference("right")),
        }],
        return_value: StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn authority(source: &StatefulCircuit) -> declarations::Declarations {
    let types = type_environment::TypeEnvironment::from_sources(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default()
        .with_type_environment(types)
        .with_root(source)
}
fn comparison(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::NotEqual(..)))
        .unwrap()
}
fn syntax(plan: &CheckedPlan) -> String {
    let steps = render(plan).unwrap().steps;
    quote::quote!({ #(#steps)* }).to_string()
}
#[test]
fn inequality_accepts_existing_same_type_wrappers_and_owned_products() {
    for ty in [
        Type::Unit,
        Type::Boolean,
        Type::Field,
        validate::uint("18446744073709551615"),
        validate::uint("680564733841876926926749214863536422911"),
        Type::Bytes { length: 32 },
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::JubjubPoint,
        tuple(12),
        vector(tuple(12), 2),
        vector(Type::OpaqueString, 0),
        product(vector(Type::OpaqueBytes, 2)),
    ] {
        let source = root(ty.clone(), ty);
        let plan = checked(&source);
        validate::check(plan.plan().clone(), authority(&source)).unwrap();
        assert_eq!(syntax(&plan).matches("!=").count(), 1);
    }
}
#[test]
fn inequality_keeps_partial_eq_named_derive_and_canonical_bounds() {
    for ty in [
        tuple(13),
        vector(tuple(13), 1),
        product(tuple(9)),
        validate::uint("01"),
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662656",
        ),
    ] {
        assert!(matches!(
            check(&root(ty.clone(), ty), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
    }
    for (left, right) in [
        (Type::Field, Type::Boolean),
        (Type::OpaqueString, Type::OpaqueBytes),
        (validate::uint("255"), validate::uint("65535")),
    ] {
        assert!(matches!(
            check(&root(left, right), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
    }
}
#[test]
fn inequality_evaluates_callback_operands_once_left_then_right() {
    let ws: Vec<_> = ["left", "right"]
        .into_iter()
        .map(|name| WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result: Type::OpaqueBytes,
        })
        .collect();
    let witnesses = ws.iter().map(|w| (w.name.as_str(), w)).collect();
    let call = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    let mut source = root(Type::Unit, Type::Unit);
    source.parameters.clear();
    source.actions = vec![StateAction::Expression {
        value: different(call("left"), call("right")),
    }];
    let plan = check(&source, &HashMap::new(), &witnesses).unwrap();
    let bindings = &plan.plan().body.bindings;
    assert!(matches!(
        bindings[0].operation,
        Operation::Witness {
            declaration: WitnessId(0),
            ..
        }
    ));
    assert!(matches!(
        bindings[1].operation,
        Operation::Witness {
            declaration: WitnessId(1),
            ..
        }
    ));
    let Operation::NotEqual(left, right) = &bindings[2].operation else {
        panic!("comparison after both witnesses")
    };
    assert_eq!(left, &bindings[0].value);
    assert_eq!(right, &bindings[1].value);
    let text = syntax(&plan);
    assert_eq!(text.matches("try_witness_metered").count(), 2);
    assert!(text.rfind("try_witness_metered").unwrap() < text.find("!=").unwrap());
}
#[test]
fn owned_inputs_survive_repeated_comparisons_without_reexecuting_operands() {
    let mut source = root(Type::OpaqueBytes, Type::OpaqueBytes);
    source.actions.push(source.actions[0].clone());
    let plan = checked(&source);
    let pairs = plan
        .plan()
        .body
        .bindings
        .iter()
        .filter_map(|b| match &b.operation {
            Operation::NotEqual(a, b) => Some((a.id, b.id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(pairs, vec![(0, 1), (0, 1)]);
    assert_eq!(syntax(&plan).matches("!=").count(), 2);
}
