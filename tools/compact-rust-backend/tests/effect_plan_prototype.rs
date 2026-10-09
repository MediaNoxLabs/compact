// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Opt-in admission and native-validation boundaries for ADR0355.
use compact_rust_backend::{ir::Contract, render_with_capabilities};

fn fixture() -> Contract {
    serde_json::from_str(include_str!("effect-plan-prototype-schema20-ir.json")).unwrap()
}

#[test]
fn two_real_source_compositions_require_explicit_opt_in() {
    let rendered = render_with_capabilities(&fixture()).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 2);
    for circuit in rendered.capabilities.circuits {
        assert_eq!(
            circuit.recorded,
            cfg!(feature = "experimental-recording-plan")
        );
        assert_eq!(
            circuit.observed_call,
            cfg!(feature = "experimental-recording-plan")
        );
        assert_eq!(circuit.recording_unavailable.is_none(), circuit.recorded);
    }
}

#[test]
fn invalid_ledger_declarations_fail_before_experimental_admission() {
    let mut contract = fixture();
    contract.ledger_fields[0].index = 1;
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn invalid_witness_signature_is_never_hidden_by_profile_fallback() {
    let mut contract = fixture();
    contract.witnesses[0]
        .parameters
        .push(compact_rust_backend::ir::Parameter {
            name: "missing".into(),
            ty: compact_rust_backend::ir::Type::Field,
        });
    assert!(render_with_capabilities(&contract).is_err());
}
