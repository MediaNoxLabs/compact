// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-native constructor ordering, matching the retained official TS cases.
include!(concat!(env!("OUT_DIR"), "/constructor_order.rs"));
#[test]
fn invalid_earlier_assignment_is_not_masked_by_later_pure_assertion() {
    for ok in [false, true] {
        let error = ledger_contract::initial_state(
            runtime::context::ConstructorContext::new(7),
            runtime::OpaqueString::from("bad\0"),
            ok,
        )
        .err()
        .unwrap();
        assert_eq!(
            error,
            runtime::CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )
        );
    }
    let error = ledger_contract::initial_state(
        runtime::context::ConstructorContext::new(7),
        runtime::OpaqueString::from("canonical"),
        false,
    )
    .err()
    .unwrap();
    assert_eq!(
        error,
        runtime::CompactError::AssertionFailed("later RHS failed".into())
    );
    let context = ledger_contract::initial_state(
        runtime::context::ConstructorContext::new(7),
        runtime::OpaqueString::from("canonical"),
        true,
    )
    .unwrap()
    .into_circuit_context(Default::default());
    assert_eq!(
        ledger_slots::text
            .inspect(context.query.state.get_ref())
            .unwrap(),
        runtime::OpaqueString::from("canonical")
    );
    assert_eq!(
        ledger_slots::count
            .inspect(context.query.state.get_ref())
            .unwrap(),
        runtime::Field::from(7_u64)
    );
}
