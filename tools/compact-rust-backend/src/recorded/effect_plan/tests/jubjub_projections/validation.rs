// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Projection laws reuse immutable root authority and the existing SSA checker.
use super::*;
#[path = "lowering.rs"]
mod lowering;

fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn project(value: Expr, y: bool) -> Expr {
    if y {
        Expr::JubjubPointY {
            value: Box::new(value),
        }
    } else {
        Expr::JubjubPointX {
            value: Box::new(value),
        }
    }
}
fn root(body: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "project".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "point".into(),
                ty: Type::JubjubPoint,
            },
            Parameter {
                name: "field".into(),
                ty: Type::Field,
            },
        ],
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression { value: body },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn input(operation: &mut Operation) -> &mut Value {
    match operation {
        Operation::JubjubPointX(value) | Operation::JubjubPointY(value) => value,
        _ => panic!("expected projection"),
    }
}
#[test]
fn projection_validator_rejects_wrong_kinds_results_scope_and_annotations() {
    for y in [false, true] {
        let checked = checked(&root(project(reference("point"), y)));
        let original = checked.plan();
        assert_eq!(original.body.bindings.len(), 1);
        assert_eq!(original.body.bindings[0].value.ty, Type::Field);
        validate::validate(original, checked.declarations()).unwrap();
        for case in 0..5 {
            let mut forged = original.clone();
            let binding = &mut forged.body.bindings[0];
            match case {
                0 => *input(&mut binding.operation) = original.parameters[1].clone(),
                1 => binding.value.ty = Type::Boolean,
                2 => input(&mut binding.operation).id = usize::MAX,
                3 => input(&mut binding.operation).id = binding.value.id,
                _ => input(&mut binding.operation).ty = Type::Field,
            }
            let error = validate::validate(&forged, checked.declarations()).unwrap_err();
            assert_eq!(error.path, "return_value.value", "{case}: {error:?}");
        }
        let mut root_forgery = original.clone();
        root_forgery.parameters[0].ty = Type::Field;
        assert!(validate::validate(&root_forgery, checked.declarations()).is_err());
    }
}
#[test]
fn branch_local_points_cannot_escape_their_checked_scope() {
    let generator = || Expr::EcMulGenerator {
        scalar: Box::new(Expr::FieldLiteral { value: "1".into() }),
    };
    let point = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(generator()),
        otherwise: Box::new(generator()),
    };
    for y in [false, true] {
        let checked = checked(&root(project(point.clone(), y)));
        let mut forged = checked.plan().clone();
        let local = forged
            .body
            .bindings
            .iter()
            .find_map(|binding| match &binding.operation {
                Operation::If { then, .. } => Some(then.result.clone()),
                _ => None,
            })
            .unwrap();
        let projection = forged.body.bindings.last_mut().unwrap();
        *input(&mut projection.operation) = local;
        let error = validate::validate(&forged, checked.declarations()).unwrap_err();
        assert!(error.detail.contains("out of scope"), "{error:?}");
    }
}
