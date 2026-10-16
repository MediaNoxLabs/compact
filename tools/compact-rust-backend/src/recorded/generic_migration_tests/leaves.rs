// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Remaining ordered binding alternatives and their actual helper authority.
use super::*;

fn pure(name: &str, parameters: Vec<Parameter>, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        internal: true,
        name: name.into(),
        parameters,
        result,
        body,
    }
}
fn bind_then_write(c: &mut Contract, ty: Type, value: Expr) {
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![binding("selected", ty, value)],
        action: Box::new(write()),
    }];
}
fn shared(c: &Contract) -> HashSet<String> {
    let cx = Context::new(c);
    generic_reference::plan_recorded_helpers(
        &c.stateful_circuits,
        &cx.ledger,
        &cx.witnesses,
        &cx.pure,
        &cx.circuits,
    )
    .unwrap()
    .0
}

#[test]
fn complete_remaining_binding_witness_and_helper_leaf_alternatives() {
    conditional_and_public_cast();
    witness_binding_types_and_discarded_unit();
    field_and_pair_hash_helpers();
    bytes_arguments_vectors_and_final_opaque_gate();
}

fn conditional_and_public_cast() {
    let small = Type::Unsigned { max: "4".into() };
    let cast = Expr::UnsignedCast {
        max: u64::MAX.to_string(),
        value: Box::new(Expr::If {
            condition: Box::new(var("choose")),
            then: Box::new(Expr::Coerce {
                ty: small.clone(),
                value: Box::new(Expr::UnsignedLiteral {
                    value: "1".into(),
                    max: "4".into(),
                }),
            }),
            otherwise: Box::new(Expr::Coerce {
                ty: small,
                value: Box::new(Expr::UnsignedLiteral {
                    value: "2".into(),
                    max: "4".into(),
                }),
            }),
        }),
    };
    let mut c = unit(vec![StateAction::Let {
        bindings: vec![binding("selected", uint64(), cast)],
        action: Box::new(StateAction::CellWrite {
            field: "cell".into(),
            index: 0,
            value: var("selected"),
        }),
    }]);
    c.stateful_circuits[0].parameters = vec![parameter("choose", Type::Boolean)];
    slot(&mut c, "cell", LedgerFieldKind::Cell { ty: uint64() });
    included(
        &c,
        "root",
        "generic Uint64 conditional rather than specialized preflight",
    );
    let mut c = cell_contract();
    c.stateful_circuits[0].parameters = vec![
        parameter("small", Type::Unsigned { max: "255".into() }),
        parameter(
            "large",
            Type::Unsigned {
                max: "4294967295".into(),
            },
        ),
    ];
    c.stateful_circuits[0].actions.insert(
        0,
        StateAction::Assert {
            condition: Expr::Equal {
                left: Box::new(Expr::UnsignedCast {
                    max: "4294967295".into(),
                    value: Box::new(var("small")),
                }),
                right: Box::new(var("large")),
            },
            message: "widen first".into(),
        },
    );
    included(
        &c,
        "root",
        "unshadowed public Uint8 to Uint32 assertion then write",
    );
}

fn witness_binding_types_and_discarded_unit() {
    let product = Type::Struct {
        name: "WitnessValue".into(),
        fields: vec![
            StructField {
                name: "field".into(),
                ty: Type::Field,
            },
            StructField {
                name: "flag".into(),
                ty: Type::Boolean,
            },
        ],
    };
    for ty in [
        Type::Boolean,
        Type::Field,
        Type::Bytes { length: 32 },
        Type::Unsigned { max: "255".into() },
        product,
    ] {
        let mut c = cell_contract();
        // A declared identity argument also exercises the recordable Struct
        // argument restriction, not just zero-argument witness declarations.
        c.stateful_circuits[0].parameters = vec![parameter("input", ty.clone())];
        c.witnesses.push(WitnessDeclaration {
            source: None,
            name: "fetch".into(),
            parameters: vec![parameter("arg", ty.clone())],
            result: ty.clone(),
        });
        bind_then_write(
            &mut c,
            ty.clone(),
            Expr::WitnessCall {
                name: "fetch".into(),
                arguments: vec![var("input")],
            },
        );
        let plan = included(&c, "root", "declared witness binding type and argument");
        let calls = methods(&plan);
        assert_eq!(
            calls.iter().filter(|m| *m == "try_witness_metered").count(),
            1
        );
        assert!(
            calls
                .iter()
                .position(|m| m == "try_witness_metered")
                .unwrap()
                < calls.iter().position(|m| m == "record_write").unwrap()
        );
        let mut wrong = c.clone();
        wrong.witnesses[0].result = Type::Unit;
        assert!(
            crate::render_with_capabilities(&wrong).is_err(),
            "declared result authority for {ty:?}"
        );
    }
    let mut c = cell_contract();
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "notice".into(),
        parameters: vec![],
        result: Type::Unit,
    });
    c.stateful_circuits[0].actions.insert(
        0,
        StateAction::Expression {
            value: Expr::WitnessCall {
                name: "notice".into(),
                arguments: vec![],
            },
        },
    );
    included(&c, "root", "discarded Unit witness before public write");
}

fn field_and_pair_hash_helpers() {
    let mut c = cell_contract();
    c.stateful_circuits.push(StatefulCircuit {
        source: None,
        internal: true,
        name: "readValue".into(),
        parameters: vec![],
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: Expr::CellRead {
                field: "cell".into(),
                index: 0,
            },
        },
    });
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![binding(
            "selected",
            Type::Field,
            Expr::Call {
                name: "readValue".into(),
                arguments: vec![],
            },
        )],
        action: Box::new(StateAction::CellWrite {
            field: "cell".into(),
            index: 0,
            value: var("selected"),
        }),
    }];
    assert!(shared(&c).contains("readValue"));
    included(&c, "root", "actual shared Field helper binding");

    let pair = Type::Vector {
        length: 2,
        element: Box::new(Type::Field),
    };
    let argument = Expr::Coerce {
        ty: pair.clone(),
        value: Box::new(Expr::Tuple {
            elements: vec![field(0), field(1)],
        }),
    };
    let mut c = cell_contract();
    c.circuits.push(pure(
        "hashPair",
        vec![parameter("pair", pair.clone())],
        Type::Field,
        Expr::TransientHash {
            value: Box::new(var("pair")),
        },
    ));
    bind_then_write(
        &mut c,
        Type::Field,
        Expr::Call {
            name: "hashPair".into(),
            arguments: vec![argument.clone()],
        },
    );
    included(&c, "root", "direct closed pure pair hash Field binding");
    c.circuits.push(pure(
        "forwardPair",
        vec![parameter("pair", pair.clone())],
        Type::Field,
        Expr::Call {
            name: "hashPair".into(),
            arguments: vec![var("pair")],
        },
    ));
    bind_then_write(
        &mut c,
        Type::Field,
        Expr::Call {
            name: "forwardPair".into(),
            arguments: vec![argument.clone()],
        },
    );
    included(&c, "root", "transitive closed pair hash binding");

    c.stateful_circuits.push(StatefulCircuit {
        source: None,
        internal: true,
        name: "hashAndRead".into(),
        parameters: vec![parameter("pair", pair.clone())],
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: Expr::Add {
                left: Box::new(Expr::Call {
                    name: "hashPair".into(),
                    arguments: vec![Expr::Coerce {
                        value: Box::new(var("pair")),
                        ty: pair,
                    }],
                }),
                right: Box::new(Expr::CellRead {
                    field: "cell".into(),
                    index: 0,
                }),
            },
        },
    });
    bind_then_write(
        &mut c,
        Type::Field,
        Expr::Call {
            name: "hashAndRead".into(),
            arguments: vec![argument],
        },
    );
    // This historical bridge is admitted at the binding. Its helper need not be
    // independently shared: never invent a planner entry to make the law pass.
    assert!(!shared(&c).contains("hashAndRead"));
    let plan = included(
        &c,
        "root",
        "internal pair hash then Field read specialized bridge",
    );
    let calls = methods(&plan);
    assert!(
        calls.iter().position(|m| m == "record_read").unwrap()
            < calls.iter().position(|m| m == "record_write").unwrap()
    );

    // Separate closed scalar Field-call branch in append_field_binding.
    c.circuits.push(pure(
        "scalar",
        vec![parameter("x", Type::Field)],
        Type::Field,
        Expr::Multiply {
            left: Box::new(var("x")),
            right: Box::new(field(2)),
        },
    ));
    bind_then_write(
        &mut c,
        Type::Field,
        Expr::Call {
            name: "scalar".into(),
            arguments: vec![field(1)],
        },
    );
    included(&c, "root", "closed scalar pure Field binding");
}

fn bytes_arguments_vectors_and_final_opaque_gate() {
    let bytes = Type::Bytes { length: 32 };
    let mut c = cell_contract();
    c.stateful_circuits[0].parameters = vec![parameter("input", bytes.clone())];
    c.circuits.push(pure(
        "bytesIdentity",
        vec![parameter("x", bytes.clone())],
        bytes.clone(),
        var("x"),
    ));
    bind_then_write(
        &mut c,
        bytes.clone(),
        Expr::Call {
            name: "bytesIdentity".into(),
            arguments: vec![Expr::Coerce {
                value: Box::new(var("input")),
                ty: bytes,
            }],
        },
    );
    included(
        &c,
        "root",
        "Bytes pure call with declared explicit Coerce argument",
    );
    // Plain static Vector already covered in lexical.rs; add the distinct
    // historical identity-map source branch of cell_source.
    let small = Type::Unsigned { max: "255".into() };
    let vector = Type::Vector {
        length: 2,
        element: Box::new(small.clone()),
    };
    let mut c = cell_contract();
    bind_then_write(
        &mut c,
        vector,
        Expr::VectorMap {
            source: Box::new(Expr::Tuple {
                elements: vec![
                    Expr::UnsignedLiteral {
                        value: "1".into(),
                        max: "255".into(),
                    },
                    Expr::UnsignedLiteral {
                        value: "2".into(),
                        max: "255".into(),
                    },
                ],
            }),
            parameter: parameter("x", small.clone()),
            body: Box::new(var("x")),
            result: small,
            length: 2,
        },
    );
    included(
        &c,
        "root",
        "static VectorMap identity source with complete continuation",
    );
    let mut c = cell_contract();
    c.stateful_circuits[0].parameters = vec![parameter("opaque", Type::OpaqueString)];
    bind_then_write(&mut c, Type::OpaqueString, var("opaque"));
    assert!(matches!(
        Context::new(&c).old(&c, "root"),
        Ok(RecordingOutcome::Unsupported(_))
    ));
    assert!(Context::new(&c).checked("root").is_ok());
    assert!(
        crate::render_with_capabilities(&c)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}
