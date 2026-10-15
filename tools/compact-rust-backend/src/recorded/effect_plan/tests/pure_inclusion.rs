// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite, old-first inclusion evidence for terminal-return pure helper grammar.
//! Every positive must first pass the complete legacy admission/type audit.
//! Native-pure extensions (including assertions) are qualified separately.
use super::*;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "7".into() }
}
fn hash(value: Expr) -> Expr {
    Expr::TransientHash {
        value: Box::new(value),
    }
}
fn pure(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body,
    }
}
fn inclusion(label: &str, helpers: &[PureCircuit], entry: &str) {
    let ledger = LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    };
    let root = StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "stored".into(),
            index: 0,
            value: Expr::CellRead {
                field: "stored".into(),
                index: 0,
            },
        }],
        result: Type::Field,
        return_value: StateReturn::Expression {
            value: Expr::Call {
                name: entry.into(),
                arguments: vec![field()],
            },
        },
    };
    let ledger = HashMap::from([("stored", &ledger)]);
    let pure = helpers.iter().map(|c| (c.name.as_str(), c)).collect();
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    assert!(
        crate::recorded::typed_plan::lower_terminal_returns(
            &root, &ledger, &witnesses, &pure, &circuits
        )
        .is_some(),
        "{label}: candidate is not in the old terminal pure domain"
    );
    let checked = check_with_calls(&root, &ledger, &witnesses, &pure, &circuits)
        .unwrap_or_else(|e| panic!("{label}: old-admitted helper refused: {e:?}"));
    render(&checked).unwrap_or_else(|e| panic!("{label}: rendering failed: {e:?}"));
}

#[test]
fn each_old_pure_expression_family_remains_recordable() {
    let pair = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let cases = vec![
        ("parameter", parameter("seed")),
        ("field literal", field()),
        ("boolean", hash(Expr::Boolean { value: true })),
        (
            "bytes literal",
            hash(Expr::BytesLiteral { bytes: vec![1, 2] }),
        ),
        (
            "enum variant",
            hash(Expr::EnumVariant {
                ty: Type::Enum {
                    name: "Choice".into(),
                    variants: vec!["One".into(), "Two".into()],
                },
                variant: "Two".into(),
            }),
        ),
        (
            "default string",
            Expr::DegradeToTransient {
                value: Box::new(Expr::PersistentHash {
                    value: Box::new(Expr::Default {
                        ty: Type::OpaqueString,
                    }),
                }),
            },
        ),
        ("default struct", hash(Expr::Default { ty: pair.clone() })),
        (
            "struct field",
            Expr::StructField {
                value: Box::new(Expr::Default { ty: pair.clone() }),
                field: "value".into(),
                index: 0,
            },
        ),
        (
            "coerce",
            Expr::Coerce {
                value: Box::new(parameter("seed")),
                ty: Type::Field,
            },
        ),
        (
            "persistent hash and degrade",
            Expr::DegradeToTransient {
                value: Box::new(Expr::PersistentHash {
                    value: Box::new(parameter("seed")),
                }),
            },
        ),
        ("transient hash", hash(parameter("seed"))),
        (
            "tuple",
            hash(Expr::Tuple {
                elements: vec![parameter("seed"), field()],
            }),
        ),
        (
            "struct literal",
            hash(Expr::StructLiteral {
                ty: pair,
                fields: vec![parameter("seed")],
            }),
        ),
        (
            "equal",
            hash(Expr::Equal {
                left: Box::new(parameter("seed")),
                right: Box::new(field()),
            }),
        ),
        (
            "transient commit",
            Expr::TransientCommit {
                value: Box::new(parameter("seed")),
                opening: Box::new(field()),
            },
        ),
        (
            "if",
            Expr::If {
                condition: Box::new(Expr::Boolean { value: true }),
                then: Box::new(parameter("seed")),
                otherwise: Box::new(field()),
            },
        ),
        (
            "let",
            Expr::Let {
                bindings: vec![LocalBinding {
                    name: "saved".into(),
                    ty: Type::Field,
                    value: parameter("seed"),
                }],
                body: Box::new(parameter("saved")),
            },
        ),
    ];
    for (label, body) in cases {
        inclusion(label, &[pure("helper", body)], "helper");
    }
}

#[test]
fn real_source_aggregate_nested_calls_and_named_fold_are_in_old_domain() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-pure-calls-schema20-ir.json"
    ))
    .unwrap();
    // digest contains nested calls, aggregate helper interfaces, aggregate If,
    // default Struct -> vector projection, and a named fold callback.
    let mut helpers = contract.circuits;
    helpers.retain(|c| c.name != "checked");
    let digest = helpers.iter_mut().find(|c| c.name == "digest").unwrap();
    // Bind the second argument inside the helper, retaining its source body.
    digest.parameters.pop();
    digest.body = Expr::Let {
        bindings: vec![LocalBinding {
            name: "selected".into(),
            ty: Type::Boolean,
            value: Expr::Boolean { value: true },
        }],
        body: Box::new(digest.body.clone()),
    };
    inclusion("source aggregate/call/fold composition", &helpers, "digest");
}
