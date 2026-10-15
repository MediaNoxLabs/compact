// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Unit composition uses checked declaration and public-interface authority.
use super::*;
use crate::ir::{StateReturn, Type};

pub(super) fn attempt(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<ProfileAttempt<TypedPlan, effect_plan::LoweringRejection>, RenderError> {
    if circuit.result != Type::Unit || circuit.return_value != StateReturn::Unit {
        return Ok(ProfileAttempt::NotApplicable);
    }
    let checked = match effect_plan::check_with_calls_diagnostic(
        circuit, ledger, witnesses, pure, circuits,
    ) {
        Ok(plan) => plan,
        Err(error) => return failure(error),
    };
    // Recording captures checked execution, including private-only effects.
    // Transaction preparation separately requires a nonempty public transcript.
    match effect_plan::render(&checked) {
        Ok(plan) => Ok(ProfileAttempt::Admitted(plan)),
        Err(gap) => failure(effect_plan::DiagnosticPlanError::Invalid(gap)),
    }
}

fn failure(
    error: effect_plan::DiagnosticPlanError,
) -> Result<ProfileAttempt<TypedPlan, effect_plan::LoweringRejection>, RenderError> {
    match error {
        effect_plan::DiagnosticPlanError::Unsupported(rejection) => {
            Ok(ProfileAttempt::Rejected(rejection))
        }
        effect_plan::DiagnosticPlanError::Invalid(gap) => {
            Err(RenderError::RecordingPlanInvariant {
                path: gap.path,
                detail: gap.detail,
            })
        }
    }
}

#[cfg(test)]
#[path = "unit_domain/tests.rs"]
mod tests;
