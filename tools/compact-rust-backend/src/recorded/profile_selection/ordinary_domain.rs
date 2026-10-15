// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Canonical declared interfaces and checked composition own ordinary admission.
use super::*;

pub(super) fn attempt(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    let checked = match effect_plan::check_with_calls_diagnostic(
        circuit, ledger, witnesses, pure, circuits,
    ) {
        Ok(plan) => plan,
        Err(error) => return failure(error),
    };
    // A rendering failure after independent validation is an invariant error,
    // never permission to retry legacy generation.
    match effect_plan::render(&checked) {
        Ok(plan) => Ok(Some(plan)),
        Err(gap) => failure(effect_plan::DiagnosticPlanError::Invalid(gap)),
    }
}

fn failure(error: effect_plan::DiagnosticPlanError) -> Result<Option<TypedPlan>, RenderError> {
    match error {
        // Preserve existing legacy/Unit/shielded diagnostic precedence. A later
        // checked refusal is not evidence that an earlier legacy error can move.
        effect_plan::DiagnosticPlanError::Unsupported(_) => Ok(None),
        effect_plan::DiagnosticPlanError::Invalid(gap) => {
            Err(RenderError::RecordingPlanInvariant {
                path: gap.path,
                detail: gap.detail,
            })
        }
    }
}

#[cfg(test)]
#[path = "ordinary_domain/tests.rs"]
mod tests;
