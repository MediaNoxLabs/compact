// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite composition of borrowed canonical intent operands at existing limits.
use super::*;
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn input() -> StateAction {
    StateAction::Expression {
        value: Expr::CreateZswapInput {
            coin: Box::new(parameter("input")),
        },
    }
}
fn output() -> StateAction {
    StateAction::Expression {
        value: Expr::CreateZswapOutput {
            coin: Box::new(parameter("coin")),
            recipient: Box::new(parameter("recipient")),
        },
    }
}
fn subject(actions: Vec<StateAction>) -> Contract {
    let mut result = contract(read());
    result.ledger_fields.clear();
    let root = &mut result.stateful_circuits[0];
    root.parameters = vec![
        Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        },
        Parameter {
            name: "input".into(),
            ty: crate::coin_shapes::qualified_coin_type(),
        },
        Parameter {
            name: "coin".into(),
            ty: crate::coin_shapes::shielded_coin_type(),
        },
        Parameter {
            name: "recipient".into(),
            ty: crate::coin_shapes::shielded_recipient_type(),
        },
    ];
    root.result = crate::coin_shapes::shielded_coin_type();
    root.return_value = StateReturn::Expression {
        value: parameter("coin"),
    };
    root.actions = actions;
    result
}
fn nested(depth: usize) -> Contract {
    let mut action = output();
    for _ in 0..depth {
        action = StateAction::If {
            condition: parameter("selected"),
            then: Box::new(action),
            otherwise: Box::new(input()),
        };
    }
    subject(vec![action])
}
fn repeated(width: usize) -> Contract {
    subject((0..width).flat_map(|_| [input(), output()]).collect())
}
fn maximum_depth() -> usize {
    let (mut accepted, mut refused) = (0, 1);
    preflight(&nested(0)).unwrap();
    while preflight(&nested(refused)).is_ok() {
        accepted = refused;
        refused *= 2;
    }
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        match preflight(&nested(middle)) {
            Ok(()) => accepted = middle,
            Err(RenderError::ResourceLimit { .. }) => refused = middle,
            Err(error) => panic!("unexpected preflight: {error}"),
        };
    }
    accepted
}
#[test]
fn mixed_intent_boundaries_on_two_mib_worker() {
    isolated(
        "composite::mixed_intent_boundaries_on_two_mib_worker",
        || {
            let depth = maximum_depth();
            let width = maximum_width(repeated);
            eprintln!(
                "composite admitted branch depth={depth}, repeated input/output pairs={width}"
            );
            for (label, make, maximum) in [
                ("intent branches", nested as fn(usize) -> Contract, depth),
                ("repeated intents", repeated, width),
            ] {
                assert!(maximum > 0);
                qualify(&make(maximum), label);
                let next = make(maximum + 1);
                for outcome in [
                    preflight(&next),
                    crate::render_with_capabilities(&next).map(|_| ()),
                ] {
                    assert!(matches!(outcome, Err(RenderError::ResourceLimit { .. })));
                }
            }
        },
    );
}
