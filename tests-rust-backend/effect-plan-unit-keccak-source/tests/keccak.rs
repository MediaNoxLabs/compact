// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{state::*, witness::*, *};
use compact_rust_effect_plan_unit_keccak_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
#[derive(Clone, Copy, PartialEq)]
enum Root {
    Close,
    Lazy,
    Digest,
}
impl Root {
    fn name(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Lazy => "lazy_close",
            Self::Digest => "digest",
        }
    }
}
struct Scenario<'a> {
    id: &'a str,
    root: Root,
    vector: Vector,
    selected: bool,
    mismatch: bool,
    authorized: bool,
    fail: Option<&'static str>,
}
impl Scenario<'_> {
    fn skipped(&self) -> bool {
        self.root == Root::Lazy && !self.selected
    }
    fn outcome(&self) -> (Vec<Event>, Option<r::CompactError>) {
        let mut events = vec![];
        if self.skipped() {
            return (events, None);
        }
        if self.root != Root::Digest || self.selected {
            let name = if self.root == Root::Digest {
                "digestPayload"
            } else {
                "expectedHash"
            };
            events.push((name, 5, true));
            if self.fail == Some(name) {
                return (events, Some(provider_failure(name)));
            }
            if self.root != Root::Digest && self.mismatch {
                return (events, Some(denied("hash mismatch")));
            }
        }
        events.push(("authorized", 5 + events.len() as u64, true));
        let error = if self.fail == Some("authorized") {
            Some(provider_failure("authorized"))
        } else if !self.authorized {
            Some(denied("not authorized"))
        } else {
            None
        };
        (events, error)
    }
    fn input(&self) -> r::fab::AlignedValue {
        match self.root {
            Root::Close => self.vector.payload().into(),
            Root::Lazy => (self.vector.payload(), self.selected).into(),
            Root::Digest => self.selected.into(),
        }
    }
    fn output(&self) -> r::fab::AlignedValue {
        if self.root == Root::Digest {
            if self.selected {
                self.vector.digest()
            } else {
                r::FixedBytes([0; 32])
            }
            .into()
        } else {
            ().into()
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
    assert_eq!(reference["vector"], s.vector.name());
    assert_eq!(reference["selected"], s.selected);
    assert_eq!(reference["mismatch"], s.mismatch);
    assert_eq!(reference["authorized"], s.authorized);
    assert_eq!(reference["fail"], serde_json::json!(s.fail));
    let args = match s.root {
        Root::Close => serde_json::json!([s.vector.payload().0]),
        Root::Lazy => serde_json::json!([s.vector.payload().0, s.selected]),
        Root::Digest => serde_json::json!([s.selected]),
    };
    assert_eq!(reference["args"], args);
    assert_eq!(reference["events"], serde_json::json!(events));
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
        slots::active
            .inspect(native.context.query.state.get_ref())
            .unwrap(),
        s.skipped()
    );
    assert_eq!(native.context.private_state, 5 + events.len() as u64);
    assert_eq!(native.result.clone().into(), s.output());
    if s.skipped() {
        assert!(matches!(
            observed.prepare(verifier, r::Field::from(0_u64)),
            Err(r::transaction::ObservedCallError::Prepare(
                r::transaction::PrepareCallError::EmptyTranscript
            ))
        ));
        assert_eq!(reference["rustPreparationExpectation"], "EmptyTranscript");
        assert_ts(&reference, &s.input(), &s.output(), &native);
    } else {
        let prepared = observed.prepare(verifier, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, s.input());
        assert_eq!(prepared.output, s.output());
        assert_eq!(
            reference["rustPreparationExpectation"],
            "nonempty ledger call"
        );
        assert_ts(&reference, &prepared.input, &prepared.output, &native);
    }
}
fn exercise(s: Scenario<'_>) {
    let (state, verifier) = setup();
    let [a, b, m, w] =
        std::array::from_fn(|_| Witness::new(s.vector, s.mismatch, s.authorized, s.fail));
    let contract = c::Contract::from(w.clone());
    if s.root == Root::Digest {
        let x = Executions {
            native: c::digest(state.circuit_context(5), &a, s.selected),
            recorded: c::recorded::digest(state.circuit_context(5), &b, s.selected),
            manual: manual_digest(state.circuit_context(5), &m, s.selected),
            observed: contract.recording().digest_call(&state, 5, s.selected),
        };
        check(&s, x, [&a, &b, &m, &w], verifier);
    } else {
        let x = if s.root == Root::Close {
            Executions {
                native: c::close(state.circuit_context(5), &a, s.vector.payload()),
                recorded: c::recorded::close(state.circuit_context(5), &b, s.vector.payload()),
                manual: manual_unit(state.circuit_context(5), &m, true),
                observed: contract
                    .recording()
                    .close_call(&state, 5, s.vector.payload()),
            }
        } else {
            Executions {
                native: c::lazy_close(state.circuit_context(5), &a, s.vector.payload(), s.selected),
                recorded: c::recorded::lazy_close(
                    state.circuit_context(5),
                    &b,
                    s.vector.payload(),
                    s.selected,
                ),
                manual: manual_unit(state.circuit_context(5), &m, s.selected),
                observed: contract.recording().lazy_close_call(
                    &state,
                    5,
                    s.vector.payload(),
                    s.selected,
                ),
            }
        };
        check(&s, x, [&a, &b, &m, &w], verifier);
    }
    // All executions borrow immutable observations; even late refusals cannot
    // mutate the caller's original ledger state.
    assert!(
        slots::active
            .inspect(state.circuit_context(5).query.state.get_ref())
            .unwrap()
    );
}
#[test]
fn known_fab_digests_match_typescript_across_unit_and_product_returns() {
    for vector in Vector::ALL {
        for (prefix, root, selected) in [
            ("close", Root::Close, true),
            ("lazy-true", Root::Lazy, true),
            ("digest-true", Root::Digest, true),
            ("digest-false", Root::Digest, false),
        ] {
            exercise(Scenario {
                id: &format!("{prefix}-{}", vector.name()),
                root,
                vector,
                selected,
                mismatch: false,
                authorized: true,
                fail: None,
            });
        }
    }
}
#[test]
fn local_unit_calls_preserve_assertion_and_provider_failure_prefixes() {
    for (kind, mismatch, authorized, fail) in [
        ("hash-refusal", false, true, Some("expectedHash")),
        ("auth-refusal", false, true, Some("authorized")),
        ("mismatch", true, true, None),
        ("denied", false, false, None),
    ] {
        for (prefix, root, selected) in [
            ("lazy-false", Root::Lazy, false),
            ("close", Root::Close, true),
            ("lazy-true", Root::Lazy, true),
        ] {
            exercise(Scenario {
                id: &format!("{prefix}-{kind}"),
                root,
                vector: Vector::Zero,
                selected,
                mismatch,
                authorized,
                fail,
            });
        }
    }
}
#[test]
fn selected_digest_is_lazy_and_authorization_keeps_its_order() {
    for (id, selected, authorized, fail) in [
        (
            "digest-suppressed-payload",
            false,
            true,
            Some("digestPayload"),
        ),
        ("digest-payload-refusal", true, true, Some("digestPayload")),
        ("digest-auth-refusal-false", false, true, Some("authorized")),
        ("digest-auth-refusal-true", true, true, Some("authorized")),
        ("digest-denied-false", false, false, None),
        ("digest-denied-true", true, false, None),
    ] {
        exercise(Scenario {
            id,
            root: Root::Digest,
            vector: Vector::Zero,
            selected,
            mismatch: false,
            authorized,
            fail,
        });
    }
}
