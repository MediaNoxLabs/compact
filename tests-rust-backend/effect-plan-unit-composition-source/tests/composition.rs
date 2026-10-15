// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, witness::*, *};
use compact_rust_effect_plan_unit_composition_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Entry,
};
struct Scenario<'a> {
    id: &'a str,
    key: &'a str,
    enabled: bool,
    selected: bool,
    choices: [bool; 2],
    fail: Option<&'static str>,
}
impl Scenario<'_> {
    fn expected_events(&self) -> Vec<Event> {
        if !self.enabled || !self.selected {
            return vec![];
        }
        if self.fail == Some("first") || !self.choices[0] {
            return vec![("first", 5, 0)];
        }
        vec![("first", 5, 0), ("second", 6, 1)]
    }
    fn error(&self) -> Option<r::CompactError> {
        if !self.enabled {
            return Some(denied("disabled"));
        }
        if !self.selected {
            return None;
        }
        if self.fail == Some("first") {
            return Some(provider_failure("first"));
        }
        if !self.choices[0] {
            return Some(denied("first denied"));
        }
        if self.fail == Some("second") {
            return Some(provider_failure("second"));
        }
        if !self.choices[1] {
            return Some(denied("second denied"));
        }
        None
    }
}
fn exercise(s: Scenario<'_>) {
    let (state, verifier) = setup();
    let (a, b, m, w) = (
        Witness::new(s.choices, s.fail),
        Witness::new(s.choices, s.fail),
        Witness::new(s.choices, s.fail),
        Witness::new(s.choices, s.fail),
    );
    let native = c::run(
        state.circuit_context(5),
        &a,
        s.key.into(),
        s.enabled,
        s.selected,
    );
    let recorded = c::recorded::run(
        state.circuit_context(5),
        &b,
        s.key.into(),
        s.enabled,
        s.selected,
    );
    let manual = manual(
        state.circuit_context(5),
        &m,
        s.key.into(),
        s.enabled,
        s.selected,
    );
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .run_call(&state, 5, s.key.into(), s.enabled, s.selected);
    let events = s.expected_events();
    for witness in [&a, &b, &m, &w] {
        assert_eq!(*witness.events.borrow(), events, "{}", s.id);
    }
    let reference = row(s.id);
    // Explicit configuration identity distinguishes equal-output unselected cases.
    assert_eq!(reference["circuit"], "run");
    assert_eq!(
        reference["args"],
        serde_json::json!([s.key, s.enabled, s.selected])
    );
    assert_eq!(reference["seed"], "seed");
    assert_eq!(reference["choices"], serde_json::json!(s.choices));
    assert_eq!(reference["fail"], serde_json::json!(s.fail));
    assert_eq!(
        reference["events"],
        serde_json::json!(
            events
                .iter()
                .map(|(name, p, count)| serde_json::json!([name, p, count.to_string()]))
                .collect::<Vec<_>>()
        )
    );
    // Observed calls borrow an immutable prestate even when a later witness refuses.
    let original = state.circuit_context(5);
    assert_eq!(
        slots::calls
            .inspect(original.query.state.get_ref())
            .unwrap(),
        0
    );
    assert!(!slots::done.inspect(original.query.state.get_ref()).unwrap());
    assert_eq!(
        slots::entries
            .inspect(original.query.state.get_ref())
            .unwrap()
            .size()
            .unwrap()
            .value(),
        1
    );
    if let Some(error) = s.error() {
        assert_eq!(native.err().unwrap(), error, "{}", s.id);
        assert_eq!(recorded.err().unwrap(), error, "{}", s.id);
        assert_eq!(manual.err().unwrap(), error, "{}", s.id);
        assert_eq!(call.err().unwrap(), error, "{}", s.id);
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
    let _: () = native.result;
    assert_eq!(native.context.private_state, if s.selected { 7 } else { 5 });
    let final_state = native.context.query.state.get_ref();
    assert_eq!(
        slots::calls.inspect(final_state).unwrap(),
        if s.selected { 2 } else { 0 }
    );
    assert_eq!(slots::done.inspect(final_state).unwrap(), s.selected);
    let entries = slots::entries.inspect(final_state).unwrap();
    assert_eq!(
        entries.size().unwrap().value(),
        if s.selected && s.key != "seed" { 2 } else { 1 }
    );
    assert_eq!(
        entries.lookup("seed".into()).unwrap(),
        Entry {
            label: "seed".into()
        }
    );
    if s.selected {
        assert_eq!(
            entries.lookup(s.key.into()).unwrap(),
            Entry {
                label: s.key.into()
            }
        );
    }
    assert_eq!(
        native
            .private_transcript_outputs
            .iter()
            .map(atoms)
            .collect::<Vec<_>>(),
        if s.selected {
            vec![vec![vec![1]], vec![vec![1]]]
        } else {
            vec![]
        }
    );
    if !s.selected {
        // Successful local/observed execution with no public effects cannot form
        // a ledger call. Preserve the existing transaction adapter boundary.
        assert!(matches!(
            call.prepare(verifier, r::Field::from(0_u64)),
            Err(r::transaction::ObservedCallError::Prepare(
                r::transaction::PrepareCallError::EmptyTranscript
            ))
        ));
        assert_eq!(reference["rustPreparationExpectation"], "EmptyTranscript");
        let input =
            r::fab::AlignedValue::from((r::OpaqueString::from(s.key), s.enabled, s.selected));
        assert_ts(&reference, &input, &r::fab::AlignedValue::from(()), &native);
        return;
    }
    assert_eq!(
        reference["rustPreparationExpectation"],
        "nonempty ledger call"
    );
    let prepared = call.prepare(verifier, r::Field::from(0_u64)).unwrap();
    assert_eq!(
        atoms(&prepared.input),
        vec![
            s.key.as_bytes().to_vec(),
            vec![1],
            if s.selected { vec![1] } else { vec![] }
        ]
    );
    assert!(atoms(&prepared.output).is_empty());
    assert_eq!(
        prepared.output.alignment,
        r::fab::AlignedValue::from(()).alignment
    );
    assert_ts(&reference, &prepared.input, &prepared.output, &native);
}
#[test]
fn broad_unit_inputs_keep_order_across_pure_guard_helpers_and_state_effects() {
    for (id, key) in [
        ("new", "new"),
        ("overwrite", "seed"),
        ("unicode-nul", "λ\0key"),
        ("empty", ""),
    ] {
        exercise(Scenario {
            id: &format!("selected-{id}"),
            key,
            enabled: true,
            selected: true,
            choices: [true, true],
            fail: None,
        });
    }
}
#[test]
fn lazy_unit_branch_skips_all_provider_answers_and_refusals() {
    for first in [false, true] {
        for second in [false, true] {
            exercise(Scenario {
                id: &format!("unselected-{}{}", u8::from(first), u8::from(second)),
                key: "new",
                enabled: true,
                selected: false,
                choices: [first, second],
                fail: None,
            });
        }
    }
    for failed in ["first", "second"] {
        exercise(Scenario {
            id: &format!("suppressed-{failed}"),
            key: "new",
            enabled: true,
            selected: false,
            choices: [true, true],
            fail: Some(failed),
        });
    }
}
#[test]
fn pure_guard_and_witness_refusals_stop_at_the_exact_ordered_prefix() {
    for selected in [false, true] {
        exercise(Scenario {
            id: &format!("disabled-{selected}"),
            key: "new",
            enabled: false,
            selected,
            choices: [true, true],
            fail: Some("first"),
        });
    }
    for (id, choices) in [
        ("first-denied", [false, true]),
        ("second-denied", [true, false]),
    ] {
        exercise(Scenario {
            id,
            key: "new",
            enabled: true,
            selected: true,
            choices,
            fail: None,
        });
    }
    for failed in ["first", "second"] {
        exercise(Scenario {
            id: &format!("provider-{failed}"),
            key: "new",
            enabled: true,
            selected: true,
            choices: [true, true],
            fail: Some(failed),
        });
    }
}
