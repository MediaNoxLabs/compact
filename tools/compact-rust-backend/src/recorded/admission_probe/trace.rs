// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only observations of the actual recorder. Hooks receive already
//! evaluated results; no recognizer is repeated and observations never select.
use super::super::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ir::StatefulCircuit};
use serde::Serialize;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    PublicEmission,
    HelperPlanning,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Attempt {
    route: &'static str,
    produced_candidate: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Guard {
    route: &'static str,
    passed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum Outcome {
    Supported,
    Unsupported { gap: RecordingGap },
    Error { error: String },
    Aborted,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Invocation {
    pub id: usize,
    pub parent: Option<usize>,
    pub phase: Phase,
    pub circuit: String,
    pub internal: bool,
    pub helper: bool,
    /// Root IR category is descriptive data, never an admission rule.
    pub return_kind: String,
    /// Candidate ownership does not imply final support; see `outcome`.
    pub body_owner: Option<&'static str>,
    pub action_dispatch: Option<&'static str>,
    pub return_dispatch: Option<&'static str>,
    pub attempts: Vec<Attempt>,
    pub guards: Vec<Guard>,
    pub outcome: Outcome,
}

struct State {
    phase: Phase,
    stack: Vec<usize>,
    invocations: Vec<Invocation>,
}
thread_local! {
    static CAPTURE: RefCell<Option<State>> = const { RefCell::new(None) };
}

pub(super) struct Capture;
impl Capture {
    pub fn begin() -> Self {
        CAPTURE.with_borrow_mut(|state| {
            assert!(state.is_none(), "nested capture sessions are unsupported");
            *state = Some(State {
                phase: Phase::PublicEmission,
                stack: vec![],
                invocations: vec![],
            });
        });
        Self
    }
    pub fn finish(self) -> Vec<Invocation> {
        let state = CAPTURE
            .with_borrow_mut(Option::take)
            .expect("active trace capture");
        assert!(state.stack.is_empty(), "unfinished recording invocation");
        state.invocations
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        // Also reset state on compiler panic; the next independent test/capture
        // must not inherit its invocation stack or phase.
        CAPTURE.with_borrow_mut(|state| *state = None);
    }
}

/// Inspect actual selector events without exposing private trace record types.
pub(in crate::recorded) fn capture_json<T>(run: impl FnOnce() -> T) -> (T, serde_json::Value) {
    let capture = Capture::begin();
    let result = run();
    let events = serde_json::to_value(capture.finish()).expect("serializable trace events");
    (result, events)
}

pub(in crate::recorded) struct HelperPlanning(Option<Phase>);
impl HelperPlanning {
    pub fn begin() -> Self {
        Self(CAPTURE.with_borrow_mut(|state| {
            state
                .as_mut()
                .map(|state| std::mem::replace(&mut state.phase, Phase::HelperPlanning))
        }))
    }
}
impl Drop for HelperPlanning {
    fn drop(&mut self) {
        if let Some(previous) = self.0 {
            CAPTURE.with_borrow_mut(|state| {
                if let Some(state) = state {
                    state.phase = previous;
                }
            });
        }
    }
}

pub(in crate::recorded) struct Scope(Option<usize>);
impl Scope {
    pub fn begin(circuit: &StatefulCircuit, helper: bool) -> Self {
        Self(CAPTURE.with_borrow_mut(|state| {
            let state = state.as_mut()?;
            let id = state.invocations.len();
            state.invocations.push(Invocation {
                id,
                parent: state.stack.last().copied(),
                phase: state.phase,
                circuit: circuit.name.clone(),
                internal: circuit.internal,
                helper,
                return_kind: RecordingGap::returned(&circuit.return_value).ir_node,
                body_owner: None,
                action_dispatch: None,
                return_dispatch: None,
                attempts: vec![],
                guards: vec![],
                outcome: Outcome::Aborted,
            });
            state.stack.push(id);
            Some(id)
        }))
    }
    pub fn finish<T>(mut self, result: &Result<RecordingOutcome<T>, RenderError>) {
        if let Some(id) = self.0.take() {
            CAPTURE.with_borrow_mut(|state| {
                let state = state.as_mut().expect("scope belongs to active capture");
                assert_eq!(
                    state.stack.pop(),
                    Some(id),
                    "recording scopes finish in order"
                );
                state.invocations[id].outcome = match result {
                    Ok(RecordingOutcome::Supported(_)) => Outcome::Supported,
                    Ok(RecordingOutcome::Unsupported(gap)) => {
                        Outcome::Unsupported { gap: gap.clone() }
                    }
                    Err(error) => Outcome::Error {
                        error: error.to_string(),
                    },
                };
            });
        }
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(id) = self.0 {
            CAPTURE.with_borrow_mut(|state| {
                if let Some(state) = state {
                    debug_assert_eq!(state.stack.last(), Some(&id));
                    state.stack.pop();
                }
            });
        }
    }
}

fn current(update: impl FnOnce(&mut Invocation)) {
    CAPTURE.with_borrow_mut(|state| {
        if let Some(state) = state
            && let Some(id) = state.stack.last()
        {
            update(&mut state.invocations[*id]);
        }
    });
}

pub(in crate::recorded) fn option(route: &'static str, produced_candidate: bool) {
    current(|row| {
        row.attempts.push(Attempt {
            route,
            produced_candidate,
        });
        // Existing eager `or` and lazy `or_else` chains keep their earliest
        // candidate. Later eager successes are recorded without overwriting it.
        if produced_candidate {
            row.body_owner.get_or_insert(route);
        }
    });
}

pub(in crate::recorded) fn handled(route: &'static str) {
    option(route, true);
}

pub(in crate::recorded) fn action(route: &'static str) {
    current(|row| {
        row.action_dispatch = Some(route);
        row.body_owner.get_or_insert(route);
    });
}

pub(in crate::recorded) fn returned() {
    current(|row| {
        row.return_dispatch = Some("return.dispatch");
        row.body_owner.get_or_insert("return.dispatch");
    });
}
