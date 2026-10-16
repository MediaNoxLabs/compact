// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exercise native dual-view stack behavior at the unchanged public envelope.
//! Old repeated typing is not replayed at these deep source boundaries.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Sequence,
    Let,
    Conditional,
}

fn contract(shape: Shape, depth: usize) -> Contract {
    let mut body = Expr::Unit;
    for _ in 0..depth {
        body = match shape {
            Shape::Sequence => Expr::Sequence {
                steps: vec![assertion(boolean(true))],
                value: Box::new(body),
            },
            Shape::Let => local("saved", Type::Boolean, boolean(true), body),
            Shape::Conditional => branch(boolean(true), body, Expr::Unit),
        };
    }
    let mut circuit = pure("probe", Type::Unit);
    circuit.body = body;
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: Vec::new(),
        ledger_fields: Vec::new(),
        constructor: None,
        witnesses: Vec::new(),
        circuits: vec![circuit],
        stateful_circuits: Vec::new(),
    }
}

#[test]
fn ordinary_two_mib_worker_renders_each_public_unit_boundary_and_refuses_one_over() {
    for shape in [Shape::Sequence, Shape::Let, Shape::Conditional] {
        // Select the actual conjunctive support boundary, not a census or an
        // increased allowance. Existing policy/traversal remain independently
        // source-pinned; this probes new rendering at that unchanged envelope.
        let depth = (1..=64)
            .take_while(|depth| crate::resource_limits::validate(&contract(shape, *depth)).is_ok())
            .last()
            .expect("small Unit controls are supported");
        assert!(
            (20..48).contains(&depth),
            "unexpected {shape:?} depth {depth}"
        );
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                assert!(
                    crate::render(&contract(shape, depth)).is_ok(),
                    "{shape:?} depth {depth}"
                );
                assert!(matches!(
                    crate::render(&contract(shape, depth + 1)),
                    Err(RenderError::ResourceLimit { .. })
                ));
                eprintln!(
                    "Unit fragment worker {shape:?}: depth {depth} accepted, {} refused",
                    depth + 1
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
