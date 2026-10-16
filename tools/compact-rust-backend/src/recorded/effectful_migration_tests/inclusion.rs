// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite compositional inclusion evidence for the inspected old effectful
//! grammar. Every case must first pass the frozen old-family lowerer; there is
//! no rejection filtering. Counts are samples, not a language-inclusion proof.
use super::*;
use crate::ir::{LocalBinding, Parameter, WitnessDeclaration};

const SMALL_MAX: &str = "340282366920938463463374607431768211455";
const WIDE_MAX: &str =
    "452312848583266388373324160190187140051835877600158453279131187530910662655";
const WIDE_VALUE: &str = "340282366920938463463374607431768211456";
const WIDE_NEXT: &str = "340282366920938463463374607431768211457";

fn local(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}

fn field(value: u64) -> Expr {
    Expr::FieldLiteral {
        value: value.to_string(),
    }
}

fn read(name: &str, index: u8) -> Expr {
    Expr::CellRead {
        field: name.into(),
        index,
    }
}

fn unsigned(value: &str, max: &str) -> Expr {
    Expr::UnsignedLiteral {
        value: value.into(),
        max: max.into(),
    }
}

fn coerce(value: Expr, ty: Type) -> Expr {
    Expr::Coerce {
        value: Box::new(value),
        ty,
    }
}

fn equal(left: Expr, right: Expr) -> Expr {
    Expr::Equal {
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn binding(name: &str, ty: Type, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty,
        value,
    }
}

fn wide_binding(name: &str, value: &str) -> LocalBinding {
    binding(
        name,
        Type::Unsigned {
            max: WIDE_MAX.into(),
        },
        unsigned(value, WIDE_MAX),
    )
}

fn write(name: &str, index: u8, value: Expr) -> StateAction {
    StateAction::CellWrite {
        field: name.into(),
        index,
        value,
    }
}

fn returned(value: Expr) -> ReturnPlan {
    ReturnPlan::Value { value }
}

fn field_expressions() -> Vec<(&'static str, Expr)> {
    vec![
        ("parameter", local("next")),
        ("field-literal", field(17)),
        ("cell-read", read("unused", 1)),
        (
            "add",
            Expr::Add {
                left: Box::new(local("before")),
                right: Box::new(field(1)),
            },
        ),
        ("field-identity", coerce(local("before"), Type::Field)),
        ("small-coerce", coerce(local("narrow"), Type::Field)),
        (
            "small-field-cast",
            Expr::FieldCast {
                value: Box::new(local("narrow")),
            },
        ),
        ("wide-coerce", coerce(local("wide"), Type::Field)),
        (
            "inline-wide-coerce",
            coerce(unsigned(WIDE_MAX, WIDE_MAX), Type::Field),
        ),
        (
            "zero-argument-witness",
            Expr::WitnessCall {
                name: "mark".into(),
                arguments: vec![],
            },
        ),
        (
            "ordered-witness-arguments",
            Expr::WitnessCall {
                name: "observe".into(),
                arguments: vec![read("state", 0), coerce(local("wide"), Type::Field)],
            },
        ),
        (
            "nested-wide-shadow",
            Expr::Let {
                bindings: vec![wide_binding("wide", WIDE_NEXT)],
                body: Box::new(Expr::Let {
                    bindings: vec![binding(
                        "next",
                        Type::Field,
                        coerce(local("wide"), Type::Field),
                    )],
                    body: Box::new(local("next")),
                }),
            },
        ),
    ]
}

fn boolean_expressions() -> Vec<(&'static str, Expr)> {
    vec![
        ("true", Expr::Boolean { value: true }),
        ("false", Expr::Boolean { value: false }),
        ("field-equality", equal(read("state", 0), local("next"))),
        (
            "small-equality",
            equal(local("narrow"), unsigned(SMALL_MAX, SMALL_MAX)),
        ),
        (
            "wide-equality",
            equal(local("wide"), unsigned(WIDE_VALUE, WIDE_MAX)),
        ),
        (
            "wide-inequality",
            equal(local("wide"), unsigned(WIDE_NEXT, WIDE_MAX)),
        ),
        (
            "boolean-if",
            Expr::If {
                condition: Box::new(local("flag")),
                then: Box::new(equal(coerce(local("wide"), Type::Field), field(1))),
                otherwise: Box::new(coerce(Expr::Boolean { value: false }, Type::Boolean)),
            },
        ),
        (
            "boolean-let-shadow",
            Expr::Let {
                bindings: vec![binding(
                    "flag",
                    Type::Boolean,
                    equal(Expr::Boolean { value: true }, local("flag")),
                )],
                body: Box::new(coerce(local("flag"), Type::Boolean)),
            },
        ),
    ]
}

/// Each arm is legal independently; static read/write evidence also covers the
/// old family's admission of an arm with no branch-local public effect.
fn conditional(condition: Expr, value: Expr) -> ReturnPlan {
    ReturnPlan::Conditional {
        condition,
        then: Box::new(ReturnPlan::Sequence {
            actions: vec![StateAction::Let {
                bindings: vec![
                    binding("before", Type::Field, field(23)),
                    wide_binding("wide", WIDE_NEXT),
                ],
                action: Box::new(StateAction::Sequence {
                    actions: vec![
                        write("state", 0, value.clone()),
                        write("unused", 1, coerce(local("wide"), Type::Field)),
                    ],
                }),
            }],
            // The action's shadowed locals must not escape to this result.
            result: Box::new(returned(value)),
        }),
        otherwise: Box::new(returned(local("before"))),
    }
}

fn body(condition: Expr, value: Expr, topology: usize) -> ReturnPlan {
    let inner = conditional(condition.clone(), value.clone());
    let result = match topology {
        0 => inner,
        1 => ReturnPlan::Sequence {
            actions: vec![StateAction::Sequence {
                actions: vec![write("unused", 1, field(31))],
            }],
            result: Box::new(ReturnPlan::Let {
                bindings: vec![binding("before", Type::Field, read("unused", 1))],
                result: Box::new(inner),
            }),
        },
        2 => ReturnPlan::Let {
            bindings: vec![
                wide_binding("wide", WIDE_NEXT),
                binding("flag", Type::Boolean, Expr::Boolean { value: false }),
            ],
            result: Box::new(ReturnPlan::Conditional {
                condition,
                then: Box::new(inner),
                otherwise: Box::new(ReturnPlan::Sequence {
                    actions: vec![write("state", 0, value)],
                    result: Box::new(returned(coerce(local("wide"), Type::Field))),
                }),
            }),
        },
        _ => unreachable!("bounded topology generator"),
    };
    ReturnPlan::Let {
        bindings: vec![
            binding("before", Type::Field, read("state", 0)),
            binding(
                "narrow",
                Type::Unsigned {
                    max: SMALL_MAX.into(),
                },
                unsigned(SMALL_MAX, SMALL_MAX),
            ),
            wide_binding("wide", WIDE_VALUE),
            binding("flag", Type::Boolean, Expr::Boolean { value: true }),
        ],
        result: Box::new(result),
    }
}

fn fixture() -> Contract {
    let mut contract = effectful_fixture();
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "observe".into(),
        parameters: vec![
            Parameter {
                name: "first".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "second".into(),
                ty: Type::Field,
            },
        ],
        result: Type::Field,
    });
    contract
}

fn assert_included(contract: &Contract, label: &str) {
    let context = Context::new(contract);
    let circuit = &contract.stateful_circuits[0];
    assert!(
        context.legacy(circuit).is_some(),
        "{label}: generator escaped old admitted grammar"
    );
    let algebra = context.algebra(circuit).unwrap_or_else(|error| {
        panic!("{label}: old-admitted program refused by algebra: {error:?}")
    });
    let selected = supported(context.selected(circuit));
    assert_eq!(
        tokens(&selected),
        tokens(&algebra),
        "{label}: default selection must use checked algebra"
    );
    let rendered = crate::render_with_capabilities(contract)
        .unwrap_or_else(|error| panic!("{label}: public compiler failure: {error:?}"));
    let capability = &rendered.capabilities.circuits[0];
    assert!(
        capability.recorded && capability.observed_call,
        "{label}: public recording route declined"
    );
}

#[test]
fn finite_compositions_of_every_inspected_old_value_action_and_return_form_are_included() {
    let fields = field_expressions();
    let conditions = boolean_expressions();
    assert_eq!(fields.len(), 12);
    assert_eq!(conditions.len(), 8);
    let mut cases = 0;
    for (field_name, value) in fields {
        for (condition_name, condition) in &conditions {
            for topology in 0..3 {
                let mut contract = fixture();
                let circuit = &mut contract.stateful_circuits[0];
                circuit.source = None;
                circuit.return_value = StateReturn::Effectful {
                    body: body(condition.clone(), value.clone(), topology),
                };
                let label = format!("{field_name}/{condition_name}/topology-{topology}");
                assert_included(&contract, &label);
                cases += 1;
            }
        }
    }
    assert_eq!(
        cases, 288,
        "finite corpus, not exhaustive language coverage"
    );
}

#[test]
fn old_admitted_unsigned_boundaries_remain_included() {
    let cases = [
        ("255", "0"),
        ("255", "255"),
        (SMALL_MAX, SMALL_MAX),
        (WIDE_VALUE, "0"),
        (WIDE_VALUE, "1"),
        (WIDE_VALUE, SMALL_MAX),
        (WIDE_VALUE, WIDE_VALUE),
        (WIDE_MAX, WIDE_MAX),
    ];
    for (max, value) in cases {
        let edge = binding(
            "edge",
            Type::Unsigned { max: max.into() },
            unsigned(value, max),
        );
        let field = Expr::Let {
            bindings: vec![edge.clone()],
            body: Box::new(coerce(local("edge"), Type::Field)),
        };
        let condition = Expr::Let {
            bindings: vec![edge],
            body: Box::new(equal(local("edge"), unsigned(value, max))),
        };
        let mut contract = fixture();
        contract.stateful_circuits[0].return_value = StateReturn::Effectful {
            body: body(condition, field, 2),
        };
        assert_included(&contract, &format!("bound-{max}/value-{value}"));
    }
}

#[test]
fn old_family_can_have_a_selected_path_without_a_public_query() {
    let mut contract = fixture();
    contract.stateful_circuits[0].return_value = StateReturn::Effectful {
        body: ReturnPlan::Conditional {
            condition: equal(local("next"), field(0)),
            then: Box::new(ReturnPlan::Let {
                bindings: vec![binding("snapshot", Type::Field, read("state", 0))],
                result: Box::new(ReturnPlan::Sequence {
                    actions: vec![write("state", 0, field(1))],
                    result: Box::new(returned(local("snapshot"))),
                }),
            }),
            otherwise: Box::new(returned(field(0))),
        },
    };
    // This checks compile-time availability only. Preparation still owns the
    // existing EmptyTranscript result if execution selects the empty arm.
    assert_included(&contract, "old-static-effects-with-empty-selected-arm");
}
