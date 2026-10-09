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
    // The declaration is separate from the immutable certificate. Bound its
    // annotations before certified_pure compares the two signatures.
    let signature = declarations
        .call(*declaration)
        .ok_or_else(|| invalid(path, "unknown pure call declaration"))?;
    declarations.types().validate_at(&signature.result, path)?;
    for parameter in &signature.parameters {
        declarations.types().validate_at(&parameter.ty, path)?;
    }
    let certified = declarations
        .certified_pure(*declaration)
        .ok_or_else(|| invalid(path, "missing or mismatched pure body certificate"))?;
    let source = certified.source();
    declarations.types().validate_at(&source.result, path)?;
    for parameter in &source.parameters {
        declarations.types().validate_at(&parameter.ty, path)?;
    }
    if source.result != binding.value.ty
        || !surface_type(&source.result)
        || source.parameters.len() != arguments.len()
    {
        return Err(invalid(path, "pure call differs from certified signature"));
    }
    for (argument, parameter) in arguments.iter().zip(&source.parameters) {
        used(argument, scope, declarations, path)?;
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
