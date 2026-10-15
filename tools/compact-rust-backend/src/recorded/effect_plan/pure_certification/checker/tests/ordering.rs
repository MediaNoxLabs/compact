// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

fn bad_type() -> Type {
    Type::Unsigned {
        max: "not-a-maximum".into(),
    }
}
fn bad_maximum() -> RenderError {
    RenderError::InvalidUnsignedMaximum("not-a-maximum".into())
}

#[test]
fn formal_name_carrier_duplicate_result_and_body_first_refusals_match_old_bridge() {
    let mut circuit = source(
        vec![parameter("!", bad_type())],
        bad_type(),
        reference("missing"),
    );
    circuit.name = "!".into();
    refusal(&circuit, RenderError::InvalidIdentifier("!".into()));
    circuit.name = "checked".into();
    refusal(&circuit, RenderError::InvalidIdentifier("!".into()));
    circuit.parameters = vec![parameter("x", Type::Field), parameter("x", bad_type())];
    refusal(&circuit, bad_maximum());
    circuit.parameters[1].ty = Type::Field;
    refusal(&circuit, RenderError::DuplicateParameter("x".into()));
    circuit.parameters.pop();
    refusal(&circuit, bad_maximum());
    circuit.result = Type::Boolean;
    refusal(&circuit, RenderError::UnknownParameter("missing".into()));
    circuit.body = field();
    refusal(
        &circuit,
        RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        },
    );
}

#[test]
fn all_control_first_errors_check_children_before_later_obligations() {
    let missing = || reference("first");
    let cases = [
        Expr::Assert {
            condition: Box::new(missing()),
            message: "assert".into(),
        },
        Expr::Sequence {
            steps: vec![missing()],
            value: Box::new(effect()),
        },
        branch(missing(), effect(), reference("later")),
        local("valid", bad_type(), missing(), effect()),
    ];
    for body in cases {
        refusal(
            &source(vec![], Type::Field, body),
            RenderError::UnknownParameter("first".into()),
        );
    }
    refusal(
        &source(
            vec![],
            Type::Field,
            local("!", bad_type(), missing(), effect()),
        ),
        RenderError::InvalidIdentifier("!".into()),
    );
    refusal(
        &source(vec![], Type::Field, branch(field(), missing(), effect())),
        RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        },
    );
    refusal(
        &source(
            vec![],
            Type::Field,
            Expr::Sequence {
                steps: vec![field()],
                value: Box::new(missing()),
            },
        ),
        RenderError::TypeMismatch {
            expected: Type::Unit,
            actual: Type::Field,
        },
    );
}
