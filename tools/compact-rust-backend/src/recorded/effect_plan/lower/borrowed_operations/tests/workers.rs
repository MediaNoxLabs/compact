// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Full native and recorded assertion paths on the unchanged ordinary worker stack.
//! Private IR fixture; not genuine frontend source or proof coverage.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION, StatefulCircuit};
fn fixture(depth: usize, action: bool, malformed: bool) -> Contract {
    let mut left = Expr::FieldLiteral {
        value: if malformed { "bad" } else { "1" }.into(),
    };
    for _ in 0..depth {
        left = Expr::Add {
            left: Box::new(left),
            right: Box::new(Expr::FieldLiteral { value: "1".into() }),
        };
    }
    let condition = Expr::Equal {
        left: Box::new(left),
        right: Box::new(Expr::FieldLiteral { value: "1".into() }),
    };
    let (actions, return_value) = if action {
        (
            vec![StateAction::Assert {
                condition,
                message: "borrowed action".into(),
            }],
            StateReturn::Unit,
        )
    } else {
        (
            vec![],
            StateReturn::Expression {
                value: Expr::Assert {
                    condition: Box::new(condition),
                    message: "original expression".into(),
                },
            },
        )
    };
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        ledger_fields: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "check".into(),
            parameters: vec![],
            actions,
            result: Type::Unit,
            return_value,
        }],
    }
}
fn render(
    source: &Contract,
    trace: Option<&Trace>,
) -> Result<crate::RenderedContract, crate::RenderError> {
    let observer = trace.map(|trace| trace as &dyn RefusalObserver);
    crate::render_with_refusals(source, None, observer, observer)
}
fn bad_child(source: &Contract, action: bool) -> &Expr {
    let circuit = &source.stateful_circuits[0];
    let condition = if action {
        let StateAction::Assert { condition, .. } = &circuit.actions[0] else {
            panic!("fixture")
        };
        condition
    } else {
        let StateReturn::Expression {
            value: Expr::Assert { condition, .. },
        } = &circuit.return_value
        else {
            panic!("fixture")
        };
        condition
    };
    let Expr::Equal { left, .. } = condition else {
        panic!("fixture")
    };
    let mut child = left.as_ref();
    while let Expr::Add { left, .. } = child {
        child = left;
    }
    child
}
#[test]
fn full_borrowed_and_original_assertions_keep_default_two_mib_near_over_and_child() {
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
        for action in [true, false] {
            let near = (1..=64).take_while(|depth| crate::resource_limits::validate(&fixture(*depth, action, false)).is_ok()).last().expect("small assertion accepted");
            assert!((3..48).contains(&near), "actual DEFAULT assertion depth {near}");
            let source = fixture(near, action, false); let trace = Trace::default();
            let observed = render(&source, Some(&trace)).unwrap(); let bare = render(&source, None).unwrap();
            assert_eq!(observed.source, bare.source); assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(), serde_json::to_value(&bare.capabilities).unwrap());
            assert!(observed.capabilities.circuits.iter().find(|c| c.name == "check").unwrap().recorded);
            assert!(trace.primary.borrow().is_none()); assert!(trace.events.borrow().is_empty());
            let over = fixture(near + 1, action, false); let trace = Trace::default();
            let observed = render(&over, Some(&trace)).err().expect("one over DEFAULT");
            assert_eq!(observed, render(&over, None).err().expect("same resource refusal")); assert!(matches!(observed, crate::RenderError::ResourceLimit { .. }));
            let malformed = fixture(near, action, true); let trace = Trace::default();
            let observed = render(&malformed, Some(&trace)).err().expect("malformed original child");
            assert_eq!(observed, render(&malformed, None).err().expect("same malformed child")); assert_eq!(observed, crate::RenderError::InvalidFieldLiteral("bad".into()));
            assert_eq!(*trace.primary.borrow(), Some(event(RefusalSite::node(Owner::Expr(bad_child(&malformed, action))))));
            eprintln!("Assertion action={action}: DEFAULT depth{near} renders/records, depth{} refuses, original malformed child retained on2MiB", near + 1);
        }
    }).unwrap().join().unwrap();
}
