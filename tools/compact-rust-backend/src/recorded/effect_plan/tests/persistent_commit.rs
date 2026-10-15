// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Persistent commitment inputs use fixed aligned FABs; opening and SSA authority
//! remain independent from representation eligibility.
use super::super::type_environment::TypeEnvironment;
use super::*;

fn bytes() -> Type {
    Type::Bytes { length: 32 }
}
fn source(input: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "commit".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "input".into(),
                ty: input,
            },
            Parameter {
                name: "opening".into(),
                ty: bytes(),
            },
        ],
        actions: vec![],
        result: bytes(),
        return_value: StateReturn::Expression {
            value: Expr::PersistentCommit {
                value: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
                opening: Box::new(Expr::Parameter {
                    name: "opening".into(),
                }),
            },
        },
    }
}
fn environment(ty: Type) -> declarations::Declarations {
    let types = TypeEnvironment::from_sources(
        &source(ty),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default().with_type_environment(types)
}
fn plan(ty: Type) -> Plan {
    let input = Value { id: 0, ty };
    let opening = Value { id: 1, ty: bytes() };
    let result = Value { id: 2, ty: bytes() };
    Plan {
        parameters: vec![input.clone(), opening.clone()],
        result_type: bytes(),
        body: Block {
            bindings: vec![Binding {
                value: result.clone(),
                operation: Operation::PersistentCommit {
                    value: input,
                    opening,
                },
                path: "return_value.value".into(),
            }],
            result,
        },
    }
}
fn product(ty: Type) -> Type {
    Type::Struct {
        name: "Payload".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}

#[test]
fn fixed_aligned_inputs_lower_and_validate_with_unchanged_runtime_signature() {
    let types = vec![
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::Bytes { length: 0 },
        bytes(),
        Type::Unsigned {
            max: u128::MAX.to_string(),
        },
        Type::Unsigned {
            max: "340282366920938463463374607431768211456".into(),
        },
        Type::Enum {
            name: "Status".into(),
            variants: vec!["Ready".into(), "Done".into()],
        },
        Type::Tuple { elements: vec![] },
        Type::Tuple {
            elements: vec![bytes()],
        },
        Type::Tuple {
            elements: vec![Type::Field; 11],
        },
        Type::Vector {
            element: Box::new(Type::Field),
            length: 2,
        },
        product(Type::Vector {
            element: Box::new(Type::Tuple {
                elements: vec![Type::Field, Type::Boolean],
            }),
            length: 2,
        }),
    ];
    for ty in types {
        let checked = check(&source(ty.clone()), &HashMap::new(), &HashMap::new()).unwrap();
        validate::check(plan(ty.clone()), environment(ty)).unwrap();
        let lowered = checked.plan();
        let Operation::PersistentCommit { value, opening } =
            &lowered.body.bindings.last().unwrap().operation
        else {
            panic!("commit operation")
        };
        assert_ne!(value.id, opening.id);
        assert_eq!(opening.ty, bytes());
        assert_eq!(lowered.result_type, bytes());
        let steps = render(&checked).unwrap().steps;
        assert_eq!(
            quote::quote!(#(#steps)*)
                .to_string()
                .matches("persistent_commit")
                .count(),
            1
        );
    }
}

#[test]
fn opaque_inputs_remain_explicit_native_ir_policy_refusals_recursively() {
    // Native Rust traits accept these carriers, but the Compact frontend excludes
    // opaque persistent commitments. These are IR policy controls, not source gaps.
    for ty in [
        Type::OpaqueString,
        Type::OpaqueBytes,
        product(Type::OpaqueBytes),
        Type::Tuple {
            elements: vec![Type::Field, Type::OpaqueString],
        },
        Type::Vector {
            element: Box::new(Type::OpaqueBytes),
            length: 0,
        },
    ] {
        let Err(PlanError::Unsupported(gap)) =
            check(&source(ty.clone()), &HashMap::new(), &HashMap::new())
        else {
            panic!("opaque policy")
        };
        assert_eq!(gap.path, "return_value.value");
        assert!(gap.detail.contains("fixed aligned FAB"));
        assert!(validate::check(plan(ty.clone()), environment(ty)).is_err());
    }
}

#[test]
fn independent_commit_validation_preserves_opening_result_and_live_value_authority() {
    for mode in 0..8 {
        let mut p = plan(Type::Field);
        match mode {
            0..=5 => {
                let Operation::PersistentCommit { value, opening } =
                    &mut p.body.bindings[0].operation
                else {
                    unreachable!()
                };
                match mode {
                    0 => value.id = 99,
                    1 => value.id = 2,
                    2 => value.ty = Type::Boolean,
                    3 => opening.id = 0,
                    4 => opening.ty = Type::Bytes { length: 31 },
                    5 => opening.id = 2,
                    _ => unreachable!(),
                }
            }
            6 => {
                p.body.bindings[0].value.ty = Type::Field;
                p.body.result.ty = Type::Field;
                p.result_type = Type::Field;
            }
            7 => {
                p.parameters.remove(0);
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(p, environment(Type::Field)).is_err(),
            "mutation {mode}"
        );
    }
    let mut source = source(Type::Field);
    source.parameters[1].ty = Type::Bytes { length: 31 };
    assert!(check(&source, &HashMap::new(), &HashMap::new()).is_err());
}

#[test]
fn representation_canonical_and_fixed_layout_limits_still_precede_hash_eligibility() {
    for ty in [
        Type::Tuple {
            elements: vec![Type::Field; 12],
        },
        product(Type::Tuple {
            elements: vec![Type::Field; 9],
        }),
        Type::Unsigned { max: "01".into() },
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
        Type::Vector {
            element: Box::new(Type::Field),
            length: 32769,
        },
    ] {
        assert!(check(&source(ty), &HashMap::new(), &HashMap::new()).is_err());
    }
    let canonical = product(Type::Field);
    let forged = product(Type::Boolean);
    assert!(validate::check(plan(forged), environment(canonical)).is_err());
}
