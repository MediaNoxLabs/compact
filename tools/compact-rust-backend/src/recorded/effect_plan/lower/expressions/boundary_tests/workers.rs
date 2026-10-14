// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Full actual renderer and recorded lowering on an ordinary2MiB worker.
//! Original Cell boundary fixture shape; private IR, not frontend/proof coverage.
use super::*;
use crate::ir::{Contract, LedgerField, SCHEMA_VERSION};
fn fixture(depth: usize) -> Contract {
    let mut value = field("1");
    for _ in 0..depth {
        value = Expr::Add {
            left: Box::new(value),
            right: Box::new(field("1")),
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
fn parity(source: &Contract, trace: &Trace) -> crate::RenderedContract {
    let observed = crate::render_with_refusals(source, None, Some(trace), Some(trace)).unwrap();
    let bare = crate::render_with_capabilities(source).unwrap();
    assert_eq!(observed.source, bare.source);
    assert_eq!(
        serde_json::to_value(&observed.capabilities).unwrap(),
        serde_json::to_value(&bare.capabilities).unwrap()
    );
    observed
}
#[test]
fn full_recorded_expression_boundary_keeps_default_two_mib_and_nonfatal_gap() {
    let depth = (1..=64)
        .take_while(|depth| crate::resource_limits::validate(&fixture(*depth)).is_ok())
        .last()
        .expect("small Cell accepted");
    assert!((3..48).contains(&depth), "actual DEFAULT depth {depth}");
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
        let near = fixture(depth); let trace = Trace::default(); let rendered = parity(&near, &trace);
        assert!(rendered.capabilities.circuits.iter().find(|circuit| circuit.name == "update").unwrap().recorded,
            "successful current recording dispatcher actually lowers the deep body");
        assert!(trace.events.borrow().is_empty());
        let over = fixture(depth + 1); let trace = Trace::default();
        let observed = crate::render_with_native_refusal(&over, None, Some(&trace)).err().expect("one over refusal");
        let bare = crate::render_with_capabilities(&over).err().expect("same one over refusal");
        assert_eq!(observed, bare); assert!(matches!(observed, crate::RenderError::ResourceLimit { .. }));
        // Existing native-supported/frontend-excluded opaque commitment is a
        // private nonfatal recording refusal control, not native-invalid syntax.
        let commitment = Expr::PersistentCommit { value: Box::new(Expr::Default { ty: Type::OpaqueString }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }) };
        let source = Contract { schema_version: SCHEMA_VERSION, type_aliases: vec![], constructor: None,
            witnesses: vec![], circuits: vec![], ledger_fields: vec![], stateful_circuits: vec![StatefulCircuit {
                source: None, internal: false, name: "commit".into(), parameters: vec![], actions: vec![],
                result: Type::Bytes { length: 32 }, return_value: StateReturn::Expression { value: commitment } }] };
        let StateReturn::Expression { value } = &source.stateful_circuits[0].return_value else { panic!("fixture") };
        let trace = Trace::default(); let rendered = parity(&source, &trace);
        let capability = rendered.capabilities.circuits.iter().find(|circuit| circuit.name == "commit").unwrap();
        assert!(!capability.recorded); assert!(capability.recording_unavailable.is_some());
        first(&trace, Some(value));
        eprintln!("Recorded expression dispatcher: fullCell DEFAULT depth{depth} accepted/recorded, depth{} refused, actual nonfatal opaque commitment unchanged on2MiB", depth + 1);
    }).unwrap().join().unwrap();
}
