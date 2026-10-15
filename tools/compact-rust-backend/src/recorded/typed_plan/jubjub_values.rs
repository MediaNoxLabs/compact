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
//! Shared typed Jubjub expression adapter for the legacy evaluator.
//! The former shape admission is retained exclusively as a test oracle.
use super::*;

#[cfg(test)]
use super::jubjub_reference::lower;

/// Each child is evaluated once, left to right, before shared syntax lowering.
pub(super) fn expression(
    plan: &mut Plan<'_>,
    expr: &Expr,
    scope: &Scope,
    steps: &mut Vec<syn::Stmt>,
) -> Option<TypedValue> {
    if plan.admission.composite_domain != CompositeDomain::JubjubValues {
        return None;
    }
    let mut operand = |v: &Expr| {
        let v = plan.expression(v, scope, steps)?;
        Some((v.value, v.ty))
    };
    use crate::jubjub_value::Operation;
    let operation = match expr {
        Expr::JubjubScalarFromNative { value } => Operation::Reduce(operand(value)?),
        Expr::EcMulGenerator { scalar } => Operation::Generator(operand(scalar)?),
        Expr::EcMul { point, scalar } => Operation::Multiply(operand(point)?, operand(scalar)?),
        Expr::EcAdd { left, right } => Operation::Add(operand(left)?, operand(right)?),
        _ => return None,
    };
    let (value, ty) = operation.lower().ok()?;
    plan.bind(value, ty, steps)
}

#[cfg(test)]
mod tests;
