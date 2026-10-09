// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::super::super::type_environment::TypeEnvironment;
use super::*;
fn value(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(id: usize, ty: Type, operation: Operation) -> Binding {
    Binding {
        value: value(id, ty),
        operation,
        path: format!("owned[{id}]"),
    }
}
fn holder(ty: Type) -> Type {
    Type::Struct {
        name: "Holder".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn environment(ty: &Type) -> declarations::Declarations {
    let mut source = root(boolean());
    source.parameters.push(Parameter {
        name: "types".into(),
        ty: ty.clone(),
    });
    let env = TypeEnvironment::from_sources(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default().with_type_environment(env)
}
fn convert(source: Type, target: Type) -> (Plan, Type) {
    let owner = holder(source.clone());
    (
        Plan {
            parameters: vec![],
            result_type: Type::Boolean,
            body: Block {
                bindings: vec![
                    binding(0, owner.clone(), Operation::Default),
                    binding(
                        1,
                        source.clone(),
                        Operation::StructProject {
                            value: value(0, owner.clone()),
                            field: "value".into(),
                            index: 0,
                        },
                    ),
                    binding(
                        2,
                        target,
                        Operation::Convert {
                            value: value(1, source),
                            kind: Conversion::Coerce,
                        },
                    ),
                    binding(3, Type::Boolean, Operation::Literal(Literal::Boolean(true))),
                ],
                result: value(3, Type::Boolean),
            },
        },
        owner,
    )
}
#[test]
fn forged_conversion_length_narrowing_reference_and_result_cannot_bypass_lowering() {
    let source = vector(Type::Unsigned { max: "255".into() }, 2);
    let target = vector(
        Type::Unsigned {
            max: "65535".into(),
        },
        2,
    );
    let (plan, owner) = convert(source.clone(), target.clone());
    validate::check(plan.clone(), environment(&owner)).unwrap();
    for mode in 0..5 {
        let mut bad = plan.clone();
        match mode {
            0 => {
                bad.body.bindings[2].value.ty = vector(
                    Type::Unsigned {
                        max: "65535".into(),
                    },
                    3,
                )
            }
            1 => bad.body.bindings[2].value.ty = vector(Type::Unsigned { max: "127".into() }, 2),
            2 => {
                let Operation::Convert { value, .. } = &mut bad.body.bindings[2].operation else {
                    unreachable!()
                };
                value.id = 99;
            }
            3 => {
                let Operation::Convert { value, .. } = &mut bad.body.bindings[2].operation else {
                    unreachable!()
                };
                value.ty = target.clone();
            }
            4 => {
                let Operation::StructProject { index, .. } = &mut bad.body.bindings[1].operation
                else {
                    unreachable!()
                };
                *index = 1;
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(bad, environment(&owner)).is_err(),
            "forged {mode}"
        );
    }
}
#[test]
fn nested_vector_cannot_replace_canonical_owned_member_definition() {
    let source = vector(carrier(), 2);
    let (plan, owner) = convert(source.clone(), source);
    validate::check(plan.clone(), environment(&owner)).unwrap();
    let mut bad = plan;
    let Type::Vector { element, .. } = &mut bad.body.bindings[1].value.ty else {
        unreachable!()
    };
    let Type::Struct { fields, .. } = element.as_mut() else {
        unreachable!()
    };
    fields.swap(0, 1);
    assert!(validate::check(bad, environment(&owner)).is_err());
}
#[test]
fn point_equality_is_refused_recursively_through_vectors_products_and_tuples() {
    for ty in [
        vector(Type::JubjubPoint, 2),
        Type::Struct {
            name: "NestedPoint".into(),
            fields: vec![StructField {
                name: "points".into(),
                ty: vector(Type::JubjubPoint, 1),
            }],
        },
        tuple(vec![vector(Type::JubjubPoint, 0)]),
    ] {
        assert!(validate::value_type(&ty));
        assert!(!validate::equality_type(&ty));
        let (mut plan, owner) = convert(ty.clone(), ty.clone());
        plan.body.bindings[2] = binding(
            2,
            Type::Boolean,
            Operation::Equal(value(1, ty.clone()), value(1, ty)),
        );
        assert!(validate::check(plan, environment(&owner)).is_err());
    }
}
#[test]
fn named_struct_tuple_representation_bound_is_distinct_from_local_tuple_values() {
    for length in [0, 1, 8, 9, 12, 13] {
        let tuple = tuple(vec![Type::Field; length]);
        assert!(validate::value_type(&tuple));
        assert_eq!(
            validate::value_type(&holder(tuple.clone())),
            length <= 8,
            "named tuple length{length}"
        );
        assert_eq!(
            validate::value_type(&holder(vector(tuple, 2))),
            length <= 8,
            "nested named tuple length{length}"
        );
    }
}
#[test]
fn vector_constructor_independently_checks_length_element_and_live_references() {
    let original = Plan {
        parameters: vec![value(0, Type::Field)],
        result_type: Type::Boolean,
        body: Block {
            bindings: vec![
                binding(
                    1,
                    vector(Type::Field, 1),
                    Operation::Vector {
                        elements: vec![value(0, Type::Field)],
                    },
                ),
                binding(2, Type::Boolean, Operation::Literal(Literal::Boolean(true))),
            ],
            result: value(2, Type::Boolean),
        },
    };
    validate::check(original.clone(), declarations::Declarations::default()).unwrap();
    for mode in 0..4 {
        let mut plan = original.clone();
        match mode {
            0 => plan.body.bindings[0].value.ty = vector(Type::Field, 2),
            1 => {
                plan.parameters[0].ty = Type::Boolean;
                let Operation::Vector { elements } = &mut plan.body.bindings[0].operation else {
                    unreachable!()
                };
                elements[0].ty = Type::Boolean;
            }
            2 => {
                let Operation::Vector { elements } = &mut plan.body.bindings[0].operation else {
                    unreachable!()
                };
                elements[0].id = 9;
            }
            3 => plan.body.bindings[0].value.ty = tuple(vec![Type::Field]),
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, declarations::Declarations::default()).is_err(),
            "vector forged{mode}"
        );
    }
}
