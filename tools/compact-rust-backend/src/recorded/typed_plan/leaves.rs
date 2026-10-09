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

//! Literal and scoped leaf values for the existing typed recording plan.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use std::collections::HashMap;

use super::{Plan, Scope, TypedValue};
use crate::ir::{Expr, Type};
use crate::{expression_with_calls, retained_value};

impl Plan<'_> {
    pub(super) fn leaf_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::Unit
                if self.read_only_assertions
                    || self.unit_actions
                    || self.phase_reset
                    || self.composite_domain.shielded_helpers()
                    || self.composition_calls.is_some() =>
            {
                Some(TypedValue {
                    ty: Type::Unit,
                    value: syn::parse_quote!(()),
                })
            }
            Expr::Parameter { name } => {
                let value = scope.get(name)?;
                Some(TypedValue {
                    ty: value.ty.clone(),
                    value: retained_value(value.value.clone(), &value.ty),
                })
            }
            Expr::Boolean { .. }
            | Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. }
            | Expr::FieldLiteral { .. }
            | Expr::UnsignedLiteral { .. } => {
                let (value, ty) =
                    expression_with_calls(expression, &HashMap::new(), &HashMap::new()).ok()?;
                self.bind(value, ty, steps)
            }
            Expr::Default {
                ty: Type::Struct { .. },
            } => {
                let (value, ty) =
                    expression_with_calls(expression, &HashMap::new(), &HashMap::new()).ok()?;
                self.bind(value, ty, steps)
            }
            _ => None,
        }
    }
}
