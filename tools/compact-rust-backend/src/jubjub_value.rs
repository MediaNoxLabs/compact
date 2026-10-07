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
//! Syntax and type rules for already evaluated Jubjub operands.
//! Callers retain ownership of lexical scope, evaluation order and effects.
use crate::{RenderError, ir::Type};

type Operand = (syn::Expr, Type);

pub(crate) enum Operation {
    Reduce(Operand),
    Generator(Operand),
    Multiply(Operand, Operand),
    Add(Operand, Operand),
}

fn expect((value, actual): Operand, expected: Type) -> Result<syn::Expr, RenderError> {
    if actual != expected {
        return Err(RenderError::TypeMismatch { expected, actual });
    }
    Ok(value)
}

impl Operation {
    pub(crate) fn lower(self) -> Result<Operand, RenderError> {
        match self {
            Self::Reduce(value) => {
                let value = expect(value, Type::Field)?;
                Ok((
                    syn::parse_quote!(runtime::jubjub_scalar_from_native(#value)),
                    Type::Field,
                ))
            }
            Self::Generator(scalar) => {
                let scalar = expect(scalar, Type::Field)?;
                Ok((
                    syn::parse_quote!(runtime::ec_mul_generator(#scalar)?),
                    Type::JubjubPoint,
                ))
            }
            Self::Multiply(point, scalar) => {
                let point = expect(point, Type::JubjubPoint)?;
                let scalar = expect(scalar, Type::Field)?;
                Ok((
                    syn::parse_quote!(runtime::ec_mul(#point, #scalar)?),
                    Type::JubjubPoint,
                ))
            }
            Self::Add(left, right) => {
                let left = expect(left, Type::JubjubPoint)?;
                let right = expect(right, Type::JubjubPoint)?;
                Ok((
                    syn::parse_quote!(runtime::ec_add(#left, #right)),
                    Type::JubjubPoint,
                ))
            }
        }
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
}
