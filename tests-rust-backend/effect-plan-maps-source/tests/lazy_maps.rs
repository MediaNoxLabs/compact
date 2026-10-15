// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, witness::*, *};
use compact_rust_effect_plan_maps_source_fixture::{
    ledger_contract as c, runtime as r, types::Present,
};
fn run(id: &str, selected: bool, choices: [bool; 2], fail: Option<&'static str>) {
    let (state, key) = setup();
    let (a, b, m, w) = (
        Witness::new(choices, fail),
        Witness::new(choices, fail),
        Witness::new(choices, fail),
        Witness::new(choices, fail),
    );
    let native = c::lazy_put(
        state.circuit_context(5),
        &a,
        selected,
        "λ".into(),
        "other".into(),
        r::Field::from(20_u64),
        r::Field::from(30_u64),
    );
    let recorded = c::recorded::lazy_put(
        state.circuit_context(5),
        &b,
        selected,
        "λ".into(),
        "other".into(),
        r::Field::from(20_u64),
        r::Field::from(30_u64),
    );
    let manual = manual(state.circuit_context(5), &m, selected);
    let contract = c::Contract::from(w.clone());
    let call = contract.recording().lazy_put_call(
        &state,
        5,
        selected,
        "λ".into(),
        "other".into(),
        r::Field::from(20_u64),
        r::Field::from(30_u64),
    );
    let events = if !selected {
        vec![]
    } else if fail == Some("pick_key") {
        vec![("pick_key", 5, 0)]
    } else {
        vec![("pick_key", 5, 0), ("pick_value", 6, 0)]
    };
    for witness in [&a, &b, &m, &w] {
        assert_eq!(*witness.events.borrow(), events);
    }
    let reference = row(id);
    // Every configuration has a unique row, including unselected providers whose
    // successful result/transcript is identical despite different choices/refusals.
    assert_eq!(reference["circuit"], "lazy_put");
    assert_eq!(
        reference["args"],
        serde_json::json!([selected, "λ", "other", "20", "30"])
    );
    assert_eq!(reference["choices"], serde_json::json!(choices));
    assert_eq!(reference["fail"], serde_json::json!(fail));
    assert_eq!(reference["seed"], "seed");
    assert_eq!(
        reference["events"],
        serde_json::json!(
            events
                .iter()
                .map(|(name, p, n)| serde_json::json!([name, p, n.to_string()]))
                .collect::<Vec<_>>()
        )
    );
    if let Some(failed) = fail.filter(|_| selected) {
        let error = failure(failed);
        assert_eq!(native.err().unwrap(), error);
        assert_eq!(recorded.err().unwrap(), error);
        assert_eq!(manual.err().unwrap(), error);
        assert_eq!(call.err().unwrap(), error);
        assert_eq!(reference["status"], "refused");
        return;
    }
    let (native, recorded, manual, call) = (
        native.unwrap(),
        recorded.unwrap(),
        manual.unwrap(),
        call.unwrap(),
    );
    compare(&native, &recorded, &manual);
    compare(&native, call.recorded(), &manual);
    let expected = selected && choices[0];
    assert_eq!(native.result, Present { value: expected });
    let mut entries = vec![("seed", 7)];
    if selected {
        entries.push((
            if choices[0] { "λ" } else { "other" },
            if choices[1] { 20 } else { 30 },
        ));
    }
    inspect(&native, &entries, u64::from(selected));
    assert_eq!(native.context.private_state, if selected { 7 } else { 5 });
    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
    assert_eq!(
        atoms(&prepared.input),
        vec![
            if selected { vec![1] } else { vec![] },
            "λ".as_bytes().to_vec(),
            b"other".to_vec(),
            vec![20],
            vec![30]
        ]
    );
    assert_eq!(
        atoms(&prepared.output),
        vec![if expected { vec![1] } else { vec![] }]
    );
    assert_ts(&reference, &prepared.input, &prepared.output, &native);
}
#[test]
fn map_insert_keeps_key_before_value_once_and_skips_unselected_witnesses() {
    for selected in [false, true] {
        for key in [false, true] {
            for value in [false, true] {
                run(
                    &format!(
                        "lazy-{}{}{}",
                        u8::from(selected),
                        u8::from(key),
                        u8::from(value)
                    ),
                    selected,
                    [key, value],
                    None,
                );
            }
        }
    }
}
#[test]
fn map_insert_witness_refusal_has_exact_prefix_and_false_branch_suppression() {
    for fail in ["pick_key", "pick_value"] {
        for selected in [false, true] {
            run(
                &format!("failure-{fail}-{selected}"),
                selected,
                [true, true],
                Some(fail),
            );
        }
    }
}
