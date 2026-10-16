// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Remaining native LocalUnit values, with independent operation and trait laws.
use super::*;
#[path = "unit_value_operations/defaults.rs"]
mod defaults;

#[derive(Clone, Copy, Debug)]
enum Binary {
    Subtract,
    ConstructPoint,
}
impl Binary {
    fn expression(self, left: Expr, right: Expr) -> Expr {
        match self {
            Self::Subtract => Expr::Subtract {
                left: Box::new(left),
                right: Box::new(right),
            },
            Self::ConstructPoint => Expr::ConstructJubjubPoint {
                x: Box::new(left),
                y: Box::new(right),
            },
        }
    }
    fn result(self) -> Type {
        match self {
            Self::Subtract => Type::Field,
            Self::ConstructPoint => Type::JubjubPoint,
        }
    }
    fn operands(operation: &mut Operation) -> (&mut Value, &mut Value) {
        match operation {
            Operation::Subtract(left, right) => (left, right),
            Operation::ConstructJubjubPoint { x, y } => (x, y),
            _ => panic!("expected tested binary operation"),
        }
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn source(value: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "local_values".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "left".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "right".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "other".into(),
                ty: Type::Boolean,
            },
        ],
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn gap(source: &StatefulCircuit) -> RecordingGap {
    match check(source, &HashMap::new(), &HashMap::new()).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        other => panic!("expected source refusal: {other:?}"),
    }
}

#[test]
fn subtract_and_point_constructor_keep_exact_field_operands_and_distinct_results() {
    for operation in [Binary::Subtract, Binary::ConstructPoint] {
        let root = source(
            operation.expression(parameter("left"), parameter("right")),
            operation.result(),
        );
        let plan = checked(&root);
        assert_eq!(plan.plan().body.bindings.len(), 1);
        assert_eq!(plan.plan().body.bindings[0].value.ty, operation.result());
        let steps = render(&plan).unwrap().steps;
        let syntax = quote::quote!(#(#steps)*).to_string();
        match operation {
            Binary::Subtract => assert!(syntax.contains(" - "), "{syntax}"),
            Binary::ConstructPoint => {
                assert_eq!(
                    syntax.matches("runtime :: construct_jubjub_point").count(),
                    1
                );
                assert!(
                    syntax.contains(") ?"),
                    "coordinate validation stays fallible: {syntax}"
                );
            }
        }
        for ty in [
            Type::Boolean,
            Type::Unit,
            Type::JubjubPoint,
            Type::Bytes { length: 32 },
            validate::uint("255"),
        ] {
            for side in [0, 1] {
                let mut wrong = root.clone();
                wrong.parameters[side].ty = ty.clone();
                gap(&wrong);
            }
        }
        let mut wrong = root;
        wrong.result = Type::Boolean;
        gap(&wrong);
    }
}

#[test]
fn binary_values_revalidate_each_reference_annotation_result_and_self_use() {
    for operation in [Binary::Subtract, Binary::ConstructPoint] {
        let checked = checked(&source(
            operation.expression(parameter("left"), parameter("right")),
            operation.result(),
        ));
        for side in [0, 1] {
            for mode in 0..4 {
                let mut forged = checked.plan().clone();
                let wrong_kind = forged.parameters[2].clone();
                let binding = &mut forged.body.bindings[0];
                let self_id = binding.value.id;
                let (left, right) = Binary::operands(&mut binding.operation);
                let operand = if side == 0 { left } else { right };
                match mode {
                    0 => operand.id = usize::MAX,
                    1 => operand.id = self_id,
                    2 => operand.ty = Type::Boolean,
                    _ => *operand = wrong_kind,
                }
                assert_eq!(
                    validate::validate(&forged, checked.declarations())
                        .unwrap_err()
                        .path,
                    "return_value.value"
                );
            }
        }
        let mut forged = checked.plan().clone();
        forged.body.bindings[0].value.ty = Type::Boolean;
        assert!(validate::validate(&forged, checked.declarations()).is_err());
    }
}

#[test]
fn new_binary_values_preserve_once_only_witness_order_and_lazy_placement() {
    let declarations: Vec<_> = ["first", "second"]
        .into_iter()
        .map(|name| WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result: Type::Field,
        })
        .collect();
    let witnesses = declarations.iter().map(|w| (w.name.as_str(), w)).collect();
    let witness = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    for operation in [Binary::Subtract, Binary::ConstructPoint] {
        let root = source(
            Expr::If {
                condition: Box::new(Expr::Boolean { value: false }),
                then: Box::new(operation.expression(witness("first"), witness("second"))),
                otherwise: Box::new(operation.expression(field("0"), field("1"))),
            },
            operation.result(),
        );
        let checked = check(&root, &HashMap::new(), &witnesses).unwrap();
        let branch = checked.plan().body.bindings.last().unwrap();
        let Operation::If {
            then, otherwise, ..
        } = &branch.operation
        else {
            panic!("branch")
        };
        assert_eq!(then.bindings.len(), 3);
        for (index, name) in ["first", "second"].iter().enumerate() {
            let Operation::Witness { declaration, .. } = &then.bindings[index].operation else {
                panic!("witness")
            };
            assert_eq!(
                checked.declarations().witness(*declaration).unwrap().name,
                *name
            );
        }
        assert!(
            !otherwise
                .bindings
                .iter()
                .any(|b| matches!(b.operation, Operation::Witness { .. }))
        );
        assert!(
            !checked
                .plan()
                .body
                .bindings
                .iter()
                .any(|b| matches!(b.operation, Operation::Witness { .. }))
        );
        let steps = render(&checked).unwrap().steps;
        let syntax = quote::quote!(#(#steps)*).to_string();
        assert_eq!(syntax.matches("first").count(), 1);
        assert_eq!(syntax.matches("second").count(), 1);
        assert!(syntax.find("first").unwrap() < syntax.find("second").unwrap());
    }
}

#[test]
fn field_subtraction_keeps_checked_arithmetic_diagnostics_while_coordinates_check_left_first() {
    let invalid = Expr::Boolean { value: false };
    let missing = Expr::Call {
        name: "missing".into(),
        arguments: vec![],
    };
    // Existing checked arithmetic visits both children before its signature test.
    // Preserve that policy; native-invalid programs do not become execution claims.
    let subtraction = gap(&source(
        Binary::Subtract.expression(invalid.clone(), missing.clone()),
        Type::Field,
    ));
    assert_eq!(subtraction.path, "return_value.value.right");
    assert_eq!(subtraction.ir_node, "Expr::Call");
    // Jubjub's existing operand helper validates X before visiting Y, as native does.
    let point = gap(&source(
        Binary::ConstructPoint.expression(invalid, missing),
        Type::JubjubPoint,
    ));
    assert_eq!(point.path, "return_value.value.x");
    assert!(point.detail.contains("expected Field"));
}
