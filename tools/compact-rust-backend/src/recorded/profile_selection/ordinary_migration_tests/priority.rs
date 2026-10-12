// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn ordinary_preserves_earlier_declared_and_effectful_owners() {
    let c = identity(Type::Field);
    let (_, row) = public(&c, "identity");
    assert_eq!(row["body_owner"], "algebra.membership");
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "algebra.ordinary")
    );
    let mut c = identity(Type::OpaqueBytes);
    c.stateful_circuits[0].return_value = StateReturn::Effectful {
        body: crate::ir::ReturnPlan::Value {
            value: Expr::Parameter {
                name: "value".into(),
            },
        },
    };
    let (_, row) = public(&c, "identity");
    assert_ne!(row["body_owner"], "algebra.ordinary");
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "algebra.ordinary")
    );
}

#[test]
fn ordinary_preserves_unsupported_operation_and_native_error_priority() {
    let mut c = identity(Type::Bytes { length: 32 });
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    };
    assert!(ordinary(&c, "identity").unwrap().is_none());
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = output.unwrap();
    let r = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == "identity")
        .unwrap();
    assert!(!r.recorded);
    assert!(rows.as_array().unwrap().iter().any(|r| {
        r["attempts"]
            .as_array()
            .is_some_and(|a| a.iter().any(|a| a["route"] == "algebra.ordinary"))
    }));
    c.stateful_circuits[0].name = "invalid-name".into();
    let (output, _) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(
        output.is_err(),
        "native identifier validation stays mandatory"
    );
}
