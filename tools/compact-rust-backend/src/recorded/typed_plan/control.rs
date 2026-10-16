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

//! Control flow and branch-local values for the existing typed recording plan.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use super::{CompositeDomain, Plan, Scope, TypedValue};
use crate::ir::{Expr, Type};
use crate::rust_type;

impl Plan<'_> {
    pub(super) fn control_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::Sequence {
                steps: expressions,
                value,
            } if self.admission.read_only_assertions
                || self.admission.unit_actions
                || self.admission.composite_domain.shielded_helpers() =>
            {
                for expression in expressions {
                    if self.expression(expression, scope, steps)?.ty != Type::Unit {
                        return None;
                    }
                }
                self.expression(value, scope, steps)
            }
            Expr::Assert { condition, message }
                if self.admission.read_only_assertions
                    || self.admission.unit_actions
                    || matches!(
                        self.admission.composite_domain,
                        CompositeDomain::ShieldedPayout
                            | CompositeDomain::ActionfulShieldedPayout
                            | CompositeDomain::ResetShieldedPayout
                    )
                    || self.admission.composite_domain.shielded_merge() =>
            {
                let condition = self.expression(condition, scope, steps)?;
                if condition.ty != Type::Boolean {
                    return None;
                }
                let condition = condition.value;
                steps.push(syn::parse_quote! {
                    if !#condition { return Err(runtime::CompactError::AssertionFailed(#message.to_owned())); }
                });
                Some(TypedValue {
                    ty: Type::Unit,
                    value: syn::parse_quote!(()),
                })
            }
            Expr::Let { bindings, body } => {
                let scoped = self.bindings(bindings, scope, steps)?;
                self.expression(body, &scoped, steps)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = self.expression(condition, scope, steps)?;
                if condition.ty != Type::Boolean {
                    return None;
                }
                let mut then_steps = Vec::new();
                let then = self.expression(then, scope, &mut then_steps)?;
                let mut else_steps = Vec::new();
                let otherwise = self.expression(otherwise, scope, &mut else_steps)?;
                if then.ty != otherwise.ty
                    || (!self.admission.composite_domain.values()
                        && !(self.admission.unit_actions && then.ty == Type::Unit)
                        && then.ty != Type::Boolean
                        && !(self.evaluation.scalar_arguments
                            && then.ty == (Type::Bytes { length: 32 })))
                {
                    return None;
                }
                let result_ty = then.ty.clone();
                let rust_ty = rust_type(&result_ty).ok()?;
                let name = self.fresh();
                let (condition, then, otherwise) = (condition.value, then.value, otherwise.value);
                steps.push(syn::parse_quote! {
                    let (frame, #name): (_, #rust_ty) = if #condition {
                        #(#then_steps)*
                        (frame, #then)
                    } else {
                        #(#else_steps)*
                        (frame, #otherwise)
                    };
                });
                Some(TypedValue {
                    ty: result_ty,
                    value: syn::parse_quote!(#name),
                })
            }
            _ => None,
        }
    }
}
