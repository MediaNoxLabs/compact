// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Syntax-free formal signature and body checks for reachable pure declarations.
//! The complete generic fold owns body semantics; this owner preserves formal
//! name/carrier/duplicate/result ordering and separate representation obligations.
use crate::RenderError;
use crate::ir::{PureCircuit, Type};
use crate::pure_semantics::{
    body_check::{BodyCheckSink, ObservedBodyCheckSink},
    expression::fold_expression,
};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use std::collections::HashMap;

pub(super) fn check(
    source: &PureCircuit,
    pure: &HashMap<&str, &PureCircuit>,
) -> Result<(), RenderError> {
    check_observed(source, pure, None)
}

pub(super) fn check_observed(
    source: &PureCircuit,
    pure: &HashMap<&str, &PureCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), RenderError> {
    crate::pure_semantics::identifier::check_name(&source.name).inspect_err(|_| {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::field(Owner::Pure(source), "name"));
        }
    })?;
    let mut scope = HashMap::new();
    for parameter in &source.parameters {
        crate::pure_semantics::identifier::check_name(&parameter.name).inspect_err(|_| {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(Owner::Parameter(parameter), "name"));
            }
        })?;
        crate::pure_semantics::carrier::check_carrier(&parameter.ty).inspect_err(|_| {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::node(Owner::Type(&parameter.ty)));
            }
        })?;
        if scope
            .insert(parameter.name.as_str(), (&parameter.ty, ()))
            .is_some()
        {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(Owner::Parameter(parameter), "name"));
            }
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
    }
    crate::pure_semantics::carrier::check_carrier(&source.result).inspect_err(|_| {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::node(Owner::Type(&source.result)));
        }
    })?;
    let mut callback = |expr: &crate::ir::Expr, _: &RenderError| {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::node(Owner::Expr(expr)));
        }
    };
    let actual = if observer.is_some() {
        fold_expression(
            &source.body,
            &scope,
            pure,
            &mut ObservedBodyCheckSink::new(&mut callback),
        )?
        .ty
    } else {
        fold_expression(&source.body, &scope, pure, &mut BodyCheckSink::new())?.ty
    };
    if actual != source.result {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::node(Owner::Type(&source.result)));
        }
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
    check_defaults_observed(defaults, path, None)
}
pub(super) fn check_defaults_observed(
    defaults: &[&Type],
    path: &str,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), super::RecordingGap> {
    for ty in defaults {
        let nodes = crate::resource_limits::recording_type_nodes(ty)
            .map_err(|error| super::invalid(path, error.to_string()))?;
        if !super::super::validate::default_tuple_traits_supported(&nodes) {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::node(Owner::Type(ty)));
            }
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
    check_named_structs_observed(types, path, None)
}
pub(super) fn check_named_structs_observed(
    types: &[&Type],
    path: &str,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), super::RecordingGap> {
    for ty in types {
        if matches!(ty, Type::Struct { .. })
            && !super::super::validate::named_struct_derives_supported(ty)
        {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::node(Owner::Type(ty)));
            }
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

#[cfg(test)]
mod resource_measurement;
