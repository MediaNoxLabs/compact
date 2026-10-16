// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn raw_signature_lookup_arity_and_each_argument_check_precede_name_validation() {
    let target = declaration("bad-name", vec![Type::Field, Type::Field], Type::Field);
    let circuits = HashMap::from([("bad-name", &target)]);
    assert_eq!(
        compare(
            &Expr::Call {
                name: "missing".into(),
                arguments: vec![reference("missing")]
            },
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::UnknownCircuit("missing".into()))
    );
    assert_eq!(
        compare(
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![reference("missing")]
            },
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::ArgumentCount {
            circuit: "bad-name".into(),
            expected: 2,
            actual: 1
        })
    );
    assert_eq!(
        compare(
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![boolean(true), reference("missing")]
            },
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![field("1"), reference("missing")]
            },
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        compare(
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![field("1"), field("2")]
            },
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
}
