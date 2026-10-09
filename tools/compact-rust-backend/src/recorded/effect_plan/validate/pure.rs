// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Only immutable, body-bound certificates authorize context-free invocation.
use super::*;

pub(super) fn validate(
    binding: &Binding,
    declarations: &Declarations,
    scope: &HashMap<usize, Type>,
    state: &mut Validation,
) -> Result<(), RecordingGap> {
    let path = &binding.path;
    let Operation::PureCall {
        declaration,
        arguments,
    } = &binding.operation
    else {
        return Err(invalid(path, "expected pure call operation"));
    };
    let certified = declarations
        .certified_pure(*declaration)
        .ok_or_else(|| invalid(path, "missing or mismatched pure body certificate"))?;
    let source = certified.source();
    if source.result != binding.value.ty
        || !surface_type(&source.result)
        || source.parameters.len() != arguments.len()
    {
        return Err(invalid(path, "pure call differs from certified signature"));
    }
    for (argument, parameter) in arguments.iter().zip(&source.parameters) {
        used(argument, scope, path)?;
        if argument.ty != parameter.ty || !surface_type(&parameter.ty) {
            return Err(invalid(
                path,
                "pure argument differs from certified signature",
            ));
        }
    }
    if state
        .active_calls
        .len()
        .saturating_add(certified.call_depth())
        > crate::resource_limits::recording_call_limits().0
    {
        return Err(invalid(
            path,
            "combined recording/pure call depth limit exceeded",
        ));
    }
    state.charge(certified.expanded_work(), path)
}
