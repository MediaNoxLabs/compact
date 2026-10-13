// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Syntax-free formal signature and body checks for reachable pure declarations.
//! The complete generic fold owns body semantics; this owner preserves formal
//! name/carrier/duplicate/result ordering and separate representation obligations.
use crate::RenderError;
use crate::ir::{PureCircuit, Type};
use crate::pure_semantics::{body_check::BodyCheckSink, expression::fold_expression};
use std::collections::HashMap;

pub(super) fn check(
    source: &PureCircuit,
    pure: &HashMap<&str, &PureCircuit>,
) -> Result<(), RenderError> {
    crate::pure_semantics::identifier::check_name(&source.name)?;
    let mut scope = HashMap::new();
    for parameter in &source.parameters {
        crate::pure_semantics::identifier::check_name(&parameter.name)?;
        crate::pure_semantics::carrier::check_carrier(&parameter.ty)?;
        if scope
            .insert(parameter.name.as_str(), (&parameter.ty, ()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
    }
    crate::pure_semantics::carrier::check_carrier(&source.result)?;
    let actual = fold_expression(&source.body, &scope, pure, &mut BodyCheckSink::new())?.ty;
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

#[cfg(test)]
mod tests;
