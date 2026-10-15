// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use compact_rust_effect_plan_map_value_products_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, support, types,
};
use r::fab::AlignedValue;
use r::ledger::CellValue;
#[test]
fn map_products_preserve_codec_and_recorded_guarded_reads() {
    let max128 = [255_u8; 16];
    let mut max129 = [255_u8; 17];
    max129[16] = 1;
    for wide in [
        support::Wide::from_le_bytes(&[]).unwrap(),
        support::Wide::from_le_bytes(&max128).unwrap(),
        support::proof_value().wide,
        support::Wide::from_le_bytes(&max129).unwrap(),
    ] {
        let value = support::value(wide);
        let encoded = AlignedValue::from(value.clone());
        let mut expected_wide = wide.as_le_bytes().to_vec();
        while expected_wide.last() == Some(&0) {
            expected_wide.pop();
        }
        assert_eq!(
            encoded
                .value
                .0
                .iter()
                .map(|a| a.0.clone())
                .collect::<Vec<_>>(),
            vec![vec![100], vec![3], vec![5], vec![7], vec![1], expected_wide]
        );
        assert_eq!(
            types::Stored::decode_cell_value(&encoded.as_slice()).unwrap(),
            value
        );
        let (state, verifier) = common::setup(value.clone());
        let original = state.contract().clone();
        for as_of in [100_u64, 120] {
            let key: r::OpaqueString = "map-λ".into();
            let time = r::BoundedUint::new(as_of as u128).unwrap();
            let native = c::readFresh(state.circuit_context(7), key.clone(), time).unwrap();
            let recorded =
                c::recorded::readFresh(state.circuit_context(7), key.clone(), time).unwrap();
            let manual = support::manual_read(state.circuit_context(7), key.clone(), time).unwrap();
            let call = c::recorded::Contract
                .readFresh_call(&state, 7, key.clone(), time)
                .unwrap();
            common::compare(&native, &recorded, call.recorded(), &manual);
            assert_eq!(
                slots::records
                    .inspect(native.context.query.state.get_ref())
                    .unwrap()
                    .lookup(key.clone())
                    .unwrap(),
                value
            );
            assert_eq!(
                native.context.query.state.get_ref(),
                original.data.get_ref()
            );
            let prepared = call.prepare(verifier.clone(), 0_u64.into()).unwrap();
            assert_eq!(prepared.input, AlignedValue::from((key, time)));
            assert_eq!(prepared.output, AlignedValue::from(()));
            assert_eq!(state.contract(), &original);
        }
    }
}
#[test]
fn missing_and_future_guards_match_across_all_lanes() {
    let (state, _) = common::setup(support::proof_value());
    for (key, time, message) in [
        ("missing", 120, "record does not exist"),
        ("map-λ", 99, "record is in the future"),
    ] {
        let key: r::OpaqueString = key.into();
        let time = r::BoundedUint::new(time).unwrap();
        let expected = r::CompactError::AssertionFailed(message.into());
        assert_eq!(
            c::readFresh(state.circuit_context(7), key.clone(), time)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::readFresh(state.circuit_context(7), key.clone(), time)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::Contract
                .readFresh_call(&state, 7, key.clone(), time)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            support::manual_read(state.circuit_context(7), key, time)
                .err()
                .unwrap(),
            expected
        );
    }
}
