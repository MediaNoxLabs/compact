// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Aligned input authority is independent from exact hash output signatures.
use super::super::validate::uint;
use super::*;

#[derive(Clone, Copy)]
enum Hash {
    Curve,
    Persistent,
}
impl Hash {
    fn result(self) -> Type {
        match self {
            Self::Curve => Type::JubjubPoint,
            Self::Persistent => Type::Bytes { length: 32 },
        }
    }
    fn expression(self, input: Expr) -> Expr {
        match self {
            Self::Curve => Expr::HashToCurve {
                value: Box::new(input),
            },
            Self::Persistent => Expr::PersistentHash {
                value: Box::new(input),
            },
        }
    }
    fn operation(self, input: Value) -> Operation {
        match self {
            Self::Curve => Operation::HashToCurve(input),
            Self::Persistent => Operation::PersistentHash(input),
        }
    }
    fn runtime(self) -> &'static str {
        match self {
            Self::Curve => "hash_to_curve",
            Self::Persistent => "persistent_hash",
        }
    }
}
fn source(hash: Hash, input: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "aligned_hash".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: input,
        }],
        actions: vec![],
        result: hash.result(),
        return_value: StateReturn::Expression {
            value: hash.expression(Expr::Parameter {
                name: "input".into(),
            }),
        },
    }
}
fn checked(hash: Hash, ty: Type) -> CheckedPlan {
    check(&source(hash, ty), &HashMap::new(), &HashMap::new()).unwrap()
}
fn plan(hash: Hash, ty: Type) -> Plan {
    let input = Value { id: 0, ty };
    let output = Value {
        id: 1,
        ty: hash.result(),
    };
    Plan {
        parameters: vec![input.clone()],
        result_type: hash.result(),
        body: Block {
            bindings: vec![Binding {
                value: output.clone(),
                operation: hash.operation(input),
                path: "hash".into(),
            }],
            result: output,
        },
    }
}

#[test]
fn aligned_hashes_accept_fixed_fab_leaves_products_and_exact_result_types() {
    let leaves = vec![
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 32 },
        uint("255"),
        uint(&u128::MAX.to_string()),
        uint("340282366920938463463374607431768211456"),
        Type::Enum {
            name: "Status".into(),
            variants: vec!["Ready".into(), "Done".into()],
        },
    ];
    for hash in [Hash::Curve, Hash::Persistent] {
        for leaf in &leaves {
            for ty in [
                leaf.clone(),
                Type::Vector {
                    element: Box::new(leaf.clone()),
                    length: 2,
                },
                Type::Struct {
                    name: "Payload".into(),
                    fields: vec![StructField {
                        name: "value".into(),
                        ty: leaf.clone(),
                    }],
                },
            ] {
                let checked = checked(hash, ty);
                assert_eq!(checked.plan().result_type, hash.result());
                let steps = render(&checked).unwrap().steps;
                assert_eq!(
                    quote::quote!(#(#steps)*)
                        .to_string()
                        .matches(hash.runtime())
                        .count(),
                    1
                );
            }
        }
        for arity in [0, 1, 8, 11] {
            checked(
                hash,
                Type::Tuple {
                    elements: vec![Type::Field; arity],
                },
            );
        }
    }
}

#[test]
fn aligned_hashes_refuse_unrepresentable_products_and_malformed_types() {
    for ty in [
        Type::Tuple {
            elements: vec![Type::Field; 12],
        },
        Type::Struct {
            name: "Overwide".into(),
            fields: vec![StructField {
                name: "value".into(),
                ty: Type::Tuple {
                    elements: vec![Type::Field; 9],
                },
            }],
        },
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
        uint("01"),
    ] {
        assert!(!validate::aligned_fab_type(&ty), "{ty:?}");
        for hash in [Hash::Curve, Hash::Persistent] {
            assert!(check(&source(hash, ty.clone()), &HashMap::new(), &HashMap::new()).is_err());
        }
    }
}

#[test]
fn independent_hash_validation_rechecks_ssa_and_operation_specific_results() {
    for hash in [Hash::Curve, Hash::Persistent] {
        validate::check(
            plan(hash, Type::Field),
            declarations::Declarations::default(),
        )
        .unwrap();
        for mode in 0..6 {
            let mut p = plan(hash, Type::Field);
            match mode {
                0 => {
                    p.body.bindings[0].operation = hash.operation(Value {
                        id: 99,
                        ty: Type::Field,
                    })
                }
                1 => {
                    p.body.bindings[0].operation = hash.operation(Value {
                        id: 1,
                        ty: Type::Field,
                    })
                }
                2 => {
                    p.body.bindings[0].operation = hash.operation(Value {
                        id: 0,
                        ty: Type::Boolean,
                    })
                }
                3 => {
                    p.body.bindings[0].value.ty = Type::Field;
                    p.body.result.ty = Type::Field;
                    p.result_type = Type::Field;
                }
                4 => {
                    p = plan(
                        hash,
                        Type::LedgerMap {
                            key: Box::new(Type::Field),
                            value: Box::new(Type::Field),
                        },
                    )
                }
                5 => {
                    p = plan(
                        hash,
                        Type::Tuple {
                            elements: vec![Type::Field; 12],
                        },
                    )
                }
                _ => unreachable!(),
            }
            assert!(
                validate::check(p, declarations::Declarations::default()).is_err(),
                "mode {mode}"
            );
        }
    }
}

#[test]
fn aligned_hashes_cannot_replace_canonical_named_declarations() {
    for hash in [Hash::Curve, Hash::Persistent] {
        let ty = Type::Struct {
            name: "Canonical".into(),
            fields: vec![StructField {
                name: "value".into(),
                ty: Type::Field,
            }],
        };
        let root = source(hash, ty.clone());
        let env = super::super::type_environment::TypeEnvironment::from_sources(
            &root,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        let mut p = plan(hash, ty);
        let Type::Struct { fields, .. } = &mut p.parameters[0].ty else {
            unreachable!()
        };
        fields[0].name = "forged".into();
        p.body.bindings[0].operation = hash.operation(p.parameters[0].clone());
        assert!(
            validate::check(
                p,
                declarations::Declarations::default().with_type_environment(env)
            )
            .is_err()
        );
    }
}

#[test]
fn aligned_hashes_preserve_lazy_witness_evaluation_and_first_error() {
    let witness = WitnessDeclaration {
        source: None,
        name: "next".into(),
        parameters: vec![],
        result: Type::Field,
    };
    for hash in [Hash::Curve, Hash::Persistent] {
        let mut root = source(hash, Type::Boolean);
        root.return_value = StateReturn::Expression {
            value: Expr::If {
                condition: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
                then: Box::new(hash.expression(Expr::WitnessCall {
                    name: "next".into(),
                    arguments: vec![],
                })),
                otherwise: Box::new(hash.expression(Expr::FieldLiteral { value: "0".into() })),
            },
        };
        let checked = check(&root, &HashMap::new(), &HashMap::from([("next", &witness)])).unwrap();
        let steps = render(&checked).unwrap().steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert_eq!(tokens.matches("try_witness_metered").count(), 1);
        assert_eq!(tokens.matches(hash.runtime()).count(), 2);
        assert!(tokens.find("if").unwrap() < tokens.find("try_witness_metered").unwrap());
        assert!(tokens.find("try_witness_metered").unwrap() < tokens.find(hash.runtime()).unwrap());
        root.return_value = StateReturn::Expression {
            value: hash.expression(Expr::Tuple {
                elements: vec![
                    Expr::Parameter {
                        name: "missing_first".into(),
                    },
                    Expr::FieldCast {
                        value: Box::new(Expr::Boolean { value: true }),
                    },
                ],
            }),
        };
        let PlanError::Unsupported(gap) =
            check(&root, &HashMap::new(), &HashMap::new()).unwrap_err()
        else {
            panic!("source refusal")
        };
        assert_eq!(gap.path, "return_value.value.value.elements[0]");
    }
}

#[path = "aligned_hashes/source.rs"]
mod source_probe;

#[path = "aligned_hashes/dynamic.rs"]
mod dynamic;
