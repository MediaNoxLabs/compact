// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rust tuple PartialEq is bounded independently from tuple construction/Clone.
use super::*;
fn tuple(arity: usize) -> Expr {
    Expr::Tuple {
        elements: (0..arity)
            .map(|i| Expr::FieldLiteral {
                value: i.to_string(),
            })
            .collect(),
    }
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        result: Type::Boolean,
        actions: vec![],
        return_value: StateReturn::Expression { value },
    }
}
fn equality(value: Expr) -> StatefulCircuit {
    root(Expr::Equal {
        left: Box::new(value.clone()),
        right: Box::new(value),
    })
}
fn checked(source: &StatefulCircuit) -> Result<CheckedPlan, PlanError> {
    check(source, &HashMap::new(), &HashMap::new())
}
#[test]
fn tuple_equality_accepts_twelve_and_refuses_thirteen_including_nested_tuples() {
    checked(&equality(tuple(12))).unwrap();
    for value in [
        tuple(13),
        Expr::Tuple {
            elements: vec![tuple(13)],
        },
    ] {
        assert!(matches!(
            checked(&equality(value)),
            Err(PlanError::Unsupported(_))
        ));
    }
    checked(&equality(Expr::Tuple {
        elements: vec![tuple(12)],
    }))
    .unwrap();
}
#[test]
fn tuple_construction_above_equality_limit_remains_supported() {
    let source = root(Expr::Let {
        bindings: vec![LocalBinding {
            name: "temporary".into(),
            ty: Type::Tuple {
                elements: vec![Type::Field; 13],
            },
            value: tuple(13),
        }],
        body: Box::new(Expr::Boolean { value: true }),
    });
    let checked = checked(&source).unwrap();
    render(&checked).unwrap();
}
#[test]
fn forged_thirteen_member_equality_is_rejected_independently_of_lowering() {
    let checked = checked(&equality(tuple(12))).unwrap();
    let environment = || declarations::Declarations::new(&HashMap::new(), &HashMap::new());
    validate::check(checked.plan().clone(), environment()).unwrap();
    let mut forged = checked.plan().clone();
    let extend = |ty: &mut Type| {
        let Type::Tuple { elements } = ty else {
            panic!("expected tuple type")
        };
        assert_eq!(elements.len(), 12);
        elements.push(Type::Field);
    };
    for binding in &mut forged.body.bindings {
        match &mut binding.operation {
            Operation::Tuple { elements } => {
                elements.push(elements[0].clone());
                extend(&mut binding.value.ty);
            }
            Operation::Equal(left, right) => {
                extend(&mut left.ty);
                extend(&mut right.ty);
            }
            _ => {}
        }
    }
    let gap = validate::check(forged, environment()).unwrap_err();
    assert!(
        gap.detail.contains("operation input/output types"),
        "{gap:?}"
    );
}
