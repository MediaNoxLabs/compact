// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Trait capability does not override PersistentHash's fixed-input semantics.
use super::*;

fn dynamic_types() -> Vec<Type> {
    [Type::OpaqueString, Type::OpaqueBytes]
        .into_iter()
        .flat_map(|ty| {
            [
                ty.clone(),
                Type::Vector {
                    element: Box::new(ty.clone()),
                    length: 2,
                },
                Type::Tuple {
                    elements: vec![Type::Field, ty.clone()],
                },
                Type::Struct {
                    name: "OwnedPayload".into(),
                    fields: vec![StructField {
                        name: "value".into(),
                        ty,
                    }],
                },
            ]
        })
        .collect()
}

#[test]
fn aligned_dynamic_inputs_preserve_operation_specific_source_policy() {
    for ty in dynamic_types() {
        assert!(validate::keccak_input_type(&ty));
        assert!(validate::aligned_fab_type(&ty));
        assert!(!validate::fixed_aligned_fab_type(&ty));
        checked(Hash::Curve, ty.clone());
        assert!(
            check(
                &source(Hash::Persistent, ty.clone()),
                &HashMap::new(),
                &HashMap::new()
            )
            .is_err()
        );
        for commit in [false, true] {
            let input = Expr::Parameter {
                name: "input".into(),
            };
            let mut root = source(Hash::Curve, ty.clone());
            root.result = Type::Field;
            root.return_value = StateReturn::Expression {
                value: if commit {
                    Expr::TransientCommit {
                        value: Box::new(input),
                        opening: Box::new(Expr::FieldLiteral { value: "7".into() }),
                    }
                } else {
                    Expr::TransientHash {
                        value: Box::new(input),
                    }
                },
            };
            let checked = check(&root, &HashMap::new(), &HashMap::new()).unwrap();
            let steps = render(&checked).unwrap().steps;
            let tokens = quote::quote!(#(#steps)*).to_string();
            assert_eq!(
                tokens
                    .matches(if commit {
                        "transient_commit"
                    } else {
                        "transient_hash"
                    })
                    .count(),
                1
            );
        }
    }
}

fn transient_plan(ty: Type, commit: bool) -> Plan {
    let mut p = plan(Hash::Curve, ty);
    let input = p.parameters[0].clone();
    p.parameters.push(Value {
        id: 1,
        ty: Type::Field,
    });
    p.body.bindings[0].value = Value {
        id: 2,
        ty: Type::Field,
    };
    p.body.bindings[0].operation = if commit {
        Operation::TransientCommit {
            value: input,
            opening: p.parameters[1].clone(),
        }
    } else {
        Operation::TransientHash(input)
    };
    p.result_type = Type::Field;
    p.body.result = p.body.bindings[0].value.clone();
    p
}

#[test]
fn independent_dynamic_validation_rechecks_policy_ssa_and_commit_opening() {
    fn declarations(ty: &Type) -> declarations::Declarations {
        let root = source(Hash::Curve, ty.clone());
        let env = super::super::super::type_environment::TypeEnvironment::from_sources(
            &root,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        declarations::Declarations::default().with_type_environment(env)
    }
    for ty in dynamic_types() {
        validate::check(plan(Hash::Curve, ty.clone()), declarations(&ty)).unwrap();
        // Same live, correctly annotated input and exact Bytes32 result must not
        // allow a forged PersistentHash operation through the dynamic policy.
        assert!(validate::check(plan(Hash::Persistent, ty.clone()), declarations(&ty)).is_err());
        for commit in [false, true] {
            let p = transient_plan(ty.clone(), commit);
            validate::check(p.clone(), declarations(&ty)).unwrap();
            for mode in 0..5 {
                let mut forged = p.clone();
                let operand = match &mut forged.body.bindings[0].operation {
                    Operation::TransientHash(value) | Operation::TransientCommit { value, .. } => {
                        value
                    }
                    _ => unreachable!(),
                };
                match mode {
                    0 => operand.id = 99,
                    1 => operand.id = 2,
                    2 => operand.ty = Type::Boolean,
                    3 => {
                        forged.body.bindings[0].value.ty = Type::Bytes { length: 32 };
                        forged.body.result.ty = Type::Bytes { length: 32 };
                        forged.result_type = Type::Bytes { length: 32 };
                    }
                    4 if commit => {
                        forged.parameters[1].ty = Type::Boolean;
                        let Operation::TransientCommit { opening, .. } =
                            &mut forged.body.bindings[0].operation
                        else {
                            unreachable!()
                        };
                        opening.ty = Type::Boolean;
                    }
                    _ => continue,
                }
                assert!(
                    validate::check(forged, declarations(&ty)).is_err(),
                    "commit={commit}, mode={mode}"
                );
            }
        }
    }
}
