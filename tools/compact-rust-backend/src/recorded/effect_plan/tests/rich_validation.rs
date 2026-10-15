// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Forged rich operations bypass lowering and cannot invent named definitions.
use super::super::type_environment::TypeEnvironment;
use super::*;

fn status() -> Type {
    Type::Enum {
        name: "Status".into(),
        variants: vec!["Ready".into(), "Done".into()],
    }
}
fn record() -> Type {
    Type::Struct {
        name: "Record".into(),
        fields: vec![
            StructField {
                name: "amount".into(),
                ty: Type::Field,
            },
            StructField {
                name: "status".into(),
                ty: status(),
            },
            StructField {
                name: "code".into(),
                ty: Type::Bytes { length: 3 },
            },
        ],
    }
}
fn environment() -> declarations::Declarations {
    let source = StatefulCircuit {
        source: None,
        name: "authoritative".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: record(),
        return_value: StateReturn::Expression {
            value: Expr::Default { ty: record() },
        },
    };
    let types = TypeEnvironment::from_sources(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&HashMap::new(), &HashMap::new()).with_type_environment(types)
}
fn value(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(id: usize, ty: Type, operation: Operation) -> Binding {
    Binding {
        value: value(id, ty),
        operation,
        path: format!("forged[{id}]"),
    }
}
fn candidate(parameters: Vec<Value>, bindings: Vec<Binding>, result: Value) -> Plan {
    Plan {
        result_type: result.ty.clone(),
        parameters,
        body: Block { bindings, result },
    }
}
fn one(ty: Type, operation: Operation) -> Plan {
    candidate(
        vec![],
        vec![binding(0, ty.clone(), operation)],
        value(0, ty),
    )
}
fn accept(plan: Plan) -> CheckedPlan {
    validate::check(plan, environment()).unwrap()
}
fn reject(plan: Plan) -> RecordingGap {
    validate::check(plan, environment()).unwrap_err()
}

#[test]
fn byte_literal_length_and_enum_variant_are_independently_checked() {
    accept(one(
        Type::Bytes { length: 3 },
        Operation::Literal(Literal::Bytes(vec![1, 2, 3])),
    ));
    for ty in [Type::Bytes { length: 4 }, Type::Field] {
        assert_eq!(
            reject(one(ty, Operation::Literal(Literal::Bytes(vec![1, 2, 3])))).path,
            "forged[0]"
        );
    }
    accept(one(
        status(),
        Operation::EnumVariant {
            variant: "Ready".into(),
        },
    ));
    for (ty, variant) in [
        (status(), "Missing"),
        (status(), ""),
        (Type::Field, "Ready"),
    ] {
        assert_eq!(
            reject(one(
                ty,
                Operation::EnumVariant {
                    variant: variant.into()
                }
            ))
            .path,
            "forged[0]"
        );
    }
}

#[test]
fn plan_cannot_supply_unknown_or_altered_canonical_named_definitions() {
    accept(one(record(), Operation::Default));
    let mut renamed = record();
    let Type::Struct { name, .. } = &mut renamed else {
        unreachable!()
    };
    *name = "NotDeclared".into();
    reject(one(renamed, Operation::Default));
    let mut changed = record();
    let Type::Struct { fields, .. } = &mut changed else {
        unreachable!()
    };
    fields[0].ty = Type::Boolean;
    reject(one(changed, Operation::Default));
    let mut changed = status();
    let Type::Enum { variants, .. } = &mut changed else {
        unreachable!()
    };
    variants.push("Invented".into());
    reject(one(
        changed,
        Operation::EnumVariant {
            variant: "Invented".into(),
        },
    ));
    // Source declarations remain necessary even if an operation is locally well typed.
    let plan = one(record(), Operation::Default);
    assert!(
        validate::check(
            plan,
            declarations::Declarations::new(&HashMap::new(), &HashMap::new())
        )
        .is_err()
    );
}

fn constructor() -> Plan {
    let amount = value(0, Type::Field);
    let tag = value(1, status());
    let bytes = value(2, Type::Bytes { length: 3 });
    candidate(
        vec![],
        vec![
            binding(
                0,
                amount.ty.clone(),
                Operation::Literal(Literal::Field([0; 32])),
            ),
            binding(
                1,
                tag.ty.clone(),
                Operation::EnumVariant {
                    variant: "Done".into(),
                },
            ),
            binding(
                2,
                bytes.ty.clone(),
                Operation::Literal(Literal::Bytes(vec![1, 2, 3])),
            ),
            binding(
                3,
                record(),
                Operation::StructConstruct {
                    fields: vec![amount, tag, bytes],
                },
            ),
        ],
        value(3, record()),
    )
}

#[test]
fn constructor_members_require_exact_count_order_type_and_live_ids() {
    accept(constructor());
    for mutation in 0..5 {
        let mut plan = constructor();
        let Operation::StructConstruct { fields } = &mut plan.body.bindings[3].operation else {
            unreachable!()
        };
        match mutation {
            0 => {
                fields.pop();
            }
            1 => fields.swap(0, 1),
            2 => fields[0].ty = Type::Boolean,
            3 => fields[0].id = 99,
            4 => fields.push(fields[0].clone()),
            _ => unreachable!(),
        }
        assert_eq!(reject(plan).path, "forged[3]", "mutation {mutation}");
    }
    let mut plan = constructor();
    let output = value(3, Type::Field);
    plan.body.bindings[3].value = output.clone();
    plan.body.result = output;
    plan.result_type = Type::Field;
    assert_eq!(reject(plan).path, "forged[3]");
}

fn projection(input: Value, field: &str, index: usize, result: Type) -> Plan {
    candidate(
        vec![input.clone()],
        vec![binding(
            1,
            result.clone(),
            Operation::StructProject {
                value: input,
                field: field.into(),
                index,
            },
        )],
        value(1, result),
    )
}

#[test]
fn projection_checks_source_kind_member_name_index_and_exact_result() {
    let input = value(0, record());
    accept(projection(input.clone(), "amount", 0, Type::Field));
    for (field, index, result) in [
        ("status", 0, Type::Field),
        ("amount", 1, Type::Field),
        ("missing", 0, Type::Field),
        ("amount", 20, Type::Field),
        ("amount", 0, Type::Boolean),
    ] {
        assert_eq!(
            reject(projection(input.clone(), field, index, result)).path,
            "forged[1]"
        );
    }
    assert_eq!(
        reject(projection(value(0, Type::Field), "amount", 0, Type::Field)).path,
        "forged[1]"
    );
    let mut missing = projection(input, "amount", 0, Type::Field);
    missing.parameters.clear();
    assert_eq!(reject(missing).path, "forged[1]");
}

#[test]
fn tuple_types_include_unit_but_cannot_forge_element_annotations_or_order() {
    let unit = value(0, Type::Unit);
    let record = value(1, record());
    let tuple_type = Type::Tuple {
        elements: vec![unit.ty.clone(), record.ty.clone()],
    };
    let tuple = value(2, tuple_type.clone());
    let good = candidate(
        vec![unit.clone(), record.clone()],
        vec![
            binding(
                2,
                tuple_type,
                Operation::Tuple {
                    elements: vec![unit, record],
                },
            ),
            binding(3, Type::Boolean, Operation::Equal(tuple.clone(), tuple)),
        ],
        value(3, Type::Boolean),
    );
    accept(good.clone());
    let mut bad = good.clone();
    let Operation::Tuple { elements } = &mut bad.body.bindings[0].operation else {
        unreachable!()
    };
    elements.swap(0, 1);
    assert_eq!(reject(bad).path, "forged[2]");
    let mut bad = good;
    let Operation::Tuple { elements } = &mut bad.body.bindings[0].operation else {
        unreachable!()
    };
    elements[0].id = 80;
    assert_eq!(reject(bad).path, "forged[2]");
}

#[test]
fn noncopy_product_reuse_retains_each_consumed_tuple_operand() {
    let input = value(0, record());
    assert!(!crate::copy_type(&input.ty));
    let ty = Type::Tuple {
        elements: vec![input.ty.clone(), input.ty.clone()],
    };
    let tuple = value(1, ty.clone());
    let plan = accept(candidate(
        vec![input.clone()],
        vec![
            binding(
                1,
                ty,
                Operation::Tuple {
                    elements: vec![input.clone(), input],
                },
            ),
            binding(2, Type::Boolean, Operation::Equal(tuple.clone(), tuple)),
        ],
        value(2, Type::Boolean),
    ));
    let rendered = render(&plan).unwrap();
    let syn::Stmt::Local(local) = &rendered.steps[1] else {
        panic!("tuple binding")
    };
    let syn::Expr::Tuple(tuple) = &*local.init.as_ref().unwrap().expr else {
        panic!("tuple expression")
    };
    assert_eq!(tuple.elems.len(), 2);
    for element in &tuple.elems {
        let syn::Expr::MethodCall(call) = element else {
            panic!("owned operand must be retained")
        };
        assert_eq!(call.method, "clone");
        let syn::Expr::Paren(receiver) = &*call.receiver else {
            panic!("retained operand")
        };
        let syn::Expr::Path(path) = &*receiver.expr else {
            panic!("SSA operand")
        };
        assert!(path.path.is_ident("__compact_effect_0"));
    }
}

#[test]
fn point_tuple_equality_checks_operand_identity_and_result_type() {
    let point = value(0, Type::JubjubPoint);
    let ty = Type::Tuple {
        elements: vec![Type::JubjubPoint],
    };
    let tuple = value(1, ty.clone());
    let plan = candidate(
        vec![point.clone()],
        vec![
            binding(
                1,
                ty,
                Operation::Tuple {
                    elements: vec![point],
                },
            ),
            binding(2, Type::Boolean, Operation::Equal(tuple.clone(), tuple)),
        ],
        value(2, Type::Boolean),
    );
    render(&accept(plan.clone())).unwrap();
    for mutation in 0..3 {
        let mut forged = plan.clone();
        match mutation {
            0 => {
                let Operation::Equal(_, right) = &mut forged.body.bindings[1].operation else {
                    unreachable!()
                };
                *right = value(0, Type::JubjubPoint);
            }
            1 => {
                let Operation::Equal(_, right) = &mut forged.body.bindings[1].operation else {
                    unreachable!()
                };
                right.id = 99;
            }
            2 => forged.body.bindings[1].value.ty = Type::Field,
            _ => unreachable!(),
        }
        assert_eq!(reject(forged).path, "forged[2]");
    }
    let hidden_point = Type::Struct {
        name: "PointHolder".into(),
        fields: vec![StructField {
            name: "point".into(),
            ty: Type::JubjubPoint,
        }],
    };
    assert!(validate::equality_type(&hidden_point));
}

#[test]
fn forged_reference_and_block_result_types_are_bounded_before_equality() {
    let oversized = (0..64).fold(Type::Field, |ty, _| Type::Tuple { elements: vec![ty] });
    let parameter = value(0, Type::Field);
    let plan = candidate(
        vec![parameter.clone()],
        vec![binding(
            1,
            Type::Boolean,
            Operation::Equal(value(0, oversized.clone()), parameter.clone()),
        )],
        value(1, Type::Boolean),
    );
    let error = reject(plan);
    assert_eq!(error.path, "forged[1]");
    assert!(error.detail.contains("resource"), "{error:?}");
    let mut plan = candidate(vec![parameter.clone()], vec![], parameter);
    plan.body.result.ty = oversized;
    let error = reject(plan);
    assert_eq!(error.path, "return_value");
    assert!(error.detail.contains("resource"), "{error:?}");
}
