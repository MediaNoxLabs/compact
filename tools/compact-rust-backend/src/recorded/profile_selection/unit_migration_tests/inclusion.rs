// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn values() -> Vec<(&'static str, &'static str, u8, Type, Expr)> {
    vec![
        ("parameter", "amount", 0, Type::Field, parameter("input")),
        ("field", "amount", 0, Type::Field, field()),
        ("field-read", "amount", 0, Type::Field, read("amount", 0)),
        (
            "field-coerce",
            "amount",
            0,
            Type::Field,
            Expr::Coerce {
                value: Box::new(field()),
                ty: Type::Field,
            },
        ),
        (
            "field-let",
            "amount",
            0,
            Type::Field,
            local("input", Type::Field, field(), parameter("input")),
        ),
        (
            "field-witness",
            "amount",
            0,
            Type::Field,
            witness("w_amount"),
        ),
        (
            "field-pure",
            "amount",
            0,
            Type::Field,
            call("p_amount", vec![field()]),
        ),
        (
            "projection",
            "amount",
            0,
            Type::Field,
            project(record_value(), "amount", 0),
        ),
        ("boolean", "flag", 1, Type::Boolean, boolean()),
        ("boolean-read", "flag", 1, Type::Boolean, read("flag", 1)),
        (
            "boolean-witness",
            "flag",
            1,
            Type::Boolean,
            witness("w_flag"),
        ),
        (
            "boolean-pure",
            "flag",
            1,
            Type::Boolean,
            call("p_flag", vec![boolean()]),
        ),
        (
            "field-equal",
            "flag",
            1,
            Type::Boolean,
            equal(field(), parameter("input")),
        ),
        (
            "unit-equal",
            "flag",
            1,
            Type::Boolean,
            equal(Expr::Unit, Expr::Unit),
        ),
        (
            "enum-equal",
            "flag",
            1,
            Type::Boolean,
            equal(variant(), variant()),
        ),
        (
            "struct-equal",
            "flag",
            1,
            Type::Boolean,
            equal(record_value(), record_value()),
        ),
        (
            "tuple-equal",
            "flag",
            1,
            Type::Boolean,
            equal(
                Expr::Tuple {
                    elements: vec![Expr::Unit, bytes(), record_value()],
                },
                Expr::Tuple {
                    elements: vec![Expr::Unit, bytes(), record_value()],
                },
            ),
        ),
        (
            "boolean-if",
            "flag",
            1,
            Type::Boolean,
            Expr::If {
                condition: Box::new(read("flag", 1)),
                then: Box::new(witness("w_flag")),
                otherwise: Box::new(boolean()),
            },
        ),
        (
            "nested-projection",
            "flag",
            1,
            Type::Boolean,
            project(project(record_value(), "inner", 1), "flag", 0),
        ),
        ("bytes", "digest", 2, bytes_type(), bytes()),
        ("bytes-read", "digest", 2, bytes_type(), read("digest", 2)),
        (
            "bytes-witness",
            "digest",
            2,
            bytes_type(),
            witness("w_digest"),
        ),
        (
            "bytes-pure",
            "digest",
            2,
            bytes_type(),
            call("p_digest", vec![bytes()]),
        ),
        ("enum", "phase", 3, phase(), variant()),
        ("enum-read", "phase", 3, phase(), read("phase", 3)),
        ("enum-witness", "phase", 3, phase(), witness("w_phase")),
        (
            "enum-pure",
            "phase",
            3,
            phase(),
            call("p_phase", vec![variant()]),
        ),
        ("struct", "record", 4, record(), record_value()),
        (
            "struct-default",
            "record",
            4,
            record(),
            Expr::Default { ty: record() },
        ),
        ("struct-read", "record", 4, record(), read("record", 4)),
        ("struct-witness", "record", 4, record(), witness("w_record")),
        (
            "struct-pure",
            "record",
            4,
            record(),
            call("p_record", vec![record_value()]),
        ),
    ]
}
fn wrap(name: &str, index: u8, ty: Type, value: Expr, mode: usize) -> StateAction {
    let action = write(name, index, value.clone());
    match mode {
        0 => action,
        1 => StateAction::Sequence {
            actions: vec![
                StateAction::Sequence { actions: vec![] },
                StateAction::Assert {
                    condition: boolean(),
                    message: "before write".into(),
                },
                action,
            ],
        },
        2 => StateAction::Let {
            bindings: vec![LocalBinding {
                name: "chosen".into(),
                ty,
                value,
            }],
            action: Box::new(write(name, index, parameter("chosen"))),
        },
        3 => StateAction::If {
            condition: read("flag", 1),
            then: Box::new(action.clone()),
            otherwise: Box::new(action),
        },
        _ => unreachable!(),
    }
}

#[test]
fn finite_rich_values_cross_each_old_action_rule() {
    let cases = values();
    assert_eq!(cases.len(), 32);
    for (label, name, index, ty, value) in cases {
        for mode in 0..4 {
            let mut contract = subject();
            contract.stateful_circuits[1].actions =
                vec![wrap(name, index, ty.clone(), value.clone(), mode)];
            included(&contract, "root", &format!("{label}/action-{mode}"));
        }
    }
}

#[test]
fn root_control_sequence_assertions_and_lexical_shadowing_are_old_positive() {
    for mode in 0..5 {
        let mut contract = subject();
        let invocation = call("store", vec![parameter("seed")]);
        let value = match mode {
            0 => Expr::If {
                condition: Box::new(read("flag", 1)),
                then: Box::new(invocation.clone()),
                otherwise: Box::new(invocation),
            },
            1 => Expr::Sequence {
                steps: vec![
                    Expr::Assert {
                        condition: Box::new(boolean()),
                        message: "root guard".into(),
                    },
                    invocation,
                ],
                value: Box::new(Expr::Unit),
            },
            2 => local("seed", Type::Field, witness("w_amount"), invocation),
            3 => Expr::Sequence {
                steps: vec![invocation.clone()],
                value: Box::new(invocation),
            },
            4 => Expr::If {
                condition: Box::new(Expr::Boolean { value: false }),
                then: Box::new(invocation),
                otherwise: Box::new(Expr::Unit),
            },
            _ => unreachable!(),
        };
        contract.stateful_circuits[0].return_value = StateReturn::Expression { value };
        included(&contract, "root", &format!("root-control-{mode}"));
    }
}

#[test]
fn nested_repeated_stateful_helpers_have_fresh_scopes_and_need_not_be_internal() {
    let mut contract = subject();
    let mut nested = contract.stateful_circuits[1].clone();
    nested.name = "nested".into();
    contract.stateful_circuits.push(nested);
    contract.stateful_circuits[1].internal = false;
    contract.stateful_circuits[1].actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "snapshot".into(),
            ty: Type::Field,
            value: Expr::Sequence {
                steps: vec![
                    call("nested", vec![parameter("input")]),
                    call("nested", vec![field()]),
                ],
                value: Box::new(parameter("input")),
            },
        }],
        action: Box::new(write("amount", 0, parameter("snapshot"))),
    }];
    included(&contract, "root", "nested/repeated/noninternal-helper");
}

#[test]
fn native_pure_helpers_cover_old_hash_and_field_struct_commitment_rules() {
    let mut contract = subject();
    let numbers = Type::Struct {
        name: "Numbers".into(),
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
    };
    contract.circuits.push(PureCircuit {
        source: None,
        name: "hash_pair".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "first".into(),
                ty: bytes_type(),
            },
            Parameter {
                name: "second".into(),
                ty: bytes_type(),
            },
        ],
        result: bytes_type(),
        body: Expr::PersistentHash {
            value: Box::new(Expr::Tuple {
                elements: vec![parameter("first"), parameter("second")],
            }),
        },
    });
    contract.circuits.push(PureCircuit {
        source: None,
        name: "commit_numbers".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "numbers".into(),
                ty: numbers.clone(),
            },
            Parameter {
                name: "opening".into(),
                ty: Type::Field,
            },
        ],
        result: Type::Field,
        body: Expr::TransientCommit {
            value: Box::new(parameter("numbers")),
            opening: Box::new(parameter("opening")),
        },
    });
    contract.circuits.push(PureCircuit {
        source: None,
        name: "pure_control".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "numbers".into(),
            ty: numbers.clone(),
        }],
        result: Type::Boolean,
        body: local(
            "shadow",
            numbers.clone(),
            Expr::Default {
                ty: numbers.clone(),
            },
            Expr::If {
                condition: Box::new(equal(parameter("shadow"), parameter("numbers"))),
                then: Box::new(equal(
                    project(parameter("numbers"), "first", 0),
                    call("p_amount", vec![field()]),
                )),
                otherwise: Box::new(Expr::Coerce {
                    value: Box::new(boolean()),
                    ty: Type::Boolean,
                }),
            },
        ),
    });
    let numbers_value = Expr::StructLiteral {
        ty: numbers,
        fields: vec![parameter("input"), field()],
    };
    contract.stateful_circuits[1].actions = vec![
        write(
            "digest",
            2,
            call("hash_pair", vec![read("digest", 2), witness("w_digest")]),
        ),
        write(
            "amount",
            0,
            call(
                "commit_numbers",
                vec![numbers_value.clone(), parameter("input")],
            ),
        ),
        write("flag", 1, call("pure_control", vec![numbers_value])),
    ];
    included(&contract, "root", "pure-hash-commit-control-transitive");
}

#[test]
fn retained_original_coracle_guess_is_admitted_before_new_checks() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/coracle-guess-schema20-ir.json"
    ))
    .unwrap();
    let exports: Vec<_> = contract
        .stateful_circuits
        .iter()
        .filter(|circuit| !circuit.internal)
        .map(|circuit| circuit.name.as_str())
        .collect();
    assert_eq!(exports, ["guess"]);
    included(&contract, "guess", "retained Coracle");
}
