// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! A call imports evaluated arguments, never its caller's lexical environment.
use super::*;
use crate::recorded::effect_plan::declarations::CallKind;

pub(super) fn validate(
    binding: &Binding,
    declarations: &Declarations,
    caller: &HashMap<usize, Type>,
    state: &mut Validation,
) -> Result<(), RecordingGap> {
    let path = &binding.path;
    let result = &binding.value.ty;
    let Operation::Call {
        declaration: id,
        arguments,
        parameters,
        body,
    } = &binding.operation
    else {
        return Err(invalid(path, "expected call operation"));
    };
    let signature = declarations
        .call(*id)
        .ok_or_else(|| invalid(path, "unknown or ambiguous call declaration"))?;
    if signature.kind != CallKind::Stateful {
        return Err(invalid(path, "expected stateful call declaration"));
    }
    if state
        .active_calls
        .iter()
        .any(|name| name == &signature.name)
    {
        return Err(invalid(path, "recursive recording call"));
    }
    if state.active_calls.len() >= crate::resource_limits::recording_call_limits().0 {
        return Err(invalid(path, "recording call depth limit exceeded"));
    }
    if signature.parameters.len() != arguments.len()
        || parameters.len() != arguments.len()
        || signature.result != *result
        || body.result.ty != *result
        || !surface_type(result)
    {
        return Err(invalid(path, "call differs from declared signature"));
    }
    state.charge(parameters.len(), path)?;
    let mut scope = HashMap::new();
    let mut names = HashSet::new();
    for ((formal, argument), parameter) in
        signature.parameters.iter().zip(arguments).zip(parameters)
    {
        used(argument, caller, path)?;
        if !surface_type(&formal.ty)
            || formal.ty != argument.ty
            || formal.ty != parameter.ty
            || !names.insert(&formal.name)
            || !state.allocated.insert(parameter.id)
        {
            return Err(invalid(path, "invalid or duplicate call formal"));
        }
        scope.insert(parameter.id, parameter.ty.clone());
    }
    state.active_calls.push(signature.name.clone());
    block(body, declarations, &scope, state, path)?;
    state.active_calls.pop();
    Ok(())
}
