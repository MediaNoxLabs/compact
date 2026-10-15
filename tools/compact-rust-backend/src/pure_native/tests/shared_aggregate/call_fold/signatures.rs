// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn signatures_are_borrowed_raw_and_private_calls_do_not_retype_callee_bodies_or_validate_parameter_names()
 {
    let mut target = declaration("call$raw", vec![Type::Field], Type::Unit);
    target.parameters[0].name = "bad-param".into();
    // Body is an effectful expression by construction. This direct family test
    // preserves signature-only use; public purity/call-graph gates stay separate.
    let circuits = HashMap::from([("call$raw", &target)]);
    let source = Expr::Call {
        name: "call$raw".into(),
        arguments: vec![field("1")],
    };
    assert_eq!(compare(&source, &HashMap::new(), &circuits), Ok(Type::Unit));
    assert_eq!(
        compare(
            &Expr::Call {
                name: "call_raw".into(),
                arguments: vec![field("1")]
            },
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::UnknownCircuit("call_raw".into()))
    );
    let malformed = uint(&"x".repeat(131073));
    let target = declaration("identity", vec![malformed.clone()], Type::Field);
    let circuits = HashMap::from([("identity", &target)]);
    let params = HashMap::from([("value", (&malformed, syn::parse_quote!(value)))]);
    assert_eq!(
        compare(
            &Expr::Call {
                name: "identity".into(),
                arguments: vec![reference("value")]
            },
            &params,
            &circuits
        ),
        Ok(Type::Field)
    );
}
