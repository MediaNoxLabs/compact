// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, witness::*, *};
use compact_rust_effect_plan_aligned_hashes_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Hashes,
};
#[derive(Clone, Copy, PartialEq)]
enum Root {
    Close,
    CloseHash,
    Hashes,
    HashOnly,
}
impl Root {
    fn name(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::CloseHash => "close_hash",
            Self::Hashes => "hashes",
            Self::HashOnly => "hash_only",
        }
    }
    fn is_close(self) -> bool {
        matches!(self, Self::Close | Self::CloseHash)
    }
}
struct Scenario<'a> {
    id: &'a str,
    root: Root,
    value: u64,
    other: u64,
    selected: bool,
    fail: Option<&'static str>,
    mismatch: Option<&'static str>,
}
impl Scenario<'_> {
    fn outcome(&self) -> (Vec<Event>, Option<r::CompactError>) {
        let names: &[&'static str] = match self.root {
            Root::Close => &["expectedPoint", "expectedDigest"],
            Root::CloseHash => &["expectedDigest"],
            Root::Hashes if self.selected => &["curveInput", "hashInput"],
            Root::HashOnly if self.selected => &["hashInput"],
            _ => &[],
        };
        let mut events = vec![];
        for &name in names {
            events.push((
                name,
                5 + events.len() as u64,
                if self.root.is_close() { 0 } else { 1 },
            ));
            if self.fail == Some(name) {
                return (events, Some(provider_failure(name)));
            }
            if self.mismatch == Some(name) {
                return (
                    events,
                    Some(denied(if name == "expectedPoint" {
                        "point mismatch"
                    } else {
                        "digest mismatch"
                    })),
                );
            }
        }
        (events, None)
    }
    fn input(&self) -> r::fab::AlignedValue {
        if self.root.is_close() {
            r::Field::from(self.value).into()
        } else {
            self.selected.into()
        }
    }
    fn output(&self) -> r::fab::AlignedValue {
        if self.root.is_close() {
            ().into()
        } else {
            Hashes {
                x: if self.selected && self.root == Root::Hashes {
                    answers::point(self.value).x().unwrap()
                } else {
                    r::Field::from(0_u64)
                },
                digest: if self.selected {
                    answers::digest(self.other)
                } else {
                    r::FixedBytes([0; 32])
                },
            }
            .into()
        }
    }
}
struct Executions<'a, T> {
    native: Result<r::context::CircuitResult<u64, T>, r::CompactError>,
    recorded: Result<r::recording::RecordedCircuitResult<u64, T>, r::CompactError>,
    manual: Result<r::recording::RecordedCircuitResult<u64, T>, r::CompactError>,
    observed: Result<r::transaction::RecordedCall<'a, u64, T>, r::CompactError>,
}
fn check<T: std::fmt::Debug + PartialEq + Clone + Into<r::fab::AlignedValue>>(
    s: &Scenario<'_>,
    x: Executions<'_, T>,
    witnesses: [&Witness; 4],
    verifier: r::transaction::VerifierKey,
) {
    let (events, error) = s.outcome();
    for w in witnesses {
        assert_eq!(*w.events.borrow(), events, "{}", s.id);
    }
    let reference = row(s.id);
    assert_eq!(reference["circuit"], s.root.name());
    assert_eq!(reference["value"], s.value.to_string());
    assert_eq!(reference["other"], s.other.to_string());
    assert_eq!(reference["selected"], s.selected);
    assert_eq!(reference["fail"], serde_json::json!(s.fail));
    assert_eq!(reference["mismatch"], serde_json::json!(s.mismatch));
    assert_eq!(
        reference["args"],
        if s.root.is_close() {
            serde_json::json!([s.value.to_string()])
        } else {
            serde_json::json!([s.selected])
        }
    );
    assert_eq!(
        reference["events"],
        serde_json::json!(
            events
                .iter()
                .map(|(name, p, c)| serde_json::json!([name, p, c.to_string()]))
                .collect::<Vec<_>>()
        )
    );
    if let Some(error) = error {
        assert_eq!(x.native.err().unwrap(), error, "{}", s.id);
        assert_eq!(x.recorded.err().unwrap(), error, "{}", s.id);
        assert_eq!(x.manual.err().unwrap(), error, "{}", s.id);
        assert_eq!(x.observed.err().unwrap(), error, "{}", s.id);
        assert_eq!(reference["status"], "refused");
        return;
    }
    let (native, recorded, manual, observed) = (
        x.native.unwrap(),
        x.recorded.unwrap(),
        x.manual.unwrap(),
        x.observed.unwrap(),
    );
    compare(&native, &recorded, &manual);
    compare(&native, observed.recorded(), &manual);
    assert_eq!(
        slots::calls
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        1
    );
    assert_eq!(native.context.private_state, 5 + events.len() as u64);
    assert_eq!(native.result.clone().into(), s.output());
    let prepared = observed.prepare(verifier, r::Field::from(0_u64)).unwrap();
    assert_eq!(prepared.input, s.input());
    assert_eq!(prepared.output, s.output());
    assert_ts(&reference, &prepared.input, &prepared.output, &native);
}
fn exercise(s: Scenario<'_>) {
    let (state, verifier) = setup();
    let [a, b, m, w] = std::array::from_fn(|_| Witness::new(s.value, s.other, s.fail, s.mismatch));
    let contract = c::Contract::from(w.clone());
    match s.root {
        Root::Hashes => check(
            &s,
            Executions {
                native: c::hashes(state.circuit_context(5), &a, s.selected),
                recorded: c::recorded::hashes(state.circuit_context(5), &b, s.selected),
                manual: manual_hashes(state.circuit_context(5), &m, s.selected),
                observed: contract.recording().hashes_call(&state, 5, s.selected),
            },
            [&a, &b, &m, &w],
            verifier,
        ),
        Root::HashOnly => check(
            &s,
            Executions {
                native: c::hash_only(state.circuit_context(5), &a, s.selected),
                recorded: c::recorded::hash_only(state.circuit_context(5), &b, s.selected),
                manual: manual_hash_only(state.circuit_context(5), &m, s.selected),
                observed: contract.recording().hash_only_call(&state, 5, s.selected),
            },
            [&a, &b, &m, &w],
            verifier,
        ),
        Root::Close => check(
            &s,
            Executions {
                native: c::close(state.circuit_context(5), &a, r::Field::from(s.value)),
                recorded: c::recorded::close(state.circuit_context(5), &b, r::Field::from(s.value)),
                manual: manual_close(state.circuit_context(5), &m, true),
                observed: contract
                    .recording()
                    .close_call(&state, 5, r::Field::from(s.value)),
            },
            [&a, &b, &m, &w],
            verifier,
        ),
        Root::CloseHash => check(
            &s,
            Executions {
                native: c::close_hash(state.circuit_context(5), &a, r::Field::from(s.value)),
                recorded: c::recorded::close_hash(
                    state.circuit_context(5),
                    &b,
                    r::Field::from(s.value),
                ),
                manual: manual_close(state.circuit_context(5), &m, false),
                observed: contract
                    .recording()
                    .close_hash_call(&state, 5, r::Field::from(s.value)),
            },
            [&a, &b, &m, &w],
            verifier,
        ),
    }
    // Observed state is immutable even when a failure follows Counter.increment.
    assert_eq!(
        slots::calls
            .inspect(state.circuit_context(5).query.state.get_ref())
            .unwrap(),
        0
    );
}
#[test]
fn aligned_hashes_match_frozen_typescript_answers_and_original_fab() {
    for value in [0, 1, 2] {
        for root in [Root::Close, Root::CloseHash, Root::Hashes, Root::HashOnly] {
            exercise(Scenario {
                id: &format!("{}-{value}", root.name()),
                root,
                value,
                other: value,
                selected: true,
                fail: None,
                mismatch: None,
            });
        }
    }
    exercise(Scenario {
        id: "hashes-ordered-distinct",
        root: Root::Hashes,
        value: 1,
        other: 2,
        selected: true,
        fail: None,
        mismatch: None,
    });
}
#[test]
fn lazy_hashes_and_provider_errors_preserve_counter_first_order() {
    for root in [Root::Hashes, Root::HashOnly] {
        exercise(Scenario {
            id: &format!("{}-lazy", root.name()),
            root,
            value: 1,
            other: 2,
            selected: false,
            fail: Some("hashInput"),
            mismatch: None,
        });
        exercise(Scenario {
            id: &format!("{}-hash-refusal", root.name()),
            root,
            value: 1,
            other: 2,
            selected: true,
            fail: Some("hashInput"),
            mismatch: None,
        });
    }
    exercise(Scenario {
        id: "hashes-curve-refusal",
        root: Root::Hashes,
        value: 1,
        other: 2,
        selected: true,
        fail: Some("curveInput"),
        mismatch: None,
    });
}
#[test]
fn local_unit_hash_guards_stop_at_exact_witness_prefixes() {
    for name in ["expectedPoint", "expectedDigest"] {
        exercise(Scenario {
            id: &format!("close-{name}-refusal"),
            root: Root::Close,
            value: 1,
            other: 2,
            selected: true,
            fail: Some(name),
            mismatch: None,
        });
        exercise(Scenario {
            id: &format!("close-{name}-mismatch"),
            root: Root::Close,
            value: 1,
            other: 2,
            selected: true,
            fail: None,
            mismatch: Some(name),
        });
    }
    for kind in ["refusal", "mismatch"] {
        exercise(Scenario {
            id: &format!("close_hash-{kind}"),
            root: Root::CloseHash,
            value: 1,
            other: 2,
            selected: true,
            fail: if kind == "refusal" {
                Some("expectedDigest")
            } else {
                None
            },
            mismatch: if kind == "mismatch" {
                Some("expectedDigest")
            } else {
                None
            },
        });
    }
}
