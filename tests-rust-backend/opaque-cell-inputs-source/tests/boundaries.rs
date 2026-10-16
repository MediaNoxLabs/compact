// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use compact_rust_opaque_cell_inputs_source_fixture::ledger_contract as c;
#[test]
fn noncanonical_public_leaves_keep_typed_refusals_in_every_execution_lane() {
    let (state, _) = common::setup();
    let original = state.contract().clone();
    for bytes in [vec![0], vec![1, 0]] {
        let value = rvalue(&bytes);
        assert_eq!(
            c::setBytes(state.circuit_context(7), value).err().unwrap(),
            common::invalid()
        );
        let value = rvalue(&bytes);
        assert_eq!(
            c::recorded::setBytes(state.circuit_context(7), value.clone())
                .err()
                .unwrap(),
            common::invalid()
        );
        assert_eq!(
            c::recorded::Contract
                .setBytes_call(&state, 7, value)
                .err()
                .unwrap(),
            common::invalid()
        );
        assert_eq!(state.contract(), &original);
    }
    for value in [
        common::envelope("bad\0", &[1]),
        common::envelope("valid", &[1, 0]),
    ] {
        assert_eq!(
            c::setEnvelope(state.circuit_context(7), value.clone())
                .err()
                .unwrap(),
            common::invalid()
        );
        assert_eq!(
            c::recorded::setEnvelope(state.circuit_context(7), value.clone())
                .err()
                .unwrap(),
            common::invalid()
        );
        assert_eq!(
            c::recorded::Contract
                .setEnvelope_call(&state, 7, value)
                .err()
                .unwrap(),
            common::invalid()
        );
        assert_eq!(state.contract(), &original);
    }
}
fn rvalue(bytes: &[u8]) -> compact_rust_opaque_cell_inputs_source_fixture::runtime::OpaqueBytes {
    bytes.to_vec().into()
}
