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

//! Numeric and comparison expressions under existing recording admission policies.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use super::{CompositeDomain, Plan, Scope, TypedValue, funded_mint, phase_reset, shielded_merge};
use crate::coerce_expression;
use crate::ir::{Expr, Type};

impl Plan<'_> {
    pub(super) fn arithmetic_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::UnsignedCast { value, max }
                if self.composite_domain.values() || self.phase_reset =>
            {
                let value = self.expression(value, scope, steps)?;
                let Type::Unsigned { max: source_max } = value.ty else {
                    return None;
                };
                if self.phase_reset
                    && !matches!(
                        (source_max.as_str(), max.as_str()),
                        ("18446744073709551615", phase_reset::WIDENED)
                            | (phase_reset::SUM, "18446744073709551615")
                    )
                {
                    return None;
                }
                if self.composite_domain.shielded_merge()
                    && !matches!(
                        (source_max.as_str(), max.as_str()),
                        (shielded_merge::INPUT, shielded_merge::WIDENED)
                            | (shielded_merge::SUM, shielded_merge::INPUT)
                    )
                    && !(matches!(
                        self.composite_domain,
                        CompositeDomain::GuardedShieldedDeposit
                            | CompositeDomain::FundedShieldedMint
                    ) && source_max == u64::MAX.to_string()
                        && max == shielded_merge::INPUT)
                    && !(self.composite_domain == CompositeDomain::FundedShieldedMint
                        && ((source_max == u64::MAX.to_string() && max == funded_mint::PRODUCT)
                            || (source_max == shielded_merge::INPUT
                                && max == shielded_merge::INPUT)))
                {
                    return None;
                }
                let converted = crate::unsigned_cast_syntax(value.value, &source_max, max).ok()?;
                self.bind(converted, Type::Unsigned { max: max.clone() }, steps)
            }
            Expr::UnsignedMultiply { max, left, right }
                if self.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.funded_multiply(expression, left, right, max, scope, steps)
            }
            Expr::NotEqual { left, right }
                if self.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.funded_not_equal(left, right, scope, steps)
            }
            Expr::NotEqual { left, right } if self.composition_calls.is_some() => {
                self.composition_scalar_not_equal(left, right, scope, steps)
            }
            Expr::JubjubPointX { value } | Expr::JubjubPointY { value }
                if self.composition_calls.is_some() =>
            {
                self.composition_point_coordinate(expression, value, scope, steps)
            }
            Expr::UnsignedAdd { max, left, right }
                if self.phase_reset || self.composite_domain.shielded_merge() =>
            {
                let left = self.expression(left, scope, steps)?;
                let right = self.expression(right, scope, steps)?;
                let Type::Unsigned { max: left_max } = left.ty else {
                    return None;
                };
                let Type::Unsigned { max: right_max } = right.ty else {
                    return None;
                };
                let (operand_bound, result_bound) = if self.phase_reset {
                    (phase_reset::WIDENED, phase_reset::SUM)
                } else {
                    (shielded_merge::WIDENED, shielded_merge::SUM)
                };
                if left_max != operand_bound || right_max != operand_bound || max != result_bound {
                    return None;
                }
                let value = crate::unsigned_arithmetic_syntax(
                    expression,
                    left.value,
                    right.value,
                    &left_max,
                    &right_max,
                    max,
                )
                .ok()?;
                self.bind(value, Type::Unsigned { max: max.clone() }, steps)
            }
            Expr::UnsignedSubtract { max, left, right }
                if self.composite_domain.shielded_send() =>
            {
                let left = self.expression(left, scope, steps)?;
                let right = self.expression(right, scope, steps)?;
                let (Type::Unsigned { max: left_max }, Type::Unsigned { max: right_max }) =
                    (&left.ty, &right.ty)
                else {
                    return None;
                };
                if left_max != max || right_max != max || max != &u128::MAX.to_string() {
                    return None;
                }
                let value = crate::unsigned_arithmetic_syntax(
                    expression,
                    left.value,
                    right.value,
                    left_max,
                    right_max,
                    max,
                )
                .ok()?;
                self.bind(value, Type::Unsigned { max: max.clone() }, steps)
            }
            Expr::Coerce { value, ty } => {
                let value = self.expression(value, scope, steps)?;
                let converted = coerce_expression(value.value, &value.ty, ty, 0).ok()?;
                self.bind(converted, ty.clone(), steps)
            }
            Expr::Equal { left, right } => {
                let left = self.expression(left, scope, steps)?;
                let right = self.expression(right, scope, steps)?;
                if left.ty != right.ty {
                    return None;
                }
                let (left, right) = (left.value, right.value);
                self.bind(syn::parse_quote!(#left == #right), Type::Boolean, steps)
            }
            Expr::Add { left, right }
                if self.field_cell_slot.is_some() || self.effectful_field_cells =>
            {
                let left = self.expression(left, scope, steps)?;
                let right = self.expression(right, scope, steps)?;
                if left.ty != Type::Field || right.ty != Type::Field {
                    return None;
                }
                let (left, right) = (left.value, right.value);
                self.bind(syn::parse_quote!(#left + #right), Type::Field, steps)
            }
            Expr::FieldCast { value } => {
                let value = self.expression(value, scope, steps)?;
                let Type::Unsigned { max } = &value.ty else {
                    return None;
                };
                if if self.effectful_field_cells {
                    !matches!(
                        crate::unsigned_maximum(max).ok()?,
                        crate::UnsignedMaximum::Small(_)
                    )
                } else {
                    max != "18446744073709551615"
                } {
                    return None;
                }
                let value = value.value;
                self.bind(
                    syn::parse_quote!(runtime::Field::from((#value).value())),
                    Type::Field,
                    steps,
                )
            }
            Expr::FieldToBytes32 { value } => {
                let value = self.expression(value, scope, steps)?;
                if value.ty != Type::Field {
                    return None;
                }
                self.bind(
                    crate::field_to_bytes_32_syntax(value.value),
                    Type::Bytes { length: 32 },
                    steps,
                )
            }
            _ => None,
        }
    }
}
