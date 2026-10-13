// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Owned Jubjub expectations shared by pure semantics and legacy adapters.
//! Callers finish both source visits before these checks. No clones or new
//! preflights are introduced here; output facts carry no unchecked source Type.
use super::sink::Typed;
use crate::{RenderError, ir::Type};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    X,
    Y,
}

pub(crate) enum JubjubOperands<V> {
    Reduce(Typed<V>),
    Generator(Typed<V>),
    Multiply(Typed<V>, Typed<V>),
    Add(Typed<V>, Typed<V>),
    Coordinate(Axis, Typed<V>),
    Construct(Typed<V>, Typed<V>),
    Negate(Typed<V>),
}

pub(crate) enum CheckedJubjub<V> {
    Reduce(V),
    Generator(V),
    Multiply(V, V),
    Add(V, V),
    Coordinate(Axis, V),
    Construct(V, V),
    Negate(V),
}

fn expect<V>(value: Typed<V>, expected: Type) -> Result<V, RenderError> {
    if value.ty != expected {
        return Err(RenderError::TypeMismatch {
            expected,
            actual: value.ty,
        });
    }
    Ok(value.output)
}

pub(crate) fn check_jubjub<V>(
    input: JubjubOperands<V>,
) -> Result<Typed<CheckedJubjub<V>>, RenderError> {
    let (output, ty) = match input {
        JubjubOperands::Reduce(value) => (
            CheckedJubjub::Reduce(expect(value, Type::Field)?),
            Type::Field,
        ),
        JubjubOperands::Generator(value) => (
            CheckedJubjub::Generator(expect(value, Type::Field)?),
            Type::JubjubPoint,
        ),
        JubjubOperands::Multiply(point, scalar) => {
            let point = expect(point, Type::JubjubPoint)?;
            let scalar = expect(scalar, Type::Field)?;
            (CheckedJubjub::Multiply(point, scalar), Type::JubjubPoint)
        }
        JubjubOperands::Add(left, right) => {
            let left = expect(left, Type::JubjubPoint)?;
            let right = expect(right, Type::JubjubPoint)?;
            (CheckedJubjub::Add(left, right), Type::JubjubPoint)
        }
        JubjubOperands::Coordinate(axis, point) => (
            CheckedJubjub::Coordinate(axis, expect(point, Type::JubjubPoint)?),
            Type::Field,
        ),
        JubjubOperands::Construct(x, y) => {
            let x = expect(x, Type::Field)?;
            let y = expect(y, Type::Field)?;
            (CheckedJubjub::Construct(x, y), Type::JubjubPoint)
        }
        JubjubOperands::Negate(point) => (
            CheckedJubjub::Negate(expect(point, Type::JubjubPoint)?),
            Type::JubjubPoint,
        ),
    };
    Ok(Typed { output, ty })
}
