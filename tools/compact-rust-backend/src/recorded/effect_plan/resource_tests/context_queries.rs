// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Public preflight boundaries for the two new context primitives.
use super::*;

fn address_bytes() -> Expr {
    Expr::StructField {
        value: Box::new(Expr::KernelSelf {
            ty: crate::ir::contract_address_type(),
        }),
        field: "bytes".into(),
        index: 0,
    }
}
fn bytes() -> Expr {
    Expr::BytesLiteral { bytes: vec![0; 32] }
}
fn context_contract(value: Expr) -> Contract {
    let mut result = contract(read());
    result.ledger_fields.clear();
    let circuit = &mut result.stateful_circuits[0];
    circuit.result = Type::Bytes { length: 32 };
    circuit.return_value = StateReturn::Expression { value };
    result
}
fn nested_commitments(depth: usize) -> Contract {
    let mut value = address_bytes();
    for _ in 0..depth {
        value = Expr::PersistentCommit {
            value: Box::new(Expr::Tuple {
                elements: vec![value, address_bytes()],
            }),
            opening: Box::new(bytes()),
        };
    }
    context_contract(value)
}
fn repeated_reads(width: usize) -> Contract {
    context_contract(Expr::Let {
        bindings: (0..width)
            .map(|i| LocalBinding {
                name: format!("address_{i}"),
                ty: Type::Bytes { length: 32 },
                value: address_bytes(),
            })
            .collect(),
        body: Box::new(address_bytes()),
    })
}
fn maximum_depth() -> usize {
    let (mut accepted, mut refused) = (0, 1);
    preflight(&nested_commitments(accepted)).unwrap();
    while preflight(&nested_commitments(refused)).is_ok() {
        accepted = refused;
        refused *= 2;
    }
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        match preflight(&nested_commitments(middle)) {
            Ok(()) => accepted = middle,
            Err(RenderError::ResourceLimit { .. }) => refused = middle,
            Err(error) => panic!("unexpected commitment preflight error: {error}"),
        }
    }
    accepted
}

#[test]
fn context_boundaries_on_two_mib_worker() {
    isolated(
        "context_queries::context_boundaries_on_two_mib_worker",
        || {
            let depth = maximum_depth();
            let width = maximum_width(repeated_reads);
            eprintln!("context admitted commitment depth={depth}, repeated read width={width}");
            for (label, make, maximum) in [
                (
                    "nested pair commitments",
                    nested_commitments as fn(usize) -> Contract,
                    depth,
                ),
                ("repeated kernel reads", repeated_reads, width),
            ] {
                assert!(maximum > 0);
                qualify(&make(maximum), label);
                let next = make(maximum + 1);
                for result in [
                    preflight(&next),
                    crate::render_with_capabilities(&next).map(|_| ()),
                ] {
                    assert!(matches!(result, Err(RenderError::ResourceLimit { .. })));
                }
            }
        },
    );
}
