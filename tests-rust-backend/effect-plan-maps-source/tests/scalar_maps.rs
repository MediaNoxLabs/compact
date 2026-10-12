// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, witness::*, *};
use compact_rust_effect_plan_maps_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
    types::{Found, Present},
};
use r::context::CircuitResult;
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::transaction::{RecordedCall, VerifierKey};
struct Scenario<'a, T> {
    id: &'a str,
    args: serde_json::Value,
    expected: T,
    input: Vec<Vec<u8>>,
    output: Vec<Vec<u8>>,
    entries: &'a [(&'a str, u64)],
    calls: u64,
}
fn checked<T: std::fmt::Debug + PartialEq + Into<r::fab::AlignedValue>>(
    scenario: Scenario<'_, T>,
    native: CircuitResult<u64, T>,
    recorded: RecordedCircuitResult<u64, T>,
    manual: RecordedCircuitResult<u64, T>,
    call: RecordedCall<'_, u64, T>,
    key: VerifierKey,
) {
    let Scenario {
        id,
        args,
        expected,
        input,
        output,
        entries,
        calls,
    } = scenario;
    assert_eq!(native.result, expected);
    compare(&native, &recorded, &manual);
    compare(&native, call.recorded(), &manual);
    inspect(&native, entries, calls);
    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_eq!(atoms(&prepared.input), input);
    assert_eq!(atoms(&prepared.output), output);
    let row = row(id);
    assert_eq!(row["circuit"], id.split('-').next().unwrap());
    assert_eq!(row["args"], args);
    assert_eq!(row["seed"], "seed");
    assert_eq!(row["choices"], serde_json::json!([false, false]));
    assert!(row["fail"].is_null());
    assert_ts(&row, &prepared.input, &prepared.output, &native);
}
#[test]
fn scalar_membership_lookup_insert_overwrite_and_remove_match_official_ts() {
    let contract = c::Contract::from(Witness::new([false, false], None));
    for (key, present) in [("seed", true), ("missing", false)] {
        let (state, vk) = setup();
        let native = c::member(state.circuit_context(5), key.into()).unwrap();
        let recorded = c::recorded::member(state.circuit_context(5), key.into()).unwrap();
        let (frame, value) = slots::table
            .record_member(RecordingFrame::new(state.circuit_context(5)), key.into())
            .unwrap();
        let manual = frame.finish(Present { value });
        let call = contract
            .recording()
            .member_call(&state, 5, key.into())
            .unwrap();
        checked(
            Scenario {
                id: &format!("member-{key}"),
                args: serde_json::json!([key]),
                expected: Present { value: present },
                input: vec![key.as_bytes().to_vec()],
                output: vec![if present { vec![1] } else { vec![] }],
                entries: &[("seed", 7)],
                calls: 0,
            },
            native,
            recorded,
            manual,
            call,
            vk,
        );
    }
    {
        let (state, vk) = setup();
        let key = "seed";
        let native = c::lookup(state.circuit_context(5), key.into()).unwrap();
        let recorded = c::recorded::lookup(state.circuit_context(5), key.into()).unwrap();
        let (frame, value) = slots::table
            .record_lookup(RecordingFrame::new(state.circuit_context(5)), key.into())
            .unwrap();
        let manual = frame.finish(Found { value });
        let call = contract
            .recording()
            .lookup_call(&state, 5, key.into())
            .unwrap();
        checked(
            Scenario {
                id: "lookup-seed",
                args: serde_json::json!([key]),
                expected: Found {
                    value: r::Field::from(7_u64),
                },
                input: vec![key.as_bytes().to_vec()],
                output: vec![vec![7]],
                entries: &[("seed", 7)],
                calls: 0,
            },
            native,
            recorded,
            manual,
            call,
            vk,
        );
    }
    for (id, key, value) in [
        ("new", "new", 13_u64),
        ("overwrite", "seed", 11),
        ("unicode", "λ", 19),
        ("interior-nul", "a\0b", 23),
    ] {
        let (state, vk) = setup();
        let field = r::Field::from(value);
        let native = c::put(state.circuit_context(5), key.into(), field).unwrap();
        let recorded = c::recorded::put(state.circuit_context(5), key.into(), field).unwrap();
        let frame = slots::table
            .record_insert(
                RecordingFrame::new(state.circuit_context(5)),
                key.into(),
                field,
            )
            .unwrap();
        let frame = slots::calls.record_increment(frame, 1).unwrap();
        let (frame, value_again) = slots::table.record_lookup(frame, key.into()).unwrap();
        let manual = frame.finish(Found { value: value_again });
        let call = contract
            .recording()
            .put_call(&state, 5, key.into(), field)
            .unwrap();
        let entries = if key == "seed" {
            vec![(key, value)]
        } else {
            vec![("seed", 7), (key, value)]
        };
        checked(
            Scenario {
                id: &format!("put-{id}"),
                args: serde_json::json!([key, value.to_string()]),
                expected: Found { value: field },
                input: vec![key.as_bytes().to_vec(), vec![value as u8]],
                output: vec![vec![value as u8]],
                entries: &entries,
                calls: 1,
            },
            native,
            recorded,
            manual,
            call,
            vk,
        );
    }
    for key in ["seed", "missing"] {
        let (state, vk) = setup();
        let native = c::remove(state.circuit_context(5), key.into()).unwrap();
        let recorded = c::recorded::remove(state.circuit_context(5), key.into()).unwrap();
        let frame = slots::table
            .record_remove(RecordingFrame::new(state.circuit_context(5)), key.into())
            .unwrap();
        let frame = slots::calls.record_increment(frame, 1).unwrap();
        let (frame, value) = slots::table.record_member(frame, key.into()).unwrap();
        let manual = frame.finish(Present { value });
        let call = contract
            .recording()
            .remove_call(&state, 5, key.into())
            .unwrap();
        let entries = if key == "seed" {
            vec![]
        } else {
            vec![("seed", 7)]
        };
        checked(
            Scenario {
                id: &format!("remove-{key}"),
                args: serde_json::json!([key]),
                expected: Present { value: false },
                input: vec![key.as_bytes().to_vec()],
                output: vec![vec![]],
                entries: &entries,
                calls: 1,
            },
            native,
            recorded,
            manual,
            call,
            vk,
        );
    }
}
#[test]
fn missing_lookup_refuses_and_direct_returns_remain_native_ts_controls() {
    let (state, _) = setup();
    let contract = c::Contract::from(Witness::new([false, false], None));
    let native = c::lookup(state.circuit_context(5), "missing".into())
        .err()
        .unwrap();
    assert_eq!(
        c::recorded::lookup(state.circuit_context(5), "missing".into())
            .err()
            .unwrap(),
        native
    );
    assert_eq!(
        contract
            .recording()
            .lookup_call(&state, 5, "missing".into())
            .err()
            .unwrap(),
        native
    );
    assert_eq!(
        slots::table
            .record_lookup(
                RecordingFrame::new(state.circuit_context(5)),
                "missing".into()
            )
            .err()
            .unwrap(),
        native
    );
    let missing = row("lookup-missing");
    assert_eq!(missing["status"], "refused");
    assert_eq!(missing["args"], serde_json::json!(["missing"]));
    assert_eq!(missing["events"], serde_json::json!([]));
    // Direct StateReturn roots intentionally retain their existing admission policy.
    // They are native+TS controls; there is no fabricated recorded/observed surface.
    for (key, present) in [("seed", true), ("missing", false)] {
        let result = c::direct_member(state.circuit_context(5), key.into()).unwrap();
        assert_eq!(result.result, present);
        inspect(&result, &[("seed", 7)], 0);
        let reference = row(&format!("direct-member-{key}"));
        assert_eq!(reference["args"], serde_json::json!([key]));
        assert_eq!(reference["result"], present);
        assert_ts(
            &reference,
            &r::fab::AlignedValue::from(r::OpaqueString::from(key)),
            &r::fab::AlignedValue::from(present),
            &result,
        );
    }
    let result = c::direct_lookup(state.circuit_context(5), "seed".into()).unwrap();
    assert_eq!(result.result, r::Field::from(7_u64));
    let reference = row("direct-lookup-seed");
    assert_eq!(reference["result"], "7");
    assert_ts(
        &reference,
        &r::fab::AlignedValue::from(r::OpaqueString::from("seed")),
        &r::fab::AlignedValue::from(r::Field::from(7_u64)),
        &result,
    );
    assert_eq!(
        c::direct_lookup(state.circuit_context(5), "missing".into())
            .err()
            .unwrap(),
        native
    );
    let reference = row("direct-lookup-missing");
    assert_eq!(reference["status"], "refused");
    assert_eq!(reference["args"], serde_json::json!(["missing"]));
    // Safe collection APIs refuse noncanonical keys before trailing-NUL normalization.
    for key in ["seed\0", "a\0b\0"] {
        assert!(c::member(state.circuit_context(5), key.into()).is_err());
        assert!(c::recorded::member(state.circuit_context(5), key.into()).is_err());
        assert!(
            contract
                .recording()
                .member_call(&state, 5, key.into())
                .is_err()
        );
    }
}
