// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// The direct VectorFoldCall child is a native-valid recording refusal; fixed Field
// commitments themselves are supported. Do not replace this with malformed types.
use super::*;

#[test]
fn ordinary_consolidates_declared_interfaces_and_preserves_effectful_bypass() {
    let c = identity(Type::Field);
    let (_, row) = public(&c, "identity");
    assert_eq!(row["body_owner"], "algebra.ordinary");
    let attempts = row["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0]["route"], "algebra.ordinary");
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
    crate::recorded::test_support::register_unsupported_fold(&mut c);
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(crate::recorded::test_support::unsupported_fixed_input()),
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
