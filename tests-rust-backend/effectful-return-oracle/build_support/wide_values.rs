// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite, IR-only execution probes. These are not compiler-source/proof fixtures.
use compact_rust_backend::ir::{
    Contract, Expr, LocalBinding, Parameter, ReturnPlan, StateAction, StateReturn, Type,
    WitnessDeclaration,
};

const MAXIMUM: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const ABOVE_U128: &str = "340282366920938463463374607431768211457";
const NEXT_ABOVE_U128: &str = "340282366920938463463374607431768211458";

fn local(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}

fn field(name: &str) -> Expr {
    Expr::Coerce {
        value: Box::new(local(name)),
        ty: Type::Field,
    }
}

fn literal_binding(name: &str, value: &str) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty: Type::Unsigned {
            max: MAXIMUM.into(),
        },
        value: Expr::UnsignedLiteral {
            value: value.into(),
            max: MAXIMUM.into(),
        },
    }
}

fn observed_return() -> ReturnPlan {
    ReturnPlan::Sequence {
        actions: vec![StateAction::CellWrite {
            field: "unused".into(),
            index: 1,
            value: Expr::WitnessCall {
                name: "observe".into(),
                arguments: vec![field("right")],
            },
        }],
        result: Box::new(ReturnPlan::Value {
            value: Expr::Add {
                left: Box::new(local("before")),
                right: Box::new(field("right")),
            },
        }),
    }
}

fn body(left: &str, right: &str) -> ReturnPlan {
    ReturnPlan::Let {
        bindings: vec![
            LocalBinding {
                name: "before".into(),
                ty: Type::Field,
                value: Expr::CellRead {
                    field: "state".into(),
                    index: 0,
                },
            },
            literal_binding("left", left),
            literal_binding("right", right),
        ],
        result: Box::new(ReturnPlan::Sequence {
            actions: vec![StateAction::CellWrite {
                field: "state".into(),
                index: 0,
                value: field("left"),
            }],
            result: Box::new(ReturnPlan::Conditional {
                condition: Expr::Equal {
                    left: Box::new(local("left")),
                    right: Box::new(local("right")),
                },
                then: Box::new(observed_return()),
                otherwise: Box::new(ReturnPlan::Conditional {
                    condition: Expr::Equal {
                        left: Box::new(local("next")),
                        right: Box::new(Expr::FieldLiteral { value: "0".into() }),
                    },
                    then: Box::new(ReturnPlan::Value {
                        value: local("before"),
                    }),
                    otherwise: Box::new(observed_return()),
                }),
            }),
        }),
    }
}

pub fn contract(original: &Contract) -> Contract {
    let mut contract = original.clone();
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "observe".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
    });
    let template = original
        .stateful_circuits
        .iter()
        .find(|circuit| circuit.name == "choose")
        .unwrap();
    for (name, left, right) in [
        ("wide_equal", ABOVE_U128, ABOVE_U128),
        ("wide_unequal", ABOVE_U128, NEXT_ABOVE_U128),
        ("wide_maximum", MAXIMUM, MAXIMUM),
    ] {
        let mut circuit = template.clone();
        circuit.name = name.into();
        circuit.source = None;
        circuit.return_value = StateReturn::Effectful {
            body: body(left, right),
        };
        contract.stateful_circuits.push(circuit);
    }
    contract
}
