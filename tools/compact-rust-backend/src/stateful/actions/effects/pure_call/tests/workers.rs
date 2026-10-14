// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Full native/recorded artifact assembly, not only the borrowed call helper.
use crate::{
    RenderError,
    ir::{
        Contract, Expr, Parameter, PureCircuit, SCHEMA_VERSION, StateAction, StateReturn,
        StatefulCircuit,
    },
    source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot},
};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(Option<usize>, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let pointer = match site.owner {
            Some(Owner::Expr(source)) => Some(std::ptr::from_ref(source) as usize),
            None => None,
            _ => panic!("unexpected owner in this single borrowed-argument fixture"),
        };
        self.0.borrow_mut().push((pointer, site.slot));
    }
}
fn fixture(depth: usize) -> Contract {
    let mut value = Expr::FieldLiteral { value: "1".into() };
    for _ in 0..depth {
        value = Expr::Add {
            left: Box::new(value),
            right: Box::new(Expr::FieldLiteral { value: "1".into() }),
        };
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            source: None,
            name: "consume".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "input".into(),
                ty: crate::ir::Type::Field,
            }],
            result: crate::ir::Type::Unit,
            body: Expr::Unit,
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "call_pure".into(),
            internal: false,
            parameters: vec![],
            actions: vec![StateAction::PureCall {
                name: "consume".into(),
                arguments: vec![value],
            }],
            result: crate::ir::Type::Unit,
            return_value: StateReturn::Unit,
        }],
    }
}

#[test]
fn full_borrowed_pure_call_keeps_default_two_mib_boundary_and_original_leaf() {
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
        // Same bounded search as the retained full effects worker. DEFAULT's
        // syntax-depth cap is inside this envelope; an escape fails the law.
        let depth = (1..=64).take_while(|depth|
            crate::resource_limits::validate(&fixture(*depth)).is_ok())
            .last().expect("small pure action accepted");
        assert!((3..48).contains(&depth), "unexpected DEFAULT pure-action depth{depth}");
        let near = fixture(depth);
        let trace = Trace::default();
        let observed = crate::render_with_native_refusal(&near, None, Some(&trace)).unwrap();
        let bare = crate::render_with_capabilities(&near).unwrap();
        assert_eq!(observed.source, bare.source);
        assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(), serde_json::to_value(&bare.capabilities).unwrap());
        assert!(trace.0.borrow().is_empty());

        let over = fixture(depth + 1);
        let trace = Trace::default();
        let observed = crate::render_with_native_refusal(&over, None, Some(&trace)).err().expect("one over refuses");
        let bare = crate::render_with_capabilities(&over).err().expect("same original boundary refuses");
        assert!(matches!(observed, RenderError::ResourceLimit { .. }));
        assert_eq!(observed, bare);
        // Summary/resource checks may provide explicit unavailable (None);
        // never manufacture an argument range to satisfy this boundary.

        let mut malformed = fixture(depth);
        let StateAction::PureCall { arguments, .. } = &mut malformed.stateful_circuits[0].actions[0] else { panic!("fixture") };
        let mut leaf = &mut arguments[0];
        while let Expr::Add { left, .. } = leaf { leaf = left; }
        *leaf = Expr::FieldLiteral { value: "bad".into() };
        let pointer = std::ptr::from_ref(leaf) as usize;
        let trace = Trace::default();
        let observed = crate::render_with_native_refusal(&malformed, None, Some(&trace)).err().expect("actual malformed original leaf");
        let bare = crate::render_with_capabilities(&malformed).err().expect("same original refusal");
        assert_eq!(observed, bare);
        assert_eq!(observed, RenderError::InvalidFieldLiteral("bad".into()));
        assert_eq!(trace.0.borrow().first(), Some(&(Some(pointer), Slot::Node)));
        eprintln!("Full borrowed PureCall DEFAULT depth{depth} accepted/{} refused, malformed original leaf, Some/None artifact/capability equality on2MiB", depth + 1);
    }).unwrap().join().unwrap();
}
