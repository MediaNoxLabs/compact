// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Enabled callback at a real DEFAULT-admitted body boundary on 2 MiB.
use super::{NativeSink, field, identity, observe, parameter};
use crate::{
    RenderError,
    ir::{Contract, Expr, PureCircuit, SCHEMA_VERSION, Type},
};

fn contract(depth: usize, missing_leaf: bool) -> Contract {
    let mut body = if missing_leaf {
        parameter("leaf_missing")
    } else {
        field("1")
    };
    for _ in 0..depth {
        body = Expr::Subtract {
            left: Box::new(field("1")),
            right: Box::new(body),
        };
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "probe".into(),
            parameters: vec![],
            result: Type::Field,
            body,
        }],
        stateful_circuits: vec![],
    }
}

#[test]
fn enabled_callback_near_default_and_nested_refusal_on_two_mib_worker() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let depth = (1..=64)
                .take_while(|depth| crate::resource_limits::validate(&contract(*depth, false)).is_ok())
                .last().expect("small Field arithmetic is admitted");
            assert!((3..48).contains(&depth), "unexpected default depth {depth}");
            let accepted = contract(depth, false);
            // Full public acceptance establishes more than private fold support.
            let public_output = crate::render(&accepted).expect("public near-boundary generation");
            assert!(!public_output.is_empty());
            crate::resource_limits::validate(&accepted).unwrap().finish(&accepted).unwrap();
            let mut events = Vec::new();
            let output = {
                let mut callback = |node: &Expr, _: &RenderError| events.push(identity(node));
                observe(&accepted.circuits[0].body,
                    &mut NativeSink::with_refusal_callback(&mut callback))
            }.expect("enabled callback near-boundary fold");
            assert_eq!(output.0, Type::Field);
            assert!(!output.1.is_empty());
            assert!(events.is_empty(), "successful fold reports no refusals");

            let over = contract(depth + 1, false);
            // Refuse before any fold: this is not a bypassed public budget claim.
            assert!(matches!(crate::resource_limits::validate(&over),
                Err(RenderError::ResourceLimit { .. })));
            assert!(matches!(crate::render(&over), Err(RenderError::ResourceLimit { .. })));

            let failing = contract(depth, true);
            crate::resource_limits::validate(&failing).unwrap().finish(&failing).unwrap();
            assert_eq!(crate::render(&failing),
                Err(RenderError::UnknownParameter("leaf_missing".into())));
            let root = &failing.circuits[0].body;
            let mut leaf = root;
            while let Expr::Subtract { right, .. } = leaf { leaf = right; }
            assert!(matches!(leaf, Expr::Parameter { name } if name == "leaf_missing"));
            let mut captured = false;
            let mut first = None;
            let mut available = None;
            let mut events = Vec::new();
            let outcome = {
                let mut callback = |node: &Expr, _: &RenderError| {
                    events.push(identity(node));
                    if !captured {
                        captured = true;
                        first = Some(identity(node));
                        // Parent availability is only a test predicate, no source authority.
                        available = (identity(node) == identity(root)).then_some(identity(node));
                    }
                };
                observe(root, &mut NativeSink::with_refusal_callback(&mut callback))
            };
            assert_eq!(outcome, Err(RenderError::UnknownParameter("leaf_missing".into())));
            assert!(captured);
            assert_eq!(first, Some(identity(leaf)));
            assert_eq!(available, None);
            assert_eq!(events.len(), depth + 1);
            assert_eq!(events.first(), Some(&identity(leaf)));
            assert_eq!(events.last(), Some(&identity(root)));
            eprintln!("Enabled callback worker: depth {depth} public/native accepted; {} preflight refused; deepest child captured first", depth + 1);
        }).unwrap().join().unwrap();
}
