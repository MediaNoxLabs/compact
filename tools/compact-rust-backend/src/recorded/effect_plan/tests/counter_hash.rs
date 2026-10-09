// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Persistent fixed-byte products and canonical Field encoding have separate signatures.
use super::*;

fn bytes(n: usize) -> Expr {
    Expr::BytesLiteral { bytes: vec![7; n] }
}
fn hash(value: Expr) -> Expr {
    Expr::PersistentHash {
        value: Box::new(value),
    }
}
fn encode(value: Expr) -> Expr {
    Expr::FieldToBytes32 {
        value: Box::new(value),
    }
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "hash".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Bytes { length: 32 },
        return_value: StateReturn::Expression { value },
    }
}
fn checked(value: Expr) -> Result<CheckedPlan, PlanError> {
    check(&root(value), &HashMap::new(), &HashMap::new())
}
fn accepted(value: Expr) {
    let checked = checked(value).unwrap();
    render(&checked).unwrap();
}
fn tuple(n: usize) -> Expr {
    Expr::Tuple {
        elements: vec![bytes(32); n],
    }
}

#[test]
fn fixed_byte_products_include_old_pairs_triples_and_exact_fab_tuple_boundary() {
    for n in [0, 1, 2, 3, 11] {
        accepted(hash(tuple(n)));
    }
    assert!(matches!(
        checked(hash(tuple(12))),
        Err(PlanError::Unsupported(_))
    ));
    for n in [0, 3, 32] {
        accepted(hash(bytes(n)));
    }
    for n in [0, 2] {
        accepted(hash(Expr::Vector {
            element: Type::Bytes { length: 3 },
            elements: vec![bytes(3); n],
        }));
    }
    let leaf = Type::Struct {
        name: "BytesLeaf".into(),
        fields: vec![StructField {
            name: "payload".into(),
            ty: Type::Bytes { length: 3 },
        }],
    };
    accepted(hash(Expr::Tuple {
        elements: vec![
            Expr::StructLiteral {
                ty: leaf,
                fields: vec![bytes(3)],
            },
            Expr::Vector {
                element: Type::Bytes { length: 1 },
                elements: vec![bytes(1); 2],
            },
        ],
    }));
    accepted(hash(Expr::Unit));
    accepted(hash(Expr::Default {
        ty: Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
    }));
}

#[test]
fn persistent_domain_does_not_inherit_other_local_leaf_types() {
    for ty in [
        Type::Field,
        Type::Boolean,
        Type::JubjubPoint,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Unsigned { max: "255".into() },
    ] {
        assert!(!validate::hash_input_type(&ty), "{ty:?}");
        assert!(!validate::hash_input_type(&Type::Vector {
            element: Box::new(ty.clone()),
            length: 0
        }));
        assert!(!validate::hash_input_type(&Type::Tuple {
            elements: vec![Type::Bytes { length: 32 }, ty]
        }));
    }
    for value in [
        Expr::Boolean { value: true },
        Expr::FieldLiteral { value: "1".into() },
    ] {
        assert!(matches!(
            checked(hash(value)),
            Err(PlanError::Unsupported(_))
        ));
    }
    for value in [
        bytes(32),
        Expr::Boolean { value: true },
        Expr::UnsignedLiteral {
            value: "1".into(),
            max: "255".into(),
        },
    ] {
        assert!(matches!(
            checked(encode(value)),
            Err(PlanError::Unsupported(_))
        ));
    }
    accepted(encode(Expr::FieldLiteral { value: "1".into() }));
}

#[test]
fn hash_operands_preserve_observation_order_and_are_rendered_once() {
    fn observed() -> Expr {
        Expr::StructField {
            value: Box::new(Expr::KernelSelf {
                ty: crate::ir::contract_address_type(),
            }),
            field: "bytes".into(),
            index: 0,
        }
    }
    let checked = checked(hash(Expr::Tuple {
        elements: vec![observed(), observed(), bytes(3)],
    }))
    .unwrap();
    let bindings = &checked.plan().body.bindings;
    let paths: Vec<_> = bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::KernelSelf))
        .map(|b| b.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "return_value.value.value.elements[0].value",
            "return_value.value.value.elements[1].value"
        ]
    );
    assert!(matches!(
        bindings.last().unwrap().operation,
        Operation::PersistentHash(_)
    ));
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("kernel_self").count(), 2);
    assert_eq!(tokens.matches("persistent_hash").count(), 1);

    let error = checked_error(hash(Expr::Tuple {
        elements: vec![
            Expr::Parameter {
                name: "first_missing".into(),
            },
            encode(Expr::Boolean { value: true }),
        ],
    }));
    assert_eq!(error.path, "return_value.value.value.elements[0]");
}
fn checked_error(value: Expr) -> RecordingGap {
    match checked(value).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        other => panic!("expected source refusal: {other:?}"),
    }
}

fn validation_plan() -> Plan {
    let field = Value {
        id: 0,
        ty: Type::Field,
    };
    let bytes = Value {
        id: 1,
        ty: Type::Bytes { length: 32 },
    };
    let result = Value {
        id: 2,
        ty: bytes.ty.clone(),
    };
    Plan {
        parameters: vec![field.clone()],
        result_type: result.ty.clone(),
        body: Block {
            bindings: vec![
                Binding {
                    value: bytes.clone(),
                    operation: Operation::FieldToBytes32(field),
                    path: "encode".into(),
                },
                Binding {
                    value: result.clone(),
                    operation: Operation::PersistentHash(bytes),
                    path: "hash".into(),
                },
            ],
            result,
        },
    }
}
#[test]
fn independent_validation_rejects_forged_operands_results_scope_and_order() {
    validate::check(validation_plan(), declarations::Declarations::default()).unwrap();
    for mode in 0..8 {
        let mut plan = validation_plan();
        match mode {
            0 => {
                plan.body.bindings[0].operation = Operation::FieldToBytes32(Value {
                    id: 99,
                    ty: Type::Field,
                })
            }
            1 => {
                plan.body.bindings[0].operation = Operation::FieldToBytes32(Value {
                    id: 0,
                    ty: Type::Boolean,
                })
            }
            2 => plan.body.bindings[0].value.ty = Type::Bytes { length: 31 },
            3 => {
                plan.body.bindings[1].operation = Operation::PersistentHash(Value {
                    id: 0,
                    ty: Type::Field,
                })
            }
            4 => {
                plan.body.bindings[1].operation = Operation::PersistentHash(Value {
                    id: 1,
                    ty: Type::Bytes { length: 31 },
                })
            }
            5 => {
                plan.body.bindings[1].operation = Operation::PersistentHash(Value {
                    id: 99,
                    ty: Type::Bytes { length: 32 },
                })
            }
            6 => {
                plan.body.bindings[1].value.ty = Type::Field;
                plan.body.result.ty = Type::Field;
                plan.result_type = Type::Field;
            }
            7 => plan.body.bindings.swap(0, 1),
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, declarations::Declarations::default()).is_err(),
            "forgery {mode}"
        );
    }
}

#[test]
fn hash_reuses_existing_whole_product_byte_budget() {
    let limit = crate::resource_limits::recording_byte_payload_limit();
    accepted(hash(bytes(limit)));
    assert!(matches!(
        checked(hash(bytes(limit + 1))),
        Err(PlanError::Unsupported(_))
    ));
    assert!(matches!(
        checked(hash(Expr::Tuple {
            elements: vec![bytes(limit), bytes(1)]
        })),
        Err(PlanError::Unsupported(_))
    ));
}

#[test]
fn independent_hash_validation_enforces_product_arity_and_canonical_annotation() {
    let baseline = checked(hash(tuple(11))).unwrap();
    let mut plan = baseline.plan().clone();
    let tuple_index = plan.body.bindings.len() - 2;
    let Type::Tuple { elements } = &mut plan.body.bindings[tuple_index].value.ty else {
        unreachable!()
    };
    elements.push(Type::Bytes { length: 32 });
    let tuple_type = plan.body.bindings[tuple_index].value.ty.clone();
    let Operation::Tuple { elements } = &mut plan.body.bindings[tuple_index].operation else {
        unreachable!()
    };
    elements.push(elements[0].clone());
    let Operation::PersistentHash(input) = &mut plan.body.bindings.last_mut().unwrap().operation
    else {
        unreachable!()
    };
    input.ty = tuple_type;
    assert!(validate::check(plan, declarations::Declarations::default()).is_err());

    let ty = Type::Struct {
        name: "CanonicalBytes".into(),
        fields: vec![StructField {
            name: "payload".into(),
            ty: Type::Bytes { length: 3 },
        }],
    };
    let expression = hash(Expr::StructLiteral {
        ty,
        fields: vec![bytes(3)],
    });
    let environment = super::super::type_environment::TypeEnvironment::from_sources(
        &root(expression.clone()),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    let checked = checked(expression).unwrap();
    let mut plan = checked.plan().clone();
    let Operation::PersistentHash(input) = &mut plan.body.bindings.last_mut().unwrap().operation
    else {
        unreachable!()
    };
    let Type::Struct { fields, .. } = &mut input.ty else {
        unreachable!()
    };
    fields[0].name = "different".into();
    assert!(
        validate::check(
            plan,
            declarations::Declarations::default().with_type_environment(environment)
        )
        .is_err()
    );
}

#[test]
fn both_retained_source_roots_lower_directly_and_have_public_recording() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-counter-hash-schema20-ir.json"
    ))
    .unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let stateful = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    for name in ["run", "products"] {
        let root = contract
            .stateful_circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        let checked = check_with_calls(root, &ledger, &witnesses, &pure, &stateful)
            .unwrap_or_else(|error| panic!("{name} direct checked failure: {error:?}"));
        render(&checked).unwrap();
    }
    let output = crate::render_with_capabilities(&contract).unwrap();
    for name in ["run", "products"] {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "{capability:?}"
        );
    }
}
