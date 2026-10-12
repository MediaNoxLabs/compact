// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The legacy Unit interface, with body admission owned by checked effects.
use super::*;
use crate::ir::{StateReturn, Type};

// Preserve the old public signature independently of the wider local/FAB domains.
fn parameter_type(ty: &Type) -> bool {
    match ty {
        Type::Boolean
        | Type::Field
        | Type::JubjubPoint
        | Type::Bytes { .. }
        | Type::OpaqueString
        | Type::Enum { .. } => true,
        Type::Unsigned { max } => max.parse::<u64>().is_ok(),
        Type::Struct { fields, .. } => fields.iter().all(|f| parameter_type(&f.ty)),
        Type::Vector { element, .. } => parameter_type(element),
        Type::Tuple { elements } => {
            !elements.is_empty() && elements.len() <= 8 && elements.iter().all(parameter_type)
        }
        _ => false,
    }
}

pub(super) fn attempt(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<ProfileAttempt<TypedPlan, effect_plan::LoweringRejection>, RenderError> {
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !circuit.parameters.iter().all(|p| parameter_type(&p.ty))
    {
        return Ok(ProfileAttempt::NotApplicable);
    }
    let checked = match effect_plan::check_with_calls_diagnostic(
        circuit, ledger, witnesses, pure, circuits,
    ) {
        Ok(plan) => plan,
        Err(error) => return failure(error),
    };
    if !effect_plan::has_public_program(&checked) {
        let mut gap = RecordingGap::no_effect();
        gap.detail = "Unit composition has no replayable public ledger operation".into();
        return Ok(ProfileAttempt::Rejected(gap.into()));
    }
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
