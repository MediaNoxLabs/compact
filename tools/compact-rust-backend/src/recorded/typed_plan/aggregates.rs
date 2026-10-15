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

//! Struct and tuple values for the existing typed recording plan.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use super::{Plan, Scope, TypedValue};
use crate::ir::{Expr, Type};
use crate::{ident, retained_value, rust_type};

impl Plan<'_> {
    pub(super) fn aggregate_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::StructLiteral {
                ty: ty @ Type::Struct { fields, .. },
                fields: values,
            } if fields.len() == values.len() => {
                let mut members: Vec<syn::FieldValue> = Vec::new();
                for (field, value) in fields.iter().zip(values) {
                    let value = self.expression(value, scope, steps)?;
                    if value.ty != field.ty {
                        return None;
                    }
                    let name = ident(&field.name).ok()?;
                    let value = value.value;
                    members.push(syn::parse_quote!(#name: #value));
                }
                let rust_ty = rust_type(ty).ok()?;
                self.bind(
                    syn::parse_quote!(#rust_ty { #(#members),* }),
                    ty.clone(),
                    steps,
                )
            }
            Expr::StructField {
                value,
                field,
                index,
            } => {
                let value = self.expression(value, scope, steps)?;
                let Type::Struct { fields, .. } = value.ty else {
                    return None;
                };
                let member = fields.get(*index)?;
                if member.name != *field {
                    return None;
                }
                let name = ident(field).ok()?;
                let source = value.value;
                let projected = retained_value(syn::parse_quote!((#source).#name), &member.ty);
                self.bind(projected, member.ty.clone(), steps)
            }
            Expr::Tuple { elements }
                if self.evaluation.context_query
                    || self.evaluation.scalar_body_depth > 0
                    || self.admission.unit_actions
                    || self.admission.composite_domain.shielded_helpers() =>
            {
                let values = elements
                    .iter()
                    .map(|element| self.expression(element, scope, steps))
                    .collect::<Option<Vec<_>>>()?;
                let types = values.iter().map(|v| v.ty.clone()).collect();
                let values: Vec<_> = values.into_iter().map(|v| v.value).collect();
                self.bind(
                    syn::parse_quote!((#(#values,)*)),
                    Type::Tuple { elements: types },
                    steps,
                )
            }
            _ => None,
        }
    }
}
