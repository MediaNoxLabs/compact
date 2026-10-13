// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//! Compatibility adapters and output-only syntax for checked Jubjub facts.
//! Callers retain lexical scope, source evaluation order and effects. The neutral
//! owner keeps type rules; coordinate quote temporaries stay on a separate path.
use crate::{
    RenderError,
    ir::Type,
    pure_semantics::{
        jubjub::{Axis, CheckedJubjub, JubjubOperands, check_jubjub},
        sink::Typed,
    },
};

type Operand = (syn::Expr, Type);

pub(crate) enum Operation {
    Reduce(Operand),
    Generator(Operand),
    Multiply(Operand, Operand),
    Add(Operand, Operand),
}
fn typed((output, ty): Operand) -> Typed<syn::Expr> {
    Typed { output, ty }
}

impl Operation {
    pub(crate) fn lower(self) -> Result<Operand, RenderError> {
        let operands = match self {
            Self::Reduce(value) => JubjubOperands::Reduce(typed(value)),
            Self::Generator(value) => JubjubOperands::Generator(typed(value)),
            Self::Multiply(point, scalar) => JubjubOperands::Multiply(typed(point), typed(scalar)),
            Self::Add(left, right) => JubjubOperands::Add(typed(left), typed(right)),
        };
        let checked = check_jubjub(operands)?;
        Ok((render(checked.output), checked.ty))
    }
}

/// Coordinate syntax remains separate from arithmetic quote temporaries.
pub(crate) enum Coordinate {
    X,
    Y,
}
impl Coordinate {
    pub(crate) fn lower(self, point: Operand) -> Result<Operand, RenderError> {
        let axis = match self {
            Self::X => Axis::X,
            Self::Y => Axis::Y,
        };
        let checked = check_jubjub(JubjubOperands::Coordinate(axis, typed(point)))?;
        Ok((render(checked.output), checked.ty))
    }
}

/// Formats checked values without Type inspection, cloning or support checks.
pub(crate) fn render(checked: CheckedJubjub<syn::Expr>) -> syn::Expr {
    match checked {
        CheckedJubjub::Reduce(value) => {
            syn::parse_quote!(runtime::jubjub_scalar_from_native(#value))
        }
        CheckedJubjub::Generator(scalar) => syn::parse_quote!(runtime::ec_mul_generator(#scalar)?),
        CheckedJubjub::Multiply(point, scalar) => {
            syn::parse_quote!(runtime::ec_mul(#point, #scalar)?)
        }
        CheckedJubjub::Add(left, right) => syn::parse_quote!(runtime::ec_add(#left, #right)),
        CheckedJubjub::Construct(x, y) => {
            syn::parse_quote!(runtime::construct_jubjub_point(#x, #y)?)
        }
        CheckedJubjub::Negate(value) => syn::parse_quote!(runtime::ec_neg(#value)),
        CheckedJubjub::Coordinate(axis, point) => render_coordinate(axis, point),
    }
}
fn render_coordinate(axis: Axis, point: syn::Expr) -> syn::Expr {
    match axis {
        Axis::X => syn::parse_quote!(runtime::jubjub_point_x(#point)),
        Axis::Y => syn::parse_quote!(runtime::jubjub_point_y(#point)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operand(ty: Type) -> Operand {
        (syn::parse_quote!(value), ty)
    }
    #[test]
    fn jubjub_operations_keep_output_types_and_fallibility() {
        for (op, ty, fallible) in [
            (Operation::Reduce(operand(Type::Field)), Type::Field, false),
            (
                Operation::Generator(operand(Type::Field)),
                Type::JubjubPoint,
                true,
            ),
            (
                Operation::Multiply(operand(Type::JubjubPoint), operand(Type::Field)),
                Type::JubjubPoint,
                true,
            ),
            (
                Operation::Add(operand(Type::JubjubPoint), operand(Type::JubjubPoint)),
                Type::JubjubPoint,
                false,
            ),
        ] {
            let (expr, actual) = op.lower().unwrap();
            assert_eq!(actual, ty);
            assert_eq!(matches!(expr, syn::Expr::Try(_)), fallible);
        }
    }
    #[test]
    fn jubjub_operations_reject_every_wrong_operand_position() {
        for op in [
            Operation::Reduce(operand(Type::JubjubPoint)),
            Operation::Generator(operand(Type::JubjubPoint)),
            Operation::Multiply(operand(Type::Field), operand(Type::Field)),
            Operation::Multiply(operand(Type::JubjubPoint), operand(Type::JubjubPoint)),
            Operation::Add(operand(Type::Field), operand(Type::JubjubPoint)),
            Operation::Add(operand(Type::JubjubPoint), operand(Type::Field)),
        ] {
            assert!(matches!(op.lower(), Err(RenderError::TypeMismatch { .. })));
        }
    }
    #[test]
    fn coordinates_require_points_and_produce_infallible_fields() {
        for coordinate in [Coordinate::X, Coordinate::Y] {
            let (expression, actual) = coordinate.lower(operand(Type::JubjubPoint)).unwrap();
            assert_eq!(actual, Type::Field);
            assert!(!matches!(expression, syn::Expr::Try(_)));
        }
        for coordinate in [Coordinate::X, Coordinate::Y] {
            assert!(matches!(
                coordinate.lower(operand(Type::Field)),
                Err(RenderError::TypeMismatch {
                    expected: Type::JubjubPoint,
                    actual: Type::Field,
                })
            ));
        }
    }
}
