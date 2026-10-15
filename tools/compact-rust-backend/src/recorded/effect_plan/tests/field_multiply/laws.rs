// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Checked Field multiplication laws; native arithmetic already has its domain tests.
use super::*;

fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn multiply(left: Expr, right: Expr) -> Expr {
    Expr::Multiply {
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn root(left: Type, right: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "multiply".into(),
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
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: multiply(reference("left"), reference("right")),
        },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn binding(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|v| matches!(v.operation, Operation::Multiply(..)))
        .unwrap()
}
fn operand(binding: &mut Binding, side: usize) -> &mut Value {
    let Operation::Multiply(left, right) = &mut binding.operation else {
        panic!("multiply")
    };
    if side == 0 { left } else { right }
}
fn other_types() -> Vec<Type> {
    vec![
        Type::Unit,
        Type::Boolean,
        Type::JubjubPoint,
        Type::Bytes { length: 32 },
        Type::OpaqueString,
        validate::uint("255"),
        validate::uint("340282366920938463463374607431768211456"),
    ]
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
#[test]
fn multiplication_admits_exact_fields_and_rejects_other_domains_on_either_side() {
    let source = root(Type::Field, Type::Field);
    let checked = checked(&source);
    assert_eq!(checked.plan().body.bindings.len(), 1);
    assert_eq!(checked.plan().body.bindings[0].value.ty, Type::Field);
    render(&checked).unwrap();
    for ty in other_types() {
        for (left, right) in [(ty.clone(), Type::Field), (Type::Field, ty.clone())] {
            assert!(
                matches!(
                    check(&root(left, right), &HashMap::new(), &HashMap::new()),
                    Err(PlanError::Unsupported(_))
                ),
                "{ty:?}"
            );
        }
    }
}
#[test]
fn multiply_checker_revalidates_operand_authority_and_output_type() {
    let checked = checked(&root(Type::Field, Type::Field));
    for side in [0, 1] {
        for mode in ["absent", "self", "annotation"] {
            let mut forged = checked.plan().clone();
            let binding = binding(&mut forged);
            let self_id = binding.value.id;
            let input = operand(binding, side);
            match mode {
                "absent" => input.id = usize::MAX,
                "self" => input.id = self_id,
                "annotation" => input.ty = Type::Boolean,
                _ => unreachable!(),
            }
            assert_eq!(
                validate::validate(&forged, checked.declarations())
                    .unwrap_err()
                    .path,
                "return_value.value"
            );
        }
    }
    for ty in [Type::Boolean, Type::Unit, validate::uint("255")] {
        let mut forged = checked.plan().clone();
        binding(&mut forged).value.ty = ty;
        let error = validate::validate(&forged, checked.declarations()).unwrap_err();
        assert_eq!(error.path, "return_value.value");
        assert!(error.detail.contains("operation input/output types"));
    }
    let mut forged = checked.plan().clone();
    forged.result_type = Type::Boolean;
    assert!(validate::validate(&forged, checked.declarations()).is_err());
}
#[test]
fn matching_use_annotations_do_not_bypass_the_field_domain_or_original_signature() {
    let source = root(Type::Field, Type::Field);
    let checked = checked(&source);
    for side in [0, 1] {
        for ty in other_types() {
            let mut substitute = source.clone();
            substitute.parameters[side].ty = ty.clone();
            let mut forged = checked.plan().clone();
            forged.parameters[side].ty = ty.clone();
            operand(binding(&mut forged), side).ty = ty;
            assert!(
                validate::validate(&forged, checked.declarations())
                    .unwrap_err()
                    .detail
                    .contains("declared signature")
            );
            let error = validate::check(forged, authority(&substitute)).unwrap_err();
            assert_eq!(error.path, "return_value.value");
            assert!(error.detail.contains("operation input/output types"));
        }
    }
}
#[test]
fn a_field_multiply_cannot_capture_a_value_from_a_finished_branch() {
    let mut source = root(Type::Field, Type::Field);
    source.return_value = StateReturn::Expression {
        value: multiply(
            Expr::If {
                condition: Box::new(Expr::Boolean { value: true }),
                then: Box::new(Expr::FieldLiteral { value: "1".into() }),
                otherwise: Box::new(Expr::FieldLiteral { value: "0".into() }),
            },
            reference("right"),
        ),
    };
    let checked = checked(&source);
    let mut forged = checked.plan().clone();
    let local = forged
        .body
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If { then, .. } => Some(then.result.clone()),
            _ => None,
        })
        .unwrap();
    *operand(binding(&mut forged), 0) = local;
    let error = validate::validate(&forged, checked.declarations()).unwrap_err();
    assert_eq!(error.path, "return_value.value");
    assert!(error.detail.contains("out of scope"));
}
