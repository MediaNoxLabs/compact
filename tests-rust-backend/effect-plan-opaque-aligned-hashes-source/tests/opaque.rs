// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, *};
use compact_rust_effect_plan_opaque_aligned_hashes_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Output,
};
fn field(text: &str) -> r::Field {
    text.bytes().fold(r::Field::from(0_u64), |value, digit| {
        assert!(digit.is_ascii_digit());
        value * r::Field::from(10_u64) + r::Field::from(u64::from(digit - b'0'))
    })
}
fn expected(reference: &serde_json::Value) -> Output {
    Output {
        x: field(reference["result"]["x"].as_str().unwrap()),
        y: field(reference["result"]["y"].as_str().unwrap()),
        value: field(reference["result"]["value"].as_str().unwrap()),
    }
}
/// A separately captured official TS answer for normalized input. This is
/// historical negative control: the exact-input backend must differ from it.
fn normalized_expected(reference: &serde_json::Value) -> Output {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/normalized-reference.json")).unwrap();
    let rows = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"] == reference["id"])
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        1,
        "unique normalization control {}",
        reference["id"]
    );
    let row = rows[0];
    assert_eq!(row["op"], reference["op"]);
    assert_eq!(row["type"], reference["type"]);
    assert_eq!(row["originalInput"], reference["input"]);
    let mut bytes = if reference["type"] == "string" {
        reference["input"].as_str().unwrap().as_bytes().to_vec()
    } else {
        reference["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
            .collect()
    };
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    assert_eq!(
        row["normalizedInput"],
        if reference["type"] == "string" {
            serde_json::json!(String::from_utf8(bytes).unwrap())
        } else {
            serde_json::json!(bytes)
        }
    );
    expected(row)
}
struct Executions<'a> {
    native: r::context::CircuitResult<u64, Output>,
    recorded: r::recording::RecordedCircuitResult<u64, Output>,
    observed: Result<r::transaction::RecordedCall<'a, u64, Output>, r::CompactError>,
}
fn check(
    reference: &serde_json::Value,
    input: r::fab::AlignedValue,
    x: Executions<'_>,
    state: &r::transaction::ObservedContractState,
    verifier: r::transaction::VerifierKey,
    trailing_zero: bool,
) {
    let original_output = expected(reference);
    assert_eq!(
        x.native.result, x.recorded.execution.result,
        "native/recorded case {}",
        reference["id"]
    );
    let output = original_output;
    if trailing_zero {
        assert_ne!(
            x.native.result,
            normalized_expected(reference),
            "exact input must differ from historical normalized hash for {}",
            reference["id"]
        );
    }
    assert_eq!(
        x.native.result, output,
        "case {} input {}",
        reference["id"], reference["input"]
    );
    let manual = slots::calls
        .record_increment(
            r::recording::RecordingFrame::new(state.circuit_context(5)),
            1,
        )
        .unwrap()
        .finish(output.clone());
    compare(&x.native, &x.recorded, &manual);
    assert_eq!(x.native.result, output);
    assert_eq!(
        slots::calls
            .inspect(x.native.context.query.state.get_ref())
            .unwrap(),
        1
    );
    assert_eq!(x.native.context.private_state, 5);
    assert!(x.native.private_transcript_outputs.is_empty());
    assert_ts_execution(reference, &output.clone().into(), &x.native);
    if trailing_zero {
        assert_eq!(
            reference["rustObservedExpectation"],
            "trailing-zero-refusal"
        );
        assert_eq!(
            x.observed.err().unwrap(),
            r::CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )
        );
    } else {
        assert_eq!(reference["rustObservedExpectation"], "success");
        let observed = x.observed.unwrap();
        compare(&x.native, observed.recorded(), &manual);
        let prepared = observed.prepare(verifier, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, input);
        assert_eq!(prepared.output, r::fab::AlignedValue::from(output));
        assert_ts(reference, &prepared.input, &prepared.output, &x.native);
    }
    assert_eq!(
        slots::calls
            .inspect(state.circuit_context(5).query.state.get_ref())
            .unwrap(),
        0
    );
}
#[test]
fn opaque_strings_preserve_exact_hashes_and_observed_input_boundary() {
    for op in ["curve", "transient", "commit"] {
        for (index, value) in ["", "a", "λ\0key", "a\0"].iter().enumerate() {
            let (state, verifier) = setup();
            let contract = c::Contract::from(());
            let reference = row(&format!("{op}_string-{index}"));
            assert_eq!(reference["circuit"], format!("{op}_string"));
            assert_eq!(reference["op"], op);
            assert_eq!(reference["type"], "string");
            assert_eq!(reference["index"], index);
            assert_eq!(reference["input"], *value);
            let input = r::OpaqueString::from(*value);
            let x = match op {
                "curve" => Executions {
                    native: c::curve_string(state.circuit_context(5), input.clone()).unwrap(),
                    recorded: c::recorded::curve_string(state.circuit_context(5), input.clone())
                        .unwrap(),
                    observed: contract
                        .recording()
                        .curve_string_call(&state, 5, input.clone()),
                },
                "transient" => Executions {
                    native: c::transient_string(state.circuit_context(5), input.clone()).unwrap(),
                    recorded: c::recorded::transient_string(
                        state.circuit_context(5),
                        input.clone(),
                    )
                    .unwrap(),
                    observed: contract
                        .recording()
                        .transient_string_call(&state, 5, input.clone()),
                },
                _ => Executions {
                    native: c::commit_string(state.circuit_context(5), input.clone()).unwrap(),
                    recorded: c::recorded::commit_string(state.circuit_context(5), input.clone())
                        .unwrap(),
                    observed: contract
                        .recording()
                        .commit_string_call(&state, 5, input.clone()),
                },
            };
            check(
                &reference,
                input.into(),
                x,
                &state,
                verifier,
                value.as_bytes().last() == Some(&0),
            );
        }
    }
}
#[test]
fn opaque_bytes_preserve_exact_hashes_and_observed_input_boundary() {
    for op in ["curve", "transient", "commit"] {
        for (index, value) in [vec![], vec![0], vec![1, 0, 2], vec![1, 0]]
            .iter()
            .enumerate()
        {
            let (state, verifier) = setup();
            let contract = c::Contract::from(());
            let reference = row(&format!("{op}_bytes-{index}"));
            assert_eq!(reference["circuit"], format!("{op}_bytes"));
            assert_eq!(reference["op"], op);
            assert_eq!(reference["type"], "bytes");
            assert_eq!(reference["index"], index);
            assert_eq!(reference["input"], serde_json::json!(value));
            let input = r::OpaqueBytes(value.clone());
            let x = match op {
                "curve" => Executions {
                    native: c::curve_bytes(state.circuit_context(5), input.clone()).unwrap(),
                    recorded: c::recorded::curve_bytes(state.circuit_context(5), input.clone())
                        .unwrap(),
                    observed: contract
                        .recording()
                        .curve_bytes_call(&state, 5, input.clone()),
                },
                "transient" => Executions {
                    native: c::transient_bytes(state.circuit_context(5), input.clone()).unwrap(),
                    recorded: c::recorded::transient_bytes(state.circuit_context(5), input.clone())
                        .unwrap(),
                    observed: contract
                        .recording()
                        .transient_bytes_call(&state, 5, input.clone()),
                },
                _ => Executions {
                    native: c::commit_bytes(state.circuit_context(5), input.clone()).unwrap(),
                    recorded: c::recorded::commit_bytes(state.circuit_context(5), input.clone())
                        .unwrap(),
                    observed: contract
                        .recording()
                        .commit_bytes_call(&state, 5, input.clone()),
                },
            };
            check(
                &reference,
                input.into(),
                x,
                &state,
                verifier,
                value.last() == Some(&0),
            );
        }
    }
}
