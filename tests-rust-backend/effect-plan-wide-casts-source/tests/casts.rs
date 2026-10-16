// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_wide_casts_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::fab::{Alignment, AlignmentAtom};
use r::recording::RecordingFrame;
type Wide = r::WideUint<1, { u128::MAX }>;

// Each row runs complete generated native, recorded and observed entry points.
macro_rules! success {
    ($method:ident, $call:ident, $input:expr, $expected:expr, $raw:expr, $input_len:expr, $output_len:expr) => {{
        let (state, key) = setup();
        let input = $input;
        let native = c::$method(state.circuit_context(5), input).unwrap();
        assert_eq!(native.result, $expected);
        let recorded = c::recorded::$method(state.circuit_context(5), input).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish($expected);
        compare(&native, &recorded, &manual);
        let contract = c::Contract::default();
        let call = contract.recording().$call(&state, 5, input).unwrap();
        compare(&native, call.recorded(), &manual);
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_ts(
            stringify!($method),
            &prepared.input,
            &prepared.output,
            &native,
        );
        assert_eq!(atoms(&prepared.input), vec![$raw]);
        assert_eq!(atoms(&prepared.output), vec![$raw]);
        assert_eq!(
            prepared.input.alignment,
            Alignment::singleton(AlignmentAtom::Bytes { length: $input_len })
        );
        assert_eq!(
            prepared.output.alignment,
            Alignment::singleton(AlignmentAtom::Bytes {
                length: $output_len
            })
        );
    }};
}
#[test]
fn canonical_cast_boundaries_preserve_state_gas_transcript_and_exact_public_atoms() {
    for value in [0, 1, 255] {
        let raw = bytes(0, value);
        success!(
            widen8,
            widen8_call,
            r::BoundedUint::<255>::new(value).unwrap(),
            types::Wide {
                value: Wide::from_le_bytes(&raw).unwrap()
            },
            raw.clone(),
            1,
            17
        );
        success!(
            narrow8,
            narrow8_call,
            Wide::from_le_bytes(&raw).unwrap(),
            types::Small {
                value: r::BoundedUint::<255>::new(value).unwrap()
            },
            raw.clone(),
            17,
            1
        );
    }
    for (high, low) in [(0, 0), (0, u128::MAX), (1, 0), (1, u128::MAX)] {
        let raw = bytes(high, low);
        success!(
            widen129,
            widen129_call,
            Wide::from_le_bytes(&raw).unwrap(),
            types::Largest {
                value: Largest::from_le_bytes(&raw).unwrap()
            },
            raw.clone(),
            17,
            31
        );
        success!(
            narrow129,
            narrow129_call,
            Largest::from_le_bytes(&raw).unwrap(),
            types::Wide {
                value: Wide::from_le_bytes(&raw).unwrap()
            },
            raw.clone(),
            31,
            17
        );
    }
    let (state, key) = setup();
    let raw = vec![255; 31];
    let native = c::max248(state.circuit_context(5)).unwrap();
    let expected = types::Largest {
        value: Largest::from_le_bytes(&raw).unwrap(),
    };
    assert_eq!(native.result, expected);
    let recorded = c::recorded::max248(state.circuit_context(5)).unwrap();
    let manual = slots::calls
        .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
        .unwrap()
        .finish(expected);
    compare(&native, &recorded, &manual);
    let contract = c::Contract::default();
    let call = contract.recording().max248_call(&state, 5).unwrap();
    compare(&native, call.recorded(), &manual);
    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_ts("max248", &prepared.input, &prepared.output, &native);
    assert!(atoms(&prepared.input).is_empty());
    assert_eq!(atoms(&prepared.output), vec![raw]);
    assert_eq!(
        prepared.output.alignment,
        Alignment::singleton(AlignmentAtom::Bytes { length: 31 })
    );
}
#[test]
fn narrowing_refuses_out_of_range_values_without_mutating_observed_snapshot() {
    let (state, _) = setup();
    let contract = c::Contract::default();
    let before = state.circuit_context(5).query.state;
    for (high, low) in [(0, 256), (1, 0), (1, u128::MAX)] {
        let value = Wide::from_le_bytes(&bytes(high, low)).unwrap();
        let expected = if high == 0 {
            r::CompactError::UnsignedOutOfRange {
                value: low,
                max: 255,
            }
        } else {
            r::CompactError::InvalidUnsignedValue
        };
        assert_eq!(
            c::narrow8(state.circuit_context(5), value).err().unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::narrow8(state.circuit_context(5), value)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            contract
                .recording()
                .narrow8_call(&state, 5, value)
                .err()
                .unwrap(),
            expected
        );
    }
    for (high, low) in [(2, 0), ((1_u128 << 120) - 1, u128::MAX)] {
        let value = Largest::from_le_bytes(&bytes(high, low)).unwrap();
        assert_eq!(
            c::narrow129(state.circuit_context(5), value).err().unwrap(),
            r::CompactError::InvalidUnsignedValue
        );
        assert_eq!(
            c::recorded::narrow129(state.circuit_context(5), value)
                .err()
                .unwrap(),
            r::CompactError::InvalidUnsignedValue
        );
        assert_eq!(
            contract
                .recording()
                .narrow129_call(&state, 5, value)
                .err()
                .unwrap(),
            r::CompactError::InvalidUnsignedValue
        );
    }
    assert_eq!(state.circuit_context(5).query.state, before);
    // These safe carriers reject invalid public values before a generated call can be made.
    assert_eq!(
        Wide::from_le_bytes(&bytes(2, 0)),
        Err(r::CompactError::InvalidUnsignedValue)
    );
    assert_eq!(
        Largest::from_le_bytes(&bytes(1_u128 << 120, 0)),
        Err(r::CompactError::InvalidUnsignedValue)
    );
}
