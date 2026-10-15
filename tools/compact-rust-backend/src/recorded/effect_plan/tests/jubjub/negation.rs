// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Negation consumes one canonical point; coordinate pairs are not a substitute.
use super::*;

fn negate(value: Expr) -> Expr {
    Expr::EcNeg {
        value: Box::new(value),
    }
}
fn point_witness() -> Expr {
    Expr::WitnessCall {
        name: "next_point".into(),
        arguments: vec![],
    }
}
fn source() -> Contract {
    let mut source = contract();
    source.witnesses.push(WitnessDeclaration {
        source: None,
        name: "next_point".into(),
        parameters: vec![],
        result: Type::JubjubPoint,
    });
    source
}
fn input(operation: &mut Operation) -> &mut Value {
    match operation {
        Operation::EcNeg(value) => value,
        _ => panic!("expected negation"),
    }
}

#[test]
fn negation_evaluates_one_witness_before_both_coordinate_projections() {
    for y in [false, true] {
        let negated = negate(point_witness());
        let expression = if y {
            Expr::JubjubPointY {
                value: Box::new(negated),
            }
        } else {
            Expr::JubjubPointX {
                value: Box::new(negated),
            }
        };
        let checked = checked(&source(), &candidate(expression, Type::Field)).unwrap();
        let bindings = &checked.plan().body.bindings;
        assert_eq!(bindings.len(), 3);
        assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
        assert!(
            matches!(&bindings[1].operation, Operation::EcNeg(value) if value == &bindings[0].value)
        );
        assert_eq!(bindings[1].value.ty, Type::JubjubPoint);
        match &bindings[2].operation {
            Operation::JubjubPointX(value) | Operation::JubjubPointY(value) => {
                assert_eq!(value, &bindings[1].value)
            }
            _ => panic!("expected coordinate projection"),
        }
        let steps = render(&checked).unwrap().steps;
        let syntax = quote::quote!(#(#steps)*).to_string();
        assert_eq!(syntax.matches("next_point").count(), 1);
        assert_eq!(syntax.matches("runtime :: ec_neg (").count(), 1);
        assert!(syntax.find("next_point").unwrap() < syntax.find("ec_neg").unwrap());
    }
    let checked = checked(
        &source(),
        &candidate(negate(negate(point_witness())), Type::JubjubPoint),
    )
    .unwrap();
    assert_eq!(checked.plan().body.bindings.len(), 3);
    let steps = render(&checked).unwrap().steps;
    let syntax = quote::quote!(#(#steps)*).to_string();
    assert_eq!(syntax.matches("next_point").count(), 1);
    assert_eq!(syntax.matches("runtime :: ec_neg (").count(), 2);
}

#[test]
fn negation_rejects_nonpoint_operands_and_coordinate_lookalikes() {
    for value in [
        parameter("scalar"),
        Expr::BytesLiteral { bytes: vec![0; 32] },
        Expr::Unit,
    ] {
        let gap = refusal(negate(value), Type::JubjubPoint);
        assert_eq!(gap.path, "return_value.value.value");
        assert!(gap.detail.contains("expected JubjubPoint"), "{gap:?}");
    }
    let mut circuit = candidate(negate(parameter("point")), Type::JubjubPoint);
    circuit.parameters[0].ty = Type::Struct {
        name: "CoordinatePair".into(),
        fields: vec![
            crate::ir::StructField {
                name: "x".into(),
                ty: Type::Field,
            },
            crate::ir::StructField {
                name: "y".into(),
                ty: Type::Field,
            },
        ],
    };
    match checked(&contract(), &circuit).unwrap_err() {
        PlanError::Unsupported(gap) => {
            assert_eq!(gap.path, "return_value.value.value");
            assert!(gap.detail.contains("expected JubjubPoint"));
        }
        other => panic!("expected typed source refusal: {other:?}"),
    }
    assert!(
        checked(
            &contract(),
            &candidate(negate(parameter("point")), Type::Field)
        )
        .is_err()
    );
}

#[test]
fn negation_validator_checks_result_operand_annotations_and_live_ssa_independently() {
    let checked = checked(
        &contract(),
        &candidate(negate(parameter("point")), Type::JubjubPoint),
    )
    .unwrap();
    for case in 0..5 {
        let mut forged = checked.plan().clone();
        let binding = &mut forged.body.bindings[0];
        match case {
            0 => *input(&mut binding.operation) = forged.parameters[1].clone(),
            1 => binding.value.ty = Type::Field,
            2 => input(&mut binding.operation).id = usize::MAX,
            3 => input(&mut binding.operation).id = binding.value.id,
            _ => input(&mut binding.operation).ty = Type::Field,
        }
        let gap = validate::validate(&forged, checked.declarations()).unwrap_err();
        assert_eq!(gap.path, "return_value.value", "{case}: {gap:?}");
    }
}

#[test]
fn negation_stays_in_selected_branch_and_branch_local_points_cannot_escape() {
    let branch = Expr::If {
        condition: Box::new(Expr::Boolean { value: false }),
        then: Box::new(negate(point_witness())),
        otherwise: Box::new(parameter("point")),
    };
    let checked = checked(&source(), &candidate(negate(branch), Type::JubjubPoint)).unwrap();
    let mut forged = checked.plan().clone();
    let escaped = forged
        .body
        .bindings
        .iter()
        .find_map(|binding| match &binding.operation {
            Operation::If {
                then, otherwise, ..
            } => {
                assert!(matches!(
                    then.bindings[0].operation,
                    Operation::Witness { .. }
                ));
                assert!(matches!(then.bindings[1].operation, Operation::EcNeg(_)));
                assert!(otherwise.bindings.is_empty());
                Some(then.result.clone())
            }
            _ => None,
        })
        .unwrap();
    // The outer block contains no eager witness; only its chosen branch can call it.
    assert!(
        !forged
            .body
            .bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::Witness { .. }))
    );
    render(&checked).unwrap();
    *input(&mut forged.body.bindings.last_mut().unwrap().operation) = escaped;
    let gap = validate::validate(&forged, checked.declarations()).unwrap_err();
    assert!(gap.detail.contains("out of scope"), "{gap:?}");
}
