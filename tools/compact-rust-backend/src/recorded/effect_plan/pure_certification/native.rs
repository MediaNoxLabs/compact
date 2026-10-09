// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Temporary bridge: reuse native pure typing, immediately discard its syntax.
//! No whole-circuit recorded profile or shape recognizer participates.
use crate::RenderError;
use crate::ir::{PureCircuit, Type};
use std::collections::HashMap;

pub(super) fn check(
    source: &PureCircuit,
    pure: &HashMap<&str, &PureCircuit>,
) -> Result<(), RenderError> {
    crate::ident(&source.name)?;
    let mut scope = HashMap::new();
    for (parameter, name) in source
        .parameters
        .iter()
        .zip(crate::naming::parameter_idents(&source.parameters, &[]))
    {
        crate::ident(&parameter.name)?;
        crate::rust_type(&parameter.ty)?;
        if scope
            .insert(parameter.name.as_str(), (&parameter.ty, name))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
    }
    crate::rust_type(&source.result)?;
    let (_, actual) = crate::expression_with_calls(&source.body, &scope, pure)?;
    if actual != source.result {
        return Err(RenderError::TypeMismatch {
            expected: source.result.clone(),
            actual,
        });
    }
    if source.result == Type::Unit {
        let _ = crate::unit_statements(&source.body, &scope, pure)?;
    }
    Ok(())
}
