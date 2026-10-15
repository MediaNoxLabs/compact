// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact structural equality on borrowed Types with charged iterative work.
//! This compares declarations, rather than checking whether they are valid.

use super::Work;
use crate::{
    RenderError,
    ir::{StructField, Type},
};

/// Continuations keep at most one sibling pending per nesting level. They never
/// expand vector lengths or copy the members of a named declaration.
enum Comparison<'a> {
    Type(&'a Type, &'a Type),
    Tuple(&'a [Type], &'a [Type]),
    Fields(&'a [StructField], &'a [StructField]),
    Variants(&'a [String], &'a [String]),
    Length(usize, usize),
}

pub(super) fn equivalent(
    actual: &Type,
    target: &Type,
    work: &mut Work,
) -> Result<bool, RenderError> {
    let mut pending = Vec::new();
    push(work, &mut pending, Comparison::Type(actual, target))?;
    while let Some(next) = pending.pop() {
        match next {
            Comparison::Length(left, right) => {
                if left != right {
                    return Ok(false);
                }
            }
            Comparison::Type(left, right) => match (left, right) {
                (Type::Unit, Type::Unit)
                | (Type::Boolean, Type::Boolean)
                | (Type::Field, Type::Field)
                | (Type::JubjubPoint, Type::JubjubPoint)
                | (Type::OpaqueString, Type::OpaqueString)
                | (Type::OpaqueBytes, Type::OpaqueBytes) => {}
                (Type::Bytes { length: left }, Type::Bytes { length: right }) => {
                    if left != right {
                        return Ok(false);
                    }
                }
                (Type::Unsigned { max: left }, Type::Unsigned { max: right }) => {
                    if !work.text_equal(left, right)? {
                        return Ok(false);
                    }
                }
                (
                    Type::Struct {
                        name: left_name,
                        fields: left,
                    },
                    Type::Struct {
                        name: right_name,
                        fields: right,
                    },
                ) => {
                    if !work.text_equal(left_name, right_name)? || left.len() != right.len() {
                        return Ok(false);
                    }
                    push(work, &mut pending, Comparison::Fields(left, right))?;
                }
                (
                    Type::Enum {
                        name: left_name,
                        variants: left,
                    },
                    Type::Enum {
                        name: right_name,
                        variants: right,
                    },
                ) => {
                    if !work.text_equal(left_name, right_name)? || left.len() != right.len() {
                        return Ok(false);
                    }
                    push(work, &mut pending, Comparison::Variants(left, right))?;
                }
                (Type::Tuple { elements: left }, Type::Tuple { elements: right }) => {
                    if left.len() != right.len() {
                        return Ok(false);
                    }
                    push(work, &mut pending, Comparison::Tuple(left, right))?;
                }
                (
                    Type::Vector {
                        element: left,
                        length: left_length,
                    },
                    Type::Vector {
                        element: right,
                        length: right_length,
                    },
                ) => {
                    push(
                        work,
                        &mut pending,
                        Comparison::Length(*left_length, *right_length),
                    )?;
                    push(work, &mut pending, Comparison::Type(left, right))?;
                }
                (
                    Type::LedgerMap {
                        key: left_key,
                        value: left_value,
                    },
                    Type::LedgerMap {
                        key: right_key,
                        value: right_value,
                    },
                ) => {
                    push(
                        work,
                        &mut pending,
                        Comparison::Type(left_value, right_value),
                    )?;
                    push(work, &mut pending, Comparison::Type(left_key, right_key))?;
                }
                // Exhaustiveness belongs to the exhaustive Type match below;
                // unequal variant pairs are structurally different.
                (left, _) => match left {
                    Type::Unit
                    | Type::Boolean
                    | Type::Field
                    | Type::JubjubPoint
                    | Type::OpaqueString
                    | Type::OpaqueBytes
                    | Type::Bytes { .. }
                    | Type::Struct { .. }
                    | Type::Enum { .. }
                    | Type::Unsigned { .. }
                    | Type::Tuple { .. }
                    | Type::Vector { .. }
                    | Type::LedgerMap { .. } => return Ok(false),
                },
            },
            Comparison::Tuple(left, right) => {
                if let Some((left_first, left_rest)) = left.split_first() {
                    let (right_first, right_rest) =
                        right.split_first().expect("equal member counts");
                    push(work, &mut pending, Comparison::Tuple(left_rest, right_rest))?;
                    push(
                        work,
                        &mut pending,
                        Comparison::Type(left_first, right_first),
                    )?;
                }
            }
            Comparison::Fields(left, right) => {
                if let Some((left_first, left_rest)) = left.split_first() {
                    let (right_first, right_rest) =
                        right.split_first().expect("equal field counts");
                    if !work.text_equal(&left_first.name, &right_first.name)? {
                        return Ok(false);
                    }
                    push(
                        work,
                        &mut pending,
                        Comparison::Fields(left_rest, right_rest),
                    )?;
                    push(
                        work,
                        &mut pending,
                        Comparison::Type(&left_first.ty, &right_first.ty),
                    )?;
                }
            }
            Comparison::Variants(left, right) => {
                if let Some((left_first, left_rest)) = left.split_first() {
                    let (right_first, right_rest) =
                        right.split_first().expect("equal variant counts");
                    if !work.text_equal(left_first, right_first)? {
                        return Ok(false);
                    }
                    push(
                        work,
                        &mut pending,
                        Comparison::Variants(left_rest, right_rest),
                    )?;
                }
            }
        }
    }
    Ok(true)
}

fn push<'a>(
    work: &mut Work,
    pending: &mut Vec<Comparison<'a>>,
    next: Comparison<'a>,
) -> Result<(), RenderError> {
    let observed = pending.len().saturating_add(1);
    if observed > work.pending_limit {
        return Err(RenderError::ResourceLimit {
            resource: "coercion_pending",
            observed,
            limit: work.pending_limit,
        });
    }
    // Charge before a possible pending Vec allocation and comparison.
    work.charge(1)?;
    pending.push(next);
    Ok(())
}
