// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Syntax-independent recording plans with staged admission (ADR-0355).
//! Checking promises this bounded fragment can render, not transaction readiness
//! or proof validity. Replay, transcript partitioning and proof remain runtime gates.

mod declarations;
mod lower;
mod model;
#[cfg(all(test, feature = "experimental-recording-plan"))]
mod property_tests;
mod render;
#[cfg(all(test, feature = "experimental-recording-plan"))]
mod resource_tests;
#[cfg(test)]
mod tests;
mod validate;

use super::{RecordingGap, RecordingGapCode, typed_plan::TypedPlan};
use crate::ir::{LedgerField, PureCircuit, StatefulCircuit, WitnessDeclaration};
pub(super) use model::CheckedPlan;
use std::collections::HashMap;

/// Only Unsupported is eligible for a legacy attempt. Invalid represents a
/// violated compiler invariant after lowering and must abort generation.
#[derive(Debug)]
pub(super) enum PlanError {
    Unsupported(RecordingGap),
    Invalid(RecordingGap),
}

#[cfg(test)]
pub(super) fn check(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
) -> Result<CheckedPlan, PlanError> {
    let declarations = declarations::Declarations::new(ledger, witnesses).with_root(circuit);
    let plan = lower::lower(circuit, &declarations).map_err(PlanError::Unsupported)?;
    validate::check(plan, declarations).map_err(PlanError::Invalid)
}

pub(super) fn check_with_calls(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<CheckedPlan, PlanError> {
    let declarations = declarations::Declarations::with_calls(ledger, witnesses, pure, circuits)
        .with_root(circuit);
    let plan = lower::lower_with_calls(circuit, &declarations, circuits)
        .map_err(PlanError::Unsupported)?;
    validate::check(plan, declarations).map_err(PlanError::Invalid)
}

pub(super) fn lower_with_calls(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<TypedPlan, PlanError> {
    render(&check_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    )?)
    .map_err(PlanError::Invalid)
}

pub(super) fn render(plan: &CheckedPlan) -> Result<TypedPlan, RecordingGap> {
    render::render(plan)
}

#[cfg(test)]
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
