// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::ir::{Expr, LedgerFieldKind, Parameter, StateAction, StateReturn};

fn enumeration(name: &str) -> Type {
    Type::Enum {
        name: name.into(),
        variants: vec!["Ready".into(), "Done".into()],
    }
}
fn structure(name: &str, ty: Type) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn root(ty: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty,
        }],
        result: Type::Unit,
        actions: vec![],
        return_value: StateReturn::Unit,
    }
}
fn environment(root: &StatefulCircuit) -> Result<TypeEnvironment, RecordingGap> {
    TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
}

#[test]
fn canonical_nested_definitions_accept_matching_values_and_reject_forged_payloads() {
    let ty = structure(
        "Envelope",
        Type::Tuple {
            elements: vec![
                Type::Unit,
                enumeration("Status"),
                Type::Bytes { length: 16 },
            ],
        },
    );
    let root = root(ty.clone());
    let env = environment(&root).unwrap();
    env.validate(&ty).unwrap();
    env.validate(&enumeration("Status")).unwrap();
    let mut wrong = ty.clone();
    if let Type::Struct { fields, .. } = &mut wrong {
        fields[0].name = "other".into();
    }
    assert!(env.validate(&wrong).is_err());
    let mut wrong = enumeration("Status");
    if let Type::Enum { variants, .. } = &mut wrong {
        variants.reverse();
    }
    assert!(env.validate(&wrong).is_err());
    assert!(env.validate(&structure("Undeclared", Type::Field)).is_err());
    assert!(TypeEnvironment::default().validate(&ty).is_err());
}

#[test]
fn conflicting_and_normalized_names_are_refused_without_changing_native_policy() {
    let definitions = vec![
        vec![
            structure("Same", Type::Field),
            structure("Same", Type::Boolean),
        ],
        vec![structure("Same", Type::Field), enumeration("Same")],
        vec![structure("a$b", Type::Field), structure("a_b", Type::Field)],
        vec![Type::Struct {
            name: "Record".into(),
            fields: vec![
                StructField {
                    name: "a$b".into(),
                    ty: Type::Field,
                },
                StructField {
                    name: "a_b".into(),
                    ty: Type::Field,
                },
            ],
        }],
        vec![Type::Enum {
            name: "Status".into(),
            variants: vec!["a$b".into(), "a_b".into()],
        }],
        vec![Type::Enum {
            name: "Status".into(),
            variants: vec![],
        }],
    ];
    for elements in definitions {
        assert!(environment(&root(Type::Tuple { elements })).is_err());
    }
}

#[test]
fn all_authoritative_source_owners_contribute_definitions() {
    let mut root = root(Type::Unit);
    root.actions = vec![StateAction::Expression {
        value: Expr::Default {
            ty: enumeration("Body"),
        },
    }];
    root.return_value = StateReturn::Effectful {
        body: crate::ir::ReturnPlan::Value {
            value: Expr::Default {
                ty: enumeration("Return"),
            },
        },
    };
    let field = LedgerField {
        source: None,
        id: "slot".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell {
            ty: enumeration("Ledger"),
        },
    };
    let witness = WitnessDeclaration {
        source: None,
        name: "witness".into(),
        parameters: vec![],
        result: enumeration("Witness"),
    };
    let pure = PureCircuit {
        source: None,
        name: "pure".into(),
        internal: true,
        parameters: vec![],
        result: Type::Unit,
        body: Expr::Default {
            ty: enumeration("PureBody"),
        },
    };
    let mut helper = root.clone();
    helper.name = "helper".into();
    helper.parameters[0].ty = enumeration("Helper");
    let env = TypeEnvironment::from_sources(
        &root,
        &HashMap::from([("slot", &field)]),
        &HashMap::from([("witness", &witness)]),
        &HashMap::from([("pure", &pure)]),
        &HashMap::from([("root", &root), ("helper", &helper)]),
    )
    .unwrap();
    for name in ["Body", "Return", "Ledger", "Witness", "PureBody", "Helper"] {
        env.validate(&enumeration(name)).unwrap();
    }
}

#[test]
fn source_conflict_hidden_in_an_uninvoked_body_is_still_refused() {
    let root = root(enumeration("Status"));
    let pure = PureCircuit {
        source: None,
        name: "unused".into(),
        internal: true,
        parameters: vec![],
        result: Type::Unit,
        body: Expr::Default {
            ty: structure("Status", Type::Field),
        },
    };
    assert!(
        TypeEnvironment::from_sources(
            &root,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("unused", &pure)]),
            &HashMap::new()
        )
        .is_err()
    );
}

#[test]
fn fixed_byte_leaf_bound_is_per_aggregate_and_checked_before_defaults() {
    let env = TypeEnvironment::default();
    let limit = crate::resource_limits::recording_byte_payload_limit();
    for length in [0, 1, 32, limit] {
        env.validate(&Type::Bytes { length }).unwrap();
    }
    assert!(env.validate(&Type::Bytes { length: limit + 1 }).is_err());
    assert!(env.validate(&Type::Bytes { length: usize::MAX }).is_err());
    let pair = Type::Tuple {
        elements: vec![Type::Bytes { length: limit / 2 }; 2],
    };
    env.validate(&pair).unwrap();
    let excessive = structure(
        "Large",
        Type::Tuple {
            elements: vec![pair, Type::Bytes { length: 1 }],
        },
    );
    let source = root(excessive.clone());
    let env = environment(&source).unwrap();
    assert!(env.validate(&excessive).is_err());
}

#[test]
fn tuples_and_owned_values_are_local_types_but_ledger_maps_remain_refused() {
    let env = TypeEnvironment::default();
    env.validate(&Type::Tuple {
        elements: vec![
            Type::Unit,
            Type::JubjubPoint,
            Type::Field,
            Type::Unsigned {
                max: "340282366920938463463374607431768211456".into(),
            },
        ],
    })
    .unwrap();
    for ty in [
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Vector {
            element: Box::new(Type::Field),
            length: 1,
        },
    ] {
        env.validate(&ty).unwrap();
    }
    for ty in [
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
        Type::Unsigned { max: "bad".into() },
    ] {
        assert!(env.validate(&ty).is_err());
    }
    let empty = Type::Struct {
        name: "Empty".into(),
        fields: vec![],
    };
    assert!(
        environment(&root(empty.clone()))
            .unwrap()
            .validate(&empty)
            .is_err()
    );
}

#[test]
fn deep_raw_sources_and_forged_types_refuse_before_recursive_collection_or_equality() {
    let mut deep = Type::Field;
    for _ in 0..80 {
        deep = Type::Tuple {
            elements: vec![deep],
        };
    }
    let env = TypeEnvironment::default();
    assert!(
        env.validate(&deep)
            .unwrap_err()
            .detail
            .contains("syntax_depth")
    );
    let deep_root = root(deep);
    assert!(
        environment(&deep_root)
            .unwrap_err()
            .detail
            .contains("syntax_depth")
    );
    let mut body = Expr::Unit;
    for _ in 0..80 {
        body = Expr::Let {
            bindings: vec![],
            body: Box::new(body),
        };
    }
    let pure = PureCircuit {
        source: None,
        name: "deep".into(),
        internal: true,
        parameters: vec![],
        result: Type::Unit,
        body,
    };
    let shallow = root(Type::Unit);
    assert!(
        TypeEnvironment::from_sources(
            &shallow,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("deep", &pure)]),
            &HashMap::new()
        )
        .unwrap_err()
        .detail
        .contains("syntax_depth")
    );
}

#[test]
fn unrelated_large_or_unsupported_declarations_do_not_refuse_a_scalar_root() {
    let root = root(Type::Field);
    let large = Type::Bytes { length: usize::MAX };
    let field = LedgerField {
        source: None,
        id: "unused".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell { ty: large.clone() },
    };
    let pure = PureCircuit {
        source: None,
        name: "unused".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Vector {
                element: Box::new(large.clone()),
                length: 8,
            },
        }],
        result: Type::Unit,
        body: Expr::Unit,
    };
    let ledger = HashMap::from([("unused", &field)]);
    let pure = HashMap::from([("unused", &pure)]);
    let env =
        TypeEnvironment::from_sources(&root, &ledger, &HashMap::new(), &pure, &HashMap::new())
            .unwrap();
    assert!(
        env.validate(&large).is_err(),
        "used type still has a checked bound"
    );
    super::super::check_with_calls(&root, &ledger, &HashMap::new(), &pure, &HashMap::new())
        .unwrap();
}

#[test]
fn active_large_aggregate_is_a_source_refusal_not_a_checked_plan_invariant() {
    let large = structure(
        "Large",
        Type::Bytes {
            length: crate::resource_limits::recording_byte_payload_limit(),
        },
    );
    let excessive = Type::Struct {
        name: "Pair".into(),
        fields: vec![
            StructField {
                name: "left".into(),
                ty: large.clone(),
            },
            StructField {
                name: "right".into(),
                ty: large,
            },
        ],
    };
    let mut source = root(Type::Field);
    source.result = excessive.clone();
    source.return_value = StateReturn::Expression {
        value: Expr::Default { ty: excessive },
    };
    let result = super::super::check_with_calls(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );
    match result {
        Err(super::super::PlanError::Unsupported(gap)) => {
            assert!(gap.detail.contains("fixed-byte aggregate"))
        }
        other => panic!("active aggregate policy must refuse before checked validation: {other:?}"),
    }
}

#[test]
fn raw_resource_and_canonical_conflicts_are_source_refusals() {
    let mut deep = Type::Field;
    for _ in 0..80 {
        deep = Type::Tuple {
            elements: vec![deep],
        };
    }
    let sources = [
        root(deep),
        root(Type::Tuple {
            elements: vec![
                enumeration("Duplicate"),
                structure("Duplicate", Type::Field),
            ],
        }),
    ];
    for source in sources {
        let result = super::super::check_with_calls(
            &source,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        );
        assert!(
            matches!(result, Err(super::super::PlanError::Unsupported(_))),
            "raw source failure is not a lowered invariant: {result:?}"
        );
    }
}

mod owned;
