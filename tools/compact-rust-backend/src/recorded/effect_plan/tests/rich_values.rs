// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Candidate rich values: exact members, ordered effects and unchanged arithmetic.
use super::*;

fn bytes_type() -> Type {
    Type::Bytes { length: 32 }
}
fn bytes(value: u8) -> Expr {
    Expr::BytesLiteral {
        bytes: vec![value; 32],
    }
}
fn status() -> Type {
    Type::Enum {
        name: "Status".into(),
        variants: vec!["Ready".into(), "Done".into()],
    }
}
fn pair_type() -> Type {
    Type::Struct {
        name: "Pair".into(),
        fields: vec![
            StructField {
                name: "first".into(),
                ty: Type::Field,
            },
            StructField {
                name: "second".into(),
                ty: Type::Field,
            },
        ],
    }
}
fn record_type() -> Type {
    Type::Struct {
        name: "Record".into(),
        fields: vec![
            StructField {
                name: "digest".into(),
                ty: bytes_type(),
            },
            StructField {
                name: "status".into(),
                ty: status(),
            },
            StructField {
                name: "pair".into(),
                ty: pair_type(),
            },
        ],
    }
}
fn literal(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn project(value: Expr, field: &str, index: usize) -> Expr {
    Expr::StructField {
        value: Box::new(value),
        field: field.into(),
        index,
    }
}
fn candidate(value: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "rich_candidate".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value },
    }
}
fn checked(value: Expr, result: Type) -> CheckedPlan {
    check(&candidate(value, result), &HashMap::new(), &HashMap::new()).unwrap()
}
fn refusal(value: Expr, result: Type) -> RecordingGap {
    match check(&candidate(value, result), &HashMap::new(), &HashMap::new()).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        other => panic!("expected source refusal: {other:?}"),
    }
}
fn record() -> Expr {
    Expr::StructLiteral {
        ty: record_type(),
        fields: vec![
            bytes(7),
            Expr::EnumVariant {
                ty: status(),
                variant: "Done".into(),
            },
            Expr::StructLiteral {
                ty: pair_type(),
                fields: vec![literal("1"), literal("2")],
            },
        ],
    }
}

#[test]
fn nested_rich_construction_projection_default_and_identity_coerce_lower() {
    let expression = Expr::Let {
        bindings: vec![LocalBinding {
            name: "saved".into(),
            ty: record_type(),
            value: record(),
        }],
        body: Box::new(Expr::Coerce {
            value: Box::new(project(project(parameter("saved"), "pair", 2), "second", 1)),
            ty: Type::Field,
        }),
    };
    let plan = checked(expression, Type::Field);
    assert_eq!(
        plan.plan()
            .body
            .bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::StructConstruct { .. }))
            .count(),
        2
    );
    assert_eq!(
        plan.plan()
            .body
            .bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::StructProject { .. }))
            .count(),
        2
    );
    assert!(
        !plan
            .plan()
            .body
            .bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::Convert { .. }))
    );
    render(&plan).unwrap();
    let default = checked(Expr::Default { ty: record_type() }, record_type());
    assert!(matches!(
        default.plan().body.bindings[0].operation,
        Operation::Default
    ));
    render(&default).unwrap();
    let same_type = checked(
        Expr::Coerce {
            value: Box::new(record()),
            ty: record_type(),
        },
        record_type(),
    );
    assert!(
        !same_type
            .plan()
            .body
            .bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::Convert { .. }))
    );
}

#[test]
fn tuple_temporaries_preserve_unit_nested_products_and_exact_equality() {
    let tuple = || Expr::Tuple {
        elements: vec![
            Expr::Unit,
            Expr::Tuple {
                elements: vec![bytes(4), record()],
            },
        ],
    };
    let plan = checked(
        Expr::Equal {
            left: Box::new(tuple()),
            right: Box::new(tuple()),
        },
        Type::Boolean,
    );
    assert_eq!(
        plan.plan()
            .body
            .bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::Tuple { .. }))
            .count(),
        4
    );
    assert!(matches!(
        plan.plan().body.bindings.last().unwrap().operation,
        Operation::Equal(..)
    ));
    render(&plan).unwrap();
}

#[test]
fn constructor_and_tuple_operands_record_each_witness_once_left_to_right() {
    let witnesses: Vec<_> = ["first", "second", "third"]
        .into_iter()
        .map(|name| WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result: Type::Field,
        })
        .collect();
    let call = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    let pair = Expr::StructLiteral {
        ty: pair_type(),
        fields: vec![call("first"), call("second")],
    };
    let tuple = Expr::Tuple {
        elements: vec![pair, call("third")],
    };
    let root = candidate(
        Expr::Equal {
            left: Box::new(tuple.clone()),
            right: Box::new(tuple),
        },
        Type::Boolean,
    );
    let witness_map = witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let plan = check(&root, &HashMap::new(), &witness_map).unwrap();
    let actual: Vec<_> = plan
        .plan()
        .body
        .bindings
        .iter()
        .filter_map(|binding| match binding.operation {
            Operation::Witness { declaration, .. } => Some(
                plan.declarations()
                    .witness(declaration)
                    .unwrap()
                    .name
                    .as_str(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        actual,
        ["first", "second", "third", "first", "second", "third"]
    );
    render(&plan).unwrap();
}

#[test]
fn malformed_constructor_members_and_projections_keep_precise_source_paths() {
    let wrong_member = Expr::StructLiteral {
        ty: pair_type(),
        fields: vec![Expr::Boolean { value: true }, literal("1")],
    };
    let error = refusal(wrong_member, pair_type());
    assert_eq!(error.path, "return_value.value.fields[0]");
    assert!(error.detail.contains("member type"));
    let missing = Expr::StructLiteral {
        ty: pair_type(),
        fields: vec![literal("1")],
    };
    assert!(
        refusal(missing, pair_type())
            .detail
            .contains("member count")
    );
    for (name, index) in [("first", 1), ("missing", 0), ("first", 8)] {
        let value = Expr::Default { ty: pair_type() };
        let error = refusal(project(value, name, index), Type::Field);
        assert_eq!(error.path, "return_value.value");
        assert!(error.detail.contains("inconsistent index"));
    }
    assert!(
        refusal(project(literal("1"), "first", 0), Type::Field)
            .detail
            .contains("struct value")
    );
}

#[test]
fn invalid_enum_variant_and_nonidentity_rich_coercion_are_refused() {
    let value = Expr::EnumVariant {
        ty: status(),
        variant: "Missing".into(),
    };
    assert!(refusal(value, status()).detail.contains("enum variant"));
    let value = Expr::EnumVariant {
        ty: Type::Field,
        variant: "Ready".into(),
    };
    assert!(refusal(value, Type::Field).detail.contains("enum type"));
    let other = Type::Struct {
        name: "DifferentPair".into(),
        fields: match pair_type() {
            Type::Struct { fields, .. } => fields,
            _ => unreachable!(),
        },
    };
    let value = Expr::Coerce {
        value: Box::new(Expr::Default { ty: pair_type() }),
        ty: other.clone(),
    };
    assert!(refusal(value, other).detail.contains("cannot coerce"));
}

#[test]
fn rich_value_support_does_not_authorize_arithmetic_or_unrelated_nodes() {
    let expression = Expr::Add {
        left: Box::new(bytes(1)),
        right: Box::new(bytes(2)),
    };
    assert!(
        refusal(expression, Type::Field)
            .detail
            .contains("arithmetic operand")
    );
    let expression = Expr::FieldCast {
        value: Box::new(bytes(1)),
    };
    assert!(
        refusal(expression, Type::Field)
            .detail
            .contains("unsigned input")
    );
    let expression = Expr::EcNeg {
        value: Box::new(Expr::HashToCurve {
            value: Box::new(bytes(1)),
        }),
    };
    render(&checked(expression, Type::JubjubPoint)).unwrap();
    let constructed = Expr::ConstructJubjubPoint {
        x: Box::new(literal("0")),
        y: Box::new(literal("1")),
    };
    render(&checked(constructed, Type::JubjubPoint)).unwrap();
    let commitment = Expr::PersistentCommit {
        value: Box::new(literal("0")),
        opening: Box::new(bytes(0)),
    };
    render(&checked(commitment, bytes_type())).unwrap();
    let mapped_commitment = Expr::PersistentCommit {
        value: Box::new(crate::recorded::test_support::fixed_map_input()),
        opening: Box::new(bytes(0)),
    };
    render(&checked(mapped_commitment, bytes_type())).unwrap();
}

#[test]
fn first_bad_product_child_is_reported_before_later_siblings() {
    let value = Expr::StructLiteral {
        ty: pair_type(),
        fields: vec![parameter("missing_first"), parameter("missing_second")],
    };
    assert_eq!(
        refusal(value, pair_type()).path,
        "return_value.value.fields[0]"
    );
    let value = Expr::Equal {
        left: Box::new(Expr::Tuple {
            elements: vec![parameter("missing_first"), parameter("missing_second")],
        }),
        right: Box::new(Expr::Boolean { value: true }),
    };
    assert_eq!(
        refusal(value, Type::Boolean).path,
        "return_value.value.left.elements[0]"
    );
}

#[test]
fn fixed_bytes_are_length_typed_and_rich_witness_interfaces_are_exact() {
    let short_type = Type::Bytes { length: 3 };
    let short_value = Expr::BytesLiteral {
        bytes: vec![1, 2, 3],
    };
    let literal_plan = checked(short_value.clone(), short_type.clone());
    assert!(
        matches!(&literal_plan.plan().body.bindings[0].operation, Operation::Literal(Literal::Bytes(bytes)) if bytes == &[1, 2, 3])
    );
    render(&literal_plan).unwrap();
    let witness = WitnessDeclaration {
        source: None,
        name: "record_from_bytes".into(),
        parameters: vec![Parameter {
            name: "digest".into(),
            ty: short_type,
        }],
        result: record_type(),
    };
    let call = |value| Expr::WitnessCall {
        name: witness.name.clone(),
        arguments: vec![value],
    };
    let root = candidate(call(short_value), record_type());
    let declarations = HashMap::from([(witness.name.as_str(), &witness)]);
    let plan = check(&root, &HashMap::new(), &declarations).unwrap();
    assert_eq!(plan.plan().body.result.ty, record_type());
    assert!(matches!(
        plan.plan().body.bindings.last().unwrap().operation,
        Operation::Witness { .. }
    ));
    render(&plan).unwrap();
    let wrong_length = candidate(call(bytes(1)), record_type());
    let PlanError::Unsupported(error) =
        check(&wrong_length, &HashMap::new(), &declarations).unwrap_err()
    else {
        panic!("wrong witness byte length must be rejected by lowering")
    };
    assert_eq!(error.path, "return_value.value.arguments[0]");
    assert!(error.detail.contains("cannot coerce"));
}
