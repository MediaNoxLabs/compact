// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite kernel composition at the existing conjunctive preflight boundary.
use super::*;
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn claim() -> StateAction {
    StateAction::Expression {
        value: Expr::KernelClaim {
            claim: KernelClaimKind::Nullifier,
            value: Box::new(parameter("domain")),
        },
    }
}
fn mint() -> StateAction {
    StateAction::Expression {
        value: Expr::KernelMintShielded {
            domain: Box::new(parameter("domain")),
            amount: Box::new(parameter("amount")),
        },
    }
}
fn kernel_contract(actions: Vec<StateAction>) -> Contract {
    let mut result = contract(read());
    result.ledger_fields.clear();
    let root = &mut result.stateful_circuits[0];
    root.parameters = vec![
        Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        },
        Parameter {
            name: "domain".into(),
            ty: Type::Bytes { length: 32 },
        },
        Parameter {
            name: "amount".into(),
            ty: Type::Unsigned {
                max: u64::MAX.to_string(),
            },
        },
    ];
    root.result = Type::Unit;
    root.return_value = StateReturn::Unit;
    root.actions = actions;
    result
}
fn nested(depth: usize) -> Contract {
    let mut action = mint();
    for _ in 0..depth {
        action = StateAction::If {
            condition: parameter("selected"),
            then: Box::new(action),
            otherwise: Box::new(claim()),
        };
    }
    kernel_contract(vec![action])
}
fn repeated(width: usize) -> Contract {
    kernel_contract((0..width).flat_map(|_| [claim(), mint()]).collect())
}
fn maximum_depth() -> usize {
    let (mut accepted, mut refused) = (0, 1);
    preflight(&nested(accepted)).unwrap();
    while preflight(&nested(refused)).is_ok() {
        accepted = refused;
        refused *= 2;
    }
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        match preflight(&nested(middle)) {
            Ok(()) => accepted = middle,
            Err(RenderError::ResourceLimit { .. }) => refused = middle,
            Err(error) => panic!("unexpected kernel preflight error: {error}"),
        }
    }
    accepted
}
#[test]
fn mixed_kernel_boundaries_on_two_mib_worker() {
    isolated("kernel::mixed_kernel_boundaries_on_two_mib_worker", || {
        let depth = maximum_depth();
        let width = maximum_width(repeated);
        eprintln!("kernel admitted branch depth={depth}, repeated claim/mint pairs={width}");
        for (label, make, maximum) in [
            (
                "nested kernel branches",
                nested as fn(usize) -> Contract,
                depth,
            ),
            ("repeated claim/mint pairs", repeated, width),
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
    });
}
