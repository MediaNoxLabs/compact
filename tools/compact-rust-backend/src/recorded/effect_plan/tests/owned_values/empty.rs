// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Empty named products retain canonical identity and source result authority.
use super::super::super::type_environment::TypeEnvironment;
use super::*;

pub(super) fn empty_type() -> Type {
    Type::Struct {
        name: "EmptyLocal".into(),
        fields: vec![],
    }
}

#[test]
fn empty_named_values_compose_through_construction_default_pure_calls_and_joins() {
    let ty = empty_type();
    assert!(validate::value_type(&ty));
    assert!(validate::pure_interface_type(&ty));
    assert!(!validate::surface_type(&ty));
    for initial in [
        Expr::Default { ty: ty.clone() },
        Expr::StructLiteral {
            ty: ty.clone(),
            fields: vec![],
        },
    ] {
        for selected in [false, true] {
            accept(
                local(
                    "empty",
                    ty.clone(),
                    initial.clone(),
                    eq(
                        Expr::If {
                            condition: Box::new(Expr::Boolean { value: selected }),
                            then: Box::new(call("identity", vec![param("empty")])),
                            otherwise: Box::new(param("empty")),
                        },
                        param("empty"),
                    ),
                ),
                vec![identity("identity", ty.clone())],
            );
        }
    }
}

fn environment() -> declarations::Declarations {
    let source = root(local(
        "empty",
        empty_type(),
        Expr::Default { ty: empty_type() },
        boolean(),
    ));
    let types = TypeEnvironment::from_sources(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default()
        .with_type_environment(types)
        .with_root(&source)
}
fn value(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(id: usize, ty: Type, operation: Operation) -> Binding {
    Binding {
        value: value(id, ty),
        operation,
        path: format!("empty[{id}]"),
    }
}

#[test]
fn empty_named_products_still_require_canonical_definitions_and_valid_operations() {
    let ty = empty_type();
    let original = Plan {
        parameters: vec![],
        result_type: Type::Boolean,
        body: Block {
            bindings: vec![
                binding(0, ty.clone(), Operation::Default),
                binding(1, ty.clone(), Operation::StructConstruct { fields: vec![] }),
                binding(
                    2,
                    Type::Boolean,
                    Operation::Equal(value(0, ty.clone()), value(1, ty.clone())),
                ),
            ],
            result: value(2, Type::Boolean),
        },
    };
    render(&validate::check(original.clone(), environment()).unwrap()).unwrap();
    assert!(validate::check(original.clone(), declarations::Declarations::default()).is_err());
    for mutation in 0..6 {
        let mut forged = original.clone();
        match mutation {
            0 => {
                let Type::Struct { fields, .. } = &mut forged.body.bindings[0].value.ty else {
                    unreachable!()
                };
                fields.push(StructField {
                    name: "invented".into(),
                    ty: Type::Field,
                });
            }
            1 => {
                let Type::Struct { name, .. } = &mut forged.body.bindings[0].value.ty else {
                    unreachable!()
                };
                *name = "UnknownEmpty".into();
            }
            2 => {
                forged.body.bindings[1].operation = Operation::StructConstruct {
                    fields: vec![value(0, ty.clone())],
                }
            }
            3 => {
                forged.body.bindings[1].operation = Operation::StructProject {
                    value: value(0, ty.clone()),
                    field: "invented".into(),
                    index: 0,
                }
            }
            4 => {
                forged.body.bindings[2].operation =
                    Operation::Equal(value(0, ty.clone()), value(99, ty.clone()))
            }
            5 => {
                forged.result_type = ty.clone();
                forged.body.result = value(0, ty.clone());
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(forged, environment()).is_err(),
            "forged {mutation}"
        );
    }
}
