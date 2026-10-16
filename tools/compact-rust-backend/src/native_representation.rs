// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Operation-specific tuple traits, separate from ordinary value ownership.
//! This bounded borrowed policy constructs no Rust syntax or replacement types.
//! It corrects proven unbuildable native operations, not general source typing.
use crate::{RenderError, ir::Type, resource_limits::recording_type_nodes};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Equal,
    NotEqual,
    TransientHash,
    TransientCommit,
    PersistentHash,
    PersistentCommit,
    Keccak256,
    HashToCurve,
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Equal => "equality",
            Self::NotEqual => "inequality",
            Self::TransientHash => "transientHash",
            Self::TransientCommit => "transientCommit",
            Self::PersistentHash => "persistentHash",
            Self::PersistentCommit => "persistentCommit",
            Self::Keccak256 => "keccak256",
            Self::HashToCurve => "hashToCurve",
        }
    }

    fn tuple_limit(self) -> usize {
        match self {
            Self::Equal | Self::NotEqual => 12,
            _ => 11,
        }
    }

    fn required_trait(self, contains_opaque: bool) -> &'static str {
        match self {
            Self::Equal | Self::NotEqual => "PartialEq",
            Self::Keccak256 => "Into<Value>",
            Self::TransientHash | Self::TransientCommit | Self::HashToCurve if contains_opaque => {
                // ExactHashInput adapts HashInputValue, whose supertrait is Aligned.
                "HashInputValue"
            }
            _ => "Into<Value> + Aligned",
        }
    }
}

/// Check every nested tuple after the existing borrowed resource preflight.
/// Construction, projection and return of wider tuples have no such obligation.
/// The three dynamic hash operations use ExactHashInput iff any opaque leaf is
/// present; its HashInputValue tuple implementations also stop at arity11.
/// Persistent opaque source policies remain independent and unchanged.
pub(crate) fn check_tuple_traits(operation: Operation, ty: &Type) -> Result<(), RenderError> {
    let nodes = recording_type_nodes(ty)?;
    let limit = operation.tuple_limit();
    let contains_opaque = nodes
        .iter()
        .any(|ty| matches!(ty, Type::OpaqueString | Type::OpaqueBytes));
    for node in nodes {
        if let Type::Tuple { elements } = node
            && elements.len() > limit
        {
            return Err(RenderError::UnsupportedTupleOperation {
                operation: operation.name(),
                arity: elements.len(),
                limit,
                required_trait: operation.required_trait(contains_opaque),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
