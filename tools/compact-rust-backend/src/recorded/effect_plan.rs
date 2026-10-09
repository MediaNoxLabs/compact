// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Experimental syntax-independent recording plans (ADR-0355).
//! Checking promises this bounded fragment can render, not transaction readiness
//! or proof validity. Replay, transcript partitioning and proof remain runtime gates.

mod lower;
mod model;
mod render;
#[cfg(test)]
mod resource_tests;
#[cfg(test)]
mod tests;
mod validate;

use super::{RecordingGap, RecordingGapCode, typed_plan::TypedPlan};
use crate::ir::{LedgerField, StatefulCircuit, WitnessDeclaration};
pub(super) use model::CheckedPlan;
use std::collections::HashMap;

/// Only Unsupported is eligible for a legacy attempt. Invalid represents a
/// violated compiler invariant after lowering and must abort generation.
#[derive(Debug)]
pub(super) enum PlanError {
    Unsupported(RecordingGap),
    Invalid(RecordingGap),
}

pub(super) fn check(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
) -> Result<CheckedPlan, PlanError> {
    let plan = lower::lower(circuit, ledger, witnesses).map_err(PlanError::Unsupported)?;
    validate::check(plan).map_err(PlanError::Invalid)
}

pub(super) fn render(plan: &CheckedPlan) -> Result<TypedPlan, RecordingGap> {
    render::render(plan)
}

pub(super) fn lower(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
) -> Result<TypedPlan, PlanError> {
    render(&check(circuit, ledger, witnesses)?).map_err(PlanError::Invalid)
}

fn invalid(path: &str, detail: impl Into<String>) -> RecordingGap {
    RecordingGap {
        code: RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: path.into(),
        detail: detail.into(),
    }
}
