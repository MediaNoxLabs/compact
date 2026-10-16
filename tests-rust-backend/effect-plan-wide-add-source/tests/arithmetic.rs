// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_wide_add_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::fab::{Alignment, AlignmentAtom};
use r::recording::RecordingFrame;
type Wide = r::WideUint<1, { u128::MAX }>;
type Wider = r::WideUint<3, { u128::MAX }>;
macro_rules! success {
    ($name:ident,$call:ident,$a:expr,$b:expr,$expected:expr,$input:expr,$output:expr,$left_width:expr,$right_width:expr,$result_width:expr) => {{
        let (state, key) = setup();
        let (a, b) = ($a, $b);
        let native = c::$name(state.circuit_context(5), a, b).unwrap();
        assert_eq!(native.result, $expected);
        let recorded = c::recorded::$name(state.circuit_context(5), a, b).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish($expected);
        compare(&native, &recorded, &manual);
        let contract = c::Contract::default();
        let call = contract.recording().$call(&state, 5, a, b).unwrap();
        compare(&native, call.recorded(), &manual);
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(atoms(&prepared.input), $input);
        assert_eq!(atoms(&prepared.output), vec![$output]);
        let left = Alignment::singleton(AlignmentAtom::Bytes {
            length: $left_width,
        });
        let right = Alignment::singleton(AlignmentAtom::Bytes {
            length: $right_width,
        });
        assert_eq!(prepared.input.alignment, Alignment::concat([&left, &right]));
        assert_eq!(
            prepared.output.alignment,
            Alignment::singleton(AlignmentAtom::Bytes {
                length: $result_width
            })
        );
        assert_ts(
            stringify!($name),
            &prepared.input,
            &prepared.output,
            &native,
        );
    }};
}
#[test]
fn wide_addition_preserves_carry_mixed_widths_and_exact_public_representation() {
    for (a, b, high, low) in [
        (0, 0, 0, 0),
        (u128::MAX, 0, 0, u128::MAX),
        (u128::MAX, 1, 1, 0),
        (u128::MAX, u128::MAX, 1, u128::MAX - 1),
    ] {
        let raw = bytes(high, low);
        success!(
            add128,
            add128_call,
            r::BoundedUint::<{ u128::MAX }>::new(a).unwrap(),
            r::BoundedUint::<{ u128::MAX }>::new(b).unwrap(),
            types::Wide {
                value: Wide::from_le_bytes(&raw).unwrap()
            },
            vec![bytes(0, a), bytes(0, b)],
            raw.clone(),
            16,
            16,
            17
        );
    }
    for (high, low, b, result_high, result_low) in [
        (0, u128::MAX, 1, 1, 0),
        (1, u128::MAX, 0, 1, u128::MAX),
        (1, u128::MAX, 255, 2, 254),
    ] {
        let raw = bytes(result_high, result_low);
        success!(
            mixed,
            mixed_call,
            Wide::from_le_bytes(&bytes(high, low)).unwrap(),
            r::BoundedUint::<255>::new(b).unwrap(),
            types::Wider {
                value: Wider::from_le_bytes(&raw).unwrap()
            },
            vec![bytes(high, low), bytes(0, b)],
            raw.clone(),
            17,
            1,
            17
        );
    }
    for value in [0, u128::MAX] {
        success!(
            narrow128,
            narrow128_call,
            r::BoundedUint::<{ u128::MAX }>::new(value).unwrap(),
            r::BoundedUint::<{ u128::MAX }>::new(0).unwrap(),
            types::Narrow {
                value: r::BoundedUint::new(value).unwrap()
            },
            vec![bytes(0, value), vec![]],
            bytes(0, value),
            16,
            16,
            16
        );
    }
}
#[test]
fn successful_wide_addition_still_refuses_a_narrowed_result_outside_uint128() {
    let (state, _) = setup();
    let contract = c::Contract::default();
    let before = state.circuit_context(5).query.state;
    for b in [1, u128::MAX] {
        let a = r::BoundedUint::<{ u128::MAX }>::new(u128::MAX).unwrap();
        let b = r::BoundedUint::<{ u128::MAX }>::new(b).unwrap();
        assert_eq!(
            c::narrow128(state.circuit_context(5), a, b).err().unwrap(),
            r::CompactError::InvalidUnsignedValue
        );
        assert_eq!(
            c::recorded::narrow128(state.circuit_context(5), a, b)
                .err()
                .unwrap(),
            r::CompactError::InvalidUnsignedValue
        );
        assert_eq!(
            contract
                .recording()
                .narrow128_call(&state, 5, a, b)
                .err()
                .unwrap(),
            r::CompactError::InvalidUnsignedValue
        );
    }
    assert_eq!(state.circuit_context(5).query.state, before);
}
