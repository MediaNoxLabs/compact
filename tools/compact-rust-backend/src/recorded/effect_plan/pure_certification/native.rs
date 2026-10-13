// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Temporary bridge: reuse native pure typing, immediately discard its syntax.
//! Signature carriers use the shared syntax-free checker. Body typing consumes
//! one native fragment; its binding symbols and both output views still construct
//! syntax until the complete Expr fold cutover.
//! No whole-circuit recorded profile or shape recognizer participates.
use crate::RenderError;
use crate::ir::{PureCircuit, Type};
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

/// The native emitter accepts Default syntax without checking Rust trait bounds.
/// Guard only this operation: constructed large tuples remain valid local values.
/// Inputs come from the exhaustive bounded borrowed source visitor above.
pub(super) fn check_defaults(defaults: &[&Type], path: &str) -> Result<(), super::RecordingGap> {
    for ty in defaults {
        let nodes = crate::resource_limits::recording_type_nodes(ty)
            .map_err(|error| super::invalid(path, error.to_string()))?;
        if !super::super::validate::default_tuple_traits_supported(&nodes) {
            return Err(super::invalid(
                path,
                "pure Default requires tuple arity at most 12, including nested vector elements",
            ));
        }
    }
    Ok(())
}

/// All named types in a reached body are emitted with representation derives,
/// even when the value is constructed, unused, or absent from the signature.
pub(super) fn check_named_structs(types: &[&Type], path: &str) -> Result<(), super::RecordingGap> {
    for ty in types {
        if matches!(ty, Type::Struct { .. })
            && !super::super::validate::named_struct_derives_supported(ty)
        {
            return Err(super::invalid(
                path,
                "pure named Struct requires supported representation derives (tuple arity at most 8)",
            ));
        }
    }
    Ok(())
}
