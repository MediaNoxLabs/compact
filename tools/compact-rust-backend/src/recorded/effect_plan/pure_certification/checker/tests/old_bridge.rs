// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Temporary bridge: reuse native pure typing, immediately discard its syntax.
//! Signature carriers use the shared syntax-free checker. Body typing consumes
//! one native fragment; its binding symbols and both output views still construct
//! syntax until the complete Expr fold cutover.
//! No whole-circuit recorded profile or shape recognizer participates.
use crate::RenderError;
use crate::ir::PureCircuit;
use std::collections::HashMap;

pub(super) fn check(
    source: &PureCircuit,
    pure: &HashMap<&str, &PureCircuit>,
) -> Result<(), RenderError> {
    crate::pure_semantics::identifier::check_name(&source.name)?;
    let mut scope = HashMap::new();
    for (parameter, name) in source
        .parameters
        .iter()
        .zip(crate::naming::parameter_idents(&source.parameters, &[]))
    {
        crate::pure_semantics::identifier::check_name(&parameter.name)?;
        crate::pure_semantics::carrier::check_carrier(&parameter.ty)?;
        if scope
            .insert(parameter.name.as_str(), (&parameter.ty, name))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
    }
    crate::pure_semantics::carrier::check_carrier(&source.result)?;
    let (_, actual) = crate::native_fragment_with_calls(&source.body, &scope, pure)?;
    if actual != source.result {
        return Err(RenderError::TypeMismatch {
            expected: source.result.clone(),
            actual,
        });
    }
    Ok(())
}
