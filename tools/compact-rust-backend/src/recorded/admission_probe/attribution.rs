// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Join actual final export invocations to compiler capabilities. Candidate-only
//! lowering stays a separate observation, never a substitute for selected owner.
use super::{
    Candidate, Row,
    trace::{Invocation, Outcome, Phase},
};
use crate::RustCapabilityReport;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub(super) struct Export {
    pub circuit: String,
    pub return_kind: Option<String>,
    pub invocation_id: Option<usize>,
    pub body_owner: Option<&'static str>,
    pub action_dispatch: Option<&'static str>,
    pub return_dispatch: Option<&'static str>,
    pub recorded: bool,
    pub observed_call: bool,
    pub outcome: Option<Outcome>,
    pub candidate: Option<Candidate>,
}

#[derive(Debug, Serialize)]
pub(super) struct Attribution {
    pub exports: Vec<Export>,
    pub errors: Vec<String>,
}

pub(super) fn join(
    report: &RustCapabilityReport,
    candidates: &[Row],
    invocations: &[Invocation],
) -> Attribution {
    let mut exports = Vec::new();
    let mut errors = Vec::new();
    for capability in &report.circuits {
        let actual: Vec<_> = invocations
            .iter()
            .filter(|row| {
                row.phase == Phase::PublicEmission
                    && !row.helper
                    && !row.internal
                    && row.circuit == capability.name
            })
            .collect();
        let candidate: Vec<_> = candidates
            .iter()
            .filter(|row| row.circuit == capability.name)
            .collect();
        if actual.len() != 1 {
            errors.push(format!(
                "{}: expected one final export invocation, got {}",
                capability.name,
                actual.len()
            ));
        }
        if candidate.len() != 1 {
            errors.push(format!(
                "{}: expected one candidate observation, got {}",
                capability.name,
                candidate.len()
            ));
        }
        let actual = (actual.len() == 1).then(|| actual[0]);
        if let Some(actual) = actual {
            if matches!(actual.outcome, Outcome::Supported) != capability.recorded {
                errors.push(format!(
                    "{}: final recording outcome disagrees with capability",
                    capability.name
                ));
            }
            if capability.recorded && actual.body_owner.is_none() {
                errors.push(format!(
                    "{}: supported export has no attributed owner",
                    capability.name
                ));
            }
            if let Outcome::Unsupported { gap } = &actual.outcome
                && capability.recording_unavailable.as_ref() != Some(gap)
            {
                errors.push(format!(
                    "{}: final refusal disagrees with capability",
                    capability.name
                ));
            }
        }
        exports.push(Export {
            circuit: capability.name.clone(),
            return_kind: actual.map(|row| row.return_kind.clone()),
            invocation_id: actual.map(|row| row.id),
            body_owner: actual.and_then(|row| row.body_owner),
            action_dispatch: actual.and_then(|row| row.action_dispatch),
            return_dispatch: actual.and_then(|row| row.return_dispatch),
            recorded: capability.recorded,
            observed_call: capability.observed_call,
            outcome: actual.map(|row| row.outcome.clone()),
            candidate: (candidate.len() == 1).then(|| candidate[0].candidate.clone()),
        });
    }
    Attribution { exports, errors }
}

#[derive(Debug, Default, Serialize)]
pub(super) struct Counts {
    exports: usize,
    recorded: usize,
    candidate_supported: usize,
    recorded_and_candidate_supported: usize,
}

pub(super) fn by_owner(exports: &[Export]) -> BTreeMap<String, Counts> {
    let mut owners = BTreeMap::<String, Counts>::new();
    for export in exports {
        let candidate = matches!(export.candidate, Some(Candidate::Supported));
        let count = owners
            .entry(export.body_owner.unwrap_or("no-selected-owner").into())
            .or_default();
        count.exports += 1;
        count.recorded += usize::from(export.recorded);
        count.candidate_supported += usize::from(candidate);
        count.recorded_and_candidate_supported += usize::from(export.recorded && candidate);
    }
    owners
}
