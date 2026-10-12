// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn malformed_declarations_and_operands_fail_native_validation_before_selection() {
    for mode in [
        "missing",
        "kind",
        "index",
        "operand",
        "result",
        "layout",
        "identifier",
    ] {
        let mut c = fixture();
        match mode {
            "missing" => c.ledger_fields.clear(),
            "kind" => c.ledger_fields[0].declaration = LedgerFieldKind::Counter,
            "index" => {
                if let StateReturn::SetMember { index, .. } =
                    &mut root(&mut c, "hasName").return_value
                {
                    *index = 99
                }
            }
            "operand" => {
                if let StateReturn::SetMember { value, .. } =
                    &mut root(&mut c, "hasName").return_value
                {
                    *value = parameter("missing")
                }
            }
            "result" => root(&mut c, "hasName").result = Type::Field,
            "layout" => c.ledger_fields[0].path = vec![2, 7],
            "identifier" => root(&mut c, "hasName").name = "bad root".into(),
            _ => unreachable!(),
        }
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "{mode}");
        assert!(rows.as_array().unwrap().is_empty(), "native first {mode}");
    }
}
#[test]
fn neighboring_interfaces_and_unsupported_compositions_keep_distinct_boundaries() {
    for (parameters, result) in [
        (vec![Type::OpaqueBytes], Type::Boolean),
        (vec![Type::OpaqueString, Type::OpaqueString], Type::Boolean),
        (vec![Type::OpaqueString], Type::Field),
    ] {
        let mut c = fixture();
        let r = root(&mut c, "hasName");
        r.parameters = parameters
            .into_iter()
            .enumerate()
            .map(|(i, ty)| Parameter {
                name: format!("arg{i}"),
                ty,
            })
            .collect();
        r.result = result.clone();
        r.return_value = StateReturn::Expression {
            value: if result == Type::Boolean {
                Expr::Boolean { value: true }
            } else {
                Expr::FieldLiteral { value: "1".into() }
            },
        };
        let cx = Context::new(&c);
        assert!(cx.domain(cx.circuits["hasName"]).unwrap().is_none());
        crate::render_with_capabilities(&c).unwrap();
    }
    let mut c = fixture();
    root(&mut c, "hasName").actions = vec![StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "0".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    }];
    let cx = Context::new(&c);
    assert!(cx.domain(cx.circuits["hasName"]).unwrap().is_none());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == "hasName" && !r.recorded && !r.observed_call)
    );
}
#[test]
fn recursive_calls_and_invalid_unselected_branches_do_not_acquire_a_permit() {
    for recursive in [false, true] {
        let mut c = fixture();
        if recursive {
            root(&mut c, "addName").actions = vec![StateAction::CircuitCall {
                name: "addName".into(),
                arguments: vec![parameter("name")],
            }];
            root(&mut c, "hasName").actions = vec![StateAction::CircuitCall {
                name: "addName".into(),
                arguments: vec![parameter("name")],
            }];
        } else {
            root(&mut c, "hasName").return_value = StateReturn::Expression {
                value: Expr::If {
                    condition: Box::new(Expr::Boolean { value: false }),
                    then: Box::new(Expr::FieldLiteral { value: "1".into() }),
                    otherwise: Box::new(Expr::Boolean { value: true }),
                },
            };
        }
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err());
        assert!(rows.as_array().unwrap().is_empty());
    }
}
