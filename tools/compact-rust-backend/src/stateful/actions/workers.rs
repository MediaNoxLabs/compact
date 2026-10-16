// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Full native Cell action assembly, including its enclosing syntax, on 2 MiB.
use crate::{
    RenderError,
    ir::{
        Contract, Expr, LedgerField, LedgerFieldKind, SCHEMA_VERSION, StateAction, StateReturn,
        StatefulCircuit, Type,
    },
    source_diagnostics::{Owner, RefusalObserver, RefusalSite},
};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<usize>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        if let Some(Owner::Expr(value)) = site.owner {
            self.0.borrow_mut().push(std::ptr::from_ref(value) as usize);
        }
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
        circuits: vec![],
        ledger_fields: vec![LedgerField {
            id: "stored".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "update".into(),
            parameters: vec![],
            actions: vec![StateAction::CellWrite {
                field: "stored".into(),
                index: 0,
                value,
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    }
}
fn refusal(source: &Contract, observer: Option<&dyn RefusalObserver>) -> RenderError {
    crate::render_with_native_refusal(source, None, observer)
        .err()
        .expect("expected full native refusal")
}

#[test]
fn full_cell_action_observer_keeps_actual_default_two_mib_boundary() {
    // Use actual whole-contract production preflight; no invented depth policy.
    let depth = (1..=64)
        .take_while(|d| crate::resource_limits::validate(&fixture(*d)).is_ok())
        .last()
        .expect("small Cell action is accepted");
    assert!(
        (3..48).contains(&depth),
        "unexpected full Cell depth {depth}"
    );
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
        let accepted = fixture(depth);
        let trace = Trace::default();
        // This includes general stateful assembly, action dispatch, Cell syntax,
        // root native output and capability generation, not the expression core.
        let observed = crate::render_with_native_refusal(&accepted, None, Some(&trace))
            .expect("full observed Cell action at actual DEFAULT boundary");
        let bare = crate::render_with_capabilities(&accepted).unwrap();
        assert_eq!(observed.source, bare.source);
        assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(),
                   serde_json::to_value(&bare.capabilities).unwrap());
        assert!(trace.0.borrow().is_empty());

        let over = fixture(depth + 1);
        let error = refusal(&over, Some(&trace));
        assert!(matches!(error, RenderError::ResourceLimit { .. }));
        assert_eq!(error, refusal(&over, None));

        let mut malformed = fixture(depth);
        let StateAction::CellWrite { value, .. } = &mut malformed.stateful_circuits[0].actions[0]
            else { panic!("Cell fixture") };
        let mut leaf = value;
        while let Expr::Add { left, .. } = leaf { leaf = left; }
        *leaf = Expr::FieldLiteral { value: "bad".into() };
        let original = std::ptr::from_ref(leaf) as usize;
        let trace = Trace::default();
        let error = refusal(&malformed, Some(&trace));
        assert!(matches!(error, RenderError::InvalidFieldLiteral(ref value) if value == "bad"));
        assert_eq!(error, refusal(&malformed, None));
        assert_eq!(trace.0.borrow().first(), Some(&original));
        eprintln!("Full Cell action: actual DEFAULT depth {depth} accepted; {} refused; original malformed leaf captured", depth + 1);
    }).unwrap().join().unwrap();
}
