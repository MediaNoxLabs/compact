// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_lexical_preflights_bindings_snapshot_and_discarded_reads() {
    // Historical typed IR fixtures: schema is explicitly projected as in their
    // maintained render tests; this does not claim fresh frontend generation.
    for (label, text) in [
        (
            "nested_uint64",
            include_str!("../../../fixtures/recorded-nested-uint64-cell-counter.json"),
        ),
        (
            "conditional_pair",
            include_str!("../../../fixtures/recorded-closed-conditional-field-pair.json"),
        ),
        (
            "annotated_uint8",
            include_str!("../../../fixtures/recorded-annotated-uint8-conditional.json"),
        ),
        (
            "curve_argument",
            include_str!("../../../fixtures/recorded-closed-curve-argument.json"),
        ),
        (
            "nested_uint4",
            include_str!("../../../fixtures/recorded-nested-uint4.json"),
        ),
        (
            "pure_struct_projection",
            include_str!("../../../fixtures/recorded-struct-constructor-cells.json"),
        ),
    ] {
        let c = parse_fixture(text);
        for circuit in c.stateful_circuits.iter().filter(|r| !r.internal) {
            included(&c, &circuit.name, label);
        }
    }
    let mut c = cell_contract();
    let record = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let cast = Expr::FieldCast {
        value: Box::new(Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(Expr::Coerce {
                value: Box::new(Expr::UnsignedLiteral {
                    value: "1".into(),
                    max: "2".into(),
                }),
                ty: Type::Unsigned { max: "2".into() },
            }),
            otherwise: Box::new(Expr::Coerce {
                value: Box::new(Expr::UnsignedLiteral {
                    value: "2".into(),
                    max: "2".into(),
                }),
                ty: Type::Unsigned { max: "2".into() },
            }),
        }),
    };
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![binding(
            "r",
            record.clone(),
            Expr::StructLiteral {
                ty: record,
                fields: vec![cast],
            },
        )],
        action: Box::new(StateAction::Let {
            bindings: vec![binding(
                "v",
                Type::Field,
                Expr::StructField {
                    value: Box::new(var("r")),
                    field: "value".into(),
                    index: 0,
                },
            )],
            action: Box::new(StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: var("v"),
            }),
        }),
    }];
    included(&c, "root", "literal struct projection seventh preflight");

    let mut c = cell_contract();
    c.stateful_circuits[0].result = Type::Field;
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: var("saved"),
    };
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![binding(
            "saved",
            Type::Field,
            Expr::CellRead {
                field: "cell".into(),
                index: 0,
            },
        )],
        action: Box::new(write()),
    }];
    let plan = included(&c, "root", "root prewrite snapshot");
    assert_eq!(
        methods(&plan)
            .iter()
            .filter(|v| v.starts_with("record_"))
            .cloned()
            .collect::<Vec<_>>(),
        vec!["record_read", "record_write"]
    );
    let mut wrong = c.clone();
    if let StateAction::Let { bindings, .. } = &mut wrong.stateful_circuits[0].actions[0] {
        bindings[0].name = "other".into();
    }
    assert!(crate::render_with_capabilities(&wrong).is_err());
    c.stateful_circuits[0].result = Type::Unit;
    c.stateful_circuits[0].actions.clear();
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Let {
            bindings: vec![
                binding(
                    "first",
                    Type::Field,
                    Expr::CellRead {
                        field: "cell".into(),
                        index: 0,
                    },
                ),
                binding(
                    "second",
                    Type::Field,
                    Expr::Add {
                        left: Box::new(var("first")),
                        right: Box::new(Expr::CellRead {
                            field: "cell".into(),
                            index: 0,
                        }),
                    },
                ),
            ],
            body: Box::new(Expr::Unit),
        },
    };
    assert_eq!(
        methods(&included(&c, "root", "discarded ordered reads"))
            .iter()
            .filter(|v| *v == "record_read")
            .count(),
        2
    );

    // The generic loop is independently exercised, not absorbed by preflights.
    let bytes = Type::Bytes { length: 32 };
    let unsigned = Type::Unsigned {
        max: "65535".into(),
    };
    let enumeration = Type::Enum {
        name: "Choice".into(),
        variants: vec!["A".into(), "B".into()],
    };
    let vector = Type::Vector {
        length: 2,
        element: Box::new(Type::Field),
    };
    let mut c = cell_contract();
    slot(
        &mut c,
        "opening",
        LedgerFieldKind::Cell { ty: bytes.clone() },
    );
    c.stateful_circuits[0].parameters = vec![parameter("bytes", bytes.clone())];
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "readFlag".into(),
        parameters: vec![],
        result: Type::Boolean,
    });
    let pair = Expr::Tuple {
        elements: vec![field(1), field(2)],
    };
    let values = vec![
        binding(
            "ph",
            bytes.clone(),
            Expr::PersistentHash {
                value: Box::new(pair.clone()),
            },
        ),
        binding(
            "th",
            Type::Field,
            Expr::TransientHash {
                value: Box::new(pair),
            },
        ),
        binding(
            "small",
            unsigned,
            Expr::UnsignedLiteral {
                value: "1".into(),
                max: "65535".into(),
            },
        ),
        binding(
            "index",
            uint64(),
            Expr::UnsignedLiteral {
                value: "1".into(),
                max: u64::MAX.to_string(),
            },
        ),
        binding("value", Type::Field, field(2)),
        binding("flag", Type::Boolean, Expr::Boolean { value: true }),
        binding(
            "choice",
            enumeration.clone(),
            Expr::EnumVariant {
                ty: enumeration,
                variant: "A".into(),
            },
        ),
        binding(
            "witnessed",
            Type::Boolean,
            Expr::WitnessCall {
                name: "readFlag".into(),
                arguments: vec![],
            },
        ),
        binding(
            "commit",
            bytes.clone(),
            Expr::PersistentCommit {
                value: Box::new(field(1)),
                opening: Box::new(Expr::CellRead {
                    field: "opening".into(),
                    index: 1,
                }),
            },
        ),
        binding(
            "literal",
            bytes.clone(),
            Expr::BytesLiteral { bytes: vec![0; 32] },
        ),
        binding("aliased", bytes, var("bytes")),
        binding(
            "vector",
            vector,
            Expr::Vector {
                element: Type::Field,
                elements: vec![field(1), field(2)],
            },
        ),
    ];
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: values,
        action: Box::new(write()),
    }];
    included(&c, "root", "generic ordered value categories");
}
