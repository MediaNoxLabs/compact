// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{witness::*, *};
use compact_rust_effect_plan_wide_field_cast_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
    types::{Checked, Converted},
};
use r::fab::{Alignment, AlignmentAtom};
use r::recording::RecordingFrame;
macro_rules! direct {
    ($method:ident,$call:ident,$carrier:ty,$raw:expr,$width:expr,$decimal:expr) => {{
        let raw = $raw;
        let value = <$carrier>::from_le_bytes(&raw).unwrap();
        let expected = field(&raw);
        let (state, key) = setup();
        let native = c::$method(state.circuit_context(5), value).unwrap();
        let recorded = c::recorded::$method(state.circuit_context(5), value).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(Converted { value: expected });
        let contract = c::Contract::from(Witness::new(raw.clone(), None));
        let call = contract.recording().$call(&state, 5, value).unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(native.result, Converted { value: expected });
        assert_eq!(native.context.private_state, 5);
        assert!(native.private_transcript_outputs.is_empty());
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(atoms(&prepared.input), vec![raw.clone()]);
        assert_eq!(atoms(&prepared.output), vec![raw]);
        assert_eq!(
            prepared.input.alignment,
            Alignment::singleton(AlignmentAtom::Bytes { length: $width })
        );
        assert_eq!(
            prepared.output.alignment,
            Alignment::singleton(AlignmentAtom::Field)
        );
        assert_ts(
            TsCase {
                circuit: stringify!($method),
                args: serde_json::json!([$decimal]),
                value: $decimal,
                fail: None,
            },
            &prepared.input,
            &prepared.output,
            &native,
        );
    }};
}
#[test]
fn canonical_wide_field_casts_preserve_all_limbs_and_reject_invalid_carriers() {
    for (high, low, decimal) in boundary_cases(Width::Bits129) {
        direct!(cast129, cast129_call, Wide, bytes(high, low), 17, decimal);
    }
    for (high, low, decimal) in boundary_cases(Width::Bits248) {
        direct!(
            cast248,
            cast248_call,
            Largest,
            bytes(high, low),
            31,
            decimal
        );
    }
    // Invalid carriers cannot enter safe generated APIs; these are construction
    // refusals, not failing conversions of an already-valid WideUint.
    assert_eq!(
        Wide::from_le_bytes(&bytes(2, 0)),
        Err(r::CompactError::InvalidUnsignedValue)
    );
    assert_eq!(
        Largest::from_le_bytes(&bytes(1_u128 << 120, 0)),
        Err(r::CompactError::InvalidUnsignedValue)
    );
    assert_eq!(
        Largest::from_le_bytes(&[0; 33]),
        Err(r::CompactError::InvalidUnsignedValue)
    );
}
#[test]
fn lazy_wide_witnesses_keep_original_alignment_order_and_prepared_bindings() {
    for width in [Width::Bits129, Width::Bits248] {
        for (high, low, decimal) in boundary_cases(width) {
            for selected in [false, true] {
                let raw = bytes(high, low);
                let (a, b, m, w) = (
                    Witness::new(raw.clone(), None),
                    Witness::new(raw.clone(), None),
                    Witness::new(raw.clone(), None),
                    Witness::new(raw.clone(), None),
                );
                let (state, key) = setup();
                let native = width
                    .native(state.circuit_context(5), &a, selected)
                    .unwrap();
                let recorded = width
                    .recorded(state.circuit_context(5), &b, selected)
                    .unwrap();
                let manual = manual(state.circuit_context(5), &m, width, selected).unwrap();
                let call = width.observed(&state, &w, selected).unwrap();
                compare(&native, &recorded, &manual);
                compare(&native, call.recorded(), &manual);
                let result_raw = if selected { raw.clone() } else { vec![7] };
                let expected = field(&result_raw);
                assert_eq!(
                    native.result,
                    Checked {
                        value: expected,
                        marker: r::Field::from(9_u64)
                    }
                );
                assert_eq!(native.context.private_state, if selected { 7 } else { 6 });
                assert_eq!(
                    native
                        .private_transcript_outputs
                        .iter()
                        .map(atoms)
                        .collect::<Vec<_>>(),
                    if selected {
                        vec![vec![raw], vec![vec![9]]]
                    } else {
                        vec![vec![vec![9]]]
                    }
                );
                let events = if selected {
                    vec![(width.witness(), 5, 1), ("marker", 6, 1)]
                } else {
                    vec![("marker", 5, 1)]
                };
                for witness in [&a, &b, &m, &w] {
                    assert_eq!(*witness.events.borrow(), events);
                }
                let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
                assert_eq!(
                    atoms(&prepared.input),
                    vec![if selected { vec![1] } else { vec![] }]
                );
                assert_eq!(atoms(&prepared.output), vec![result_raw, vec![9]]);
                assert_ts(
                    TsCase {
                        circuit: width.name(),
                        args: serde_json::json!([selected]),
                        value: decimal,
                        fail: None,
                    },
                    &prepared.input,
                    &prepared.output,
                    &native,
                );
            }
        }
    }
}
#[test]
fn wide_witness_refusal_stops_at_exact_prefix_and_unselected_provider_is_skipped() {
    for width in [Width::Bits129, Width::Bits248] {
        for failed in [width.witness(), "marker"] {
            for selected in [false, true] {
                let (a, b, m, w) = (
                    Witness::new(vec![1], Some(failed)),
                    Witness::new(vec![1], Some(failed)),
                    Witness::new(vec![1], Some(failed)),
                    Witness::new(vec![1], Some(failed)),
                );
                let (state, key) = setup();
                let native = width.native(state.circuit_context(5), &a, selected);
                let recorded = width.recorded(state.circuit_context(5), &b, selected);
                let manual = manual(state.circuit_context(5), &m, width, selected);
                let call = width.observed(&state, &w, selected);
                let events = if !selected {
                    vec![("marker", 5, 1)]
                } else if failed == width.witness() {
                    vec![(width.witness(), 5, 1)]
                } else {
                    vec![(width.witness(), 5, 1), ("marker", 6, 1)]
                };
                for witness in [&a, &b, &m, &w] {
                    assert_eq!(*witness.events.borrow(), events);
                }
                if selected || failed == "marker" {
                    assert_eq!(native.err().unwrap(), failure(failed));
                    assert_eq!(recorded.err().unwrap(), failure(failed));
                    assert_eq!(manual.err().unwrap(), failure(failed));
                    assert_eq!(call.err().unwrap(), failure(failed));
                    let oracle: serde_json::Value =
                        serde_json::from_str(include_str!("../oracle/typescript.json")).unwrap();
                    let row = oracle["rows"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|x| {
                            x["circuit"] == width.name()
                                && x["status"] == "refused"
                                && x["fail"] == failed
                                && x["args"] == serde_json::json!([selected])
                        })
                        .expect("matching TS callback refusal");
                    let expected = events
                        .iter()
                        .map(|(name, private, count)| {
                            serde_json::json!([name, private, count.to_string()])
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(row["events"], serde_json::json!(expected));
                } else {
                    let (native, recorded, manual, call) = (
                        native.unwrap(),
                        recorded.unwrap(),
                        manual.unwrap(),
                        call.unwrap(),
                    );
                    compare(&native, &recorded, &manual);
                    compare(&native, call.recorded(), &manual);
                    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
                    assert_ts(
                        TsCase {
                            circuit: width.name(),
                            args: serde_json::json!([selected]),
                            value: "1",
                            fail: Some(failed),
                        },
                        &prepared.input,
                        &prepared.output,
                        &native,
                    );
                }
                assert_eq!(
                    slots::calls
                        .inspect(state.circuit_context(5).query.state.get_ref())
                        .unwrap(),
                    0
                );
            }
        }
    }
}
