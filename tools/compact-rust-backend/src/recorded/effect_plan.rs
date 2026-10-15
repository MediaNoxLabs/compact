// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Syntax-independent recording plans with staged admission (ADR-0355).
//! Checking promises this bounded fragment can render, not transaction readiness
//! or proof validity. Replay, transcript partitioning and proof remain runtime gates.

mod context_free;
mod declarations;
mod lower;
use crate::source_diagnostics::RefusalObserver;
mod model;
#[cfg(all(test, feature = "experimental-recording-plan"))]
mod property_tests;
#[cfg(test)]
mod public_effects;
mod pure_certification;
mod rejection;
mod render;
#[cfg(all(test, feature = "experimental-recording-plan"))]
mod resource_tests;
#[cfg(test)]
mod tests;
mod type_environment;
mod validate;

use super::{RecordingGap, RecordingGapCode, recording_plan::TypedPlan};
use crate::ir::{LedgerField, PureCircuit, StatefulCircuit, WitnessDeclaration};
pub(super) use model::CheckedPlan;
#[cfg(test)]
pub(super) use public_effects::has_public_program;
pub(super) use rejection::{DiagnosticPlanError, LoweringRejection};
use std::collections::HashMap;

/// Historical gap-only adapter used by test oracles. Production retains typed
/// refusal provenance; Invalid always aborts generation.
#[cfg(test)]
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
    check_with_calls(circuit, ledger, witnesses, &HashMap::new(), &HashMap::new())
}

#[cfg(test)]
pub(super) fn check_with_calls(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<CheckedPlan, PlanError> {
    check_with_calls_diagnostic(circuit, ledger, witnesses, pure, circuits).map_err(Into::into)
}

#[cfg(test)]
pub(super) fn check_with_calls_diagnostic(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<CheckedPlan, DiagnosticPlanError> {
    check_with_calls_observed(circuit, ledger, witnesses, pure, circuits, None)
}
pub(super) fn check_with_calls_observed(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<CheckedPlan, DiagnosticPlanError> {
    let types = type_environment::TypeEnvironment::from_sources_observed(
        circuit, ledger, witnesses, pure, circuits, observer,
    )
    .map_err(|gap| DiagnosticPlanError::Unsupported(gap.into()))?;
    let declarations = declarations::Declarations::with_calls(ledger, witnesses, pure, circuits)
        .with_type_environment(types)
        .with_root(circuit);
    let (plan, certificates) = if observer.is_some() {
        lower::lower_with_pure_calls_observed(circuit, &declarations, pure, circuits, observer)
    } else {
        lower::lower_with_pure_calls(circuit, &declarations, pure, circuits)
    }
    .map_err(DiagnosticPlanError::Unsupported)?;
    let declarations = declarations
        .with_certificates(certificates, pure)
        .map_err(DiagnosticPlanError::Invalid)?;
    validate::check(plan, declarations).map_err(DiagnosticPlanError::Invalid)
}

#[cfg(test)]
pub(super) fn lower_with_calls(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<TypedPlan, PlanError> {
    lower_with_calls_diagnostic(circuit, ledger, witnesses, pure, circuits).map_err(Into::into)
}

#[cfg(test)]
pub(super) fn lower_with_calls_diagnostic(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<TypedPlan, DiagnosticPlanError> {
    lower_with_calls_observed(circuit, ledger, witnesses, pure, circuits, None)
}
pub(super) fn lower_with_calls_observed(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<TypedPlan, DiagnosticPlanError> {
    render(&check_with_calls_observed(
        circuit, ledger, witnesses, pure, circuits, observer,
    )?)
    .map_err(DiagnosticPlanError::Invalid)
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
        resource: None,
        code: RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: path.into(),
        detail: detail.into(),
    }
}
