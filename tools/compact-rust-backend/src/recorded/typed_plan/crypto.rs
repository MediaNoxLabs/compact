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

//! Hash and commitment expressions under existing typed recording policies.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use super::{Plan, Scope, TypedValue, unit_actions};
use crate::ir::{Expr, Type};

impl Plan<'_> {
    pub(super) fn crypto_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::PersistentCommit { value, opening } if self.context_query => {
                let value = self.expression(value, scope, steps)?;
                let opening = self.expression(opening, scope, steps)?;
                if opening.ty != (Type::Bytes { length: 32 })
                    || value.ty
                        != (Type::Tuple {
                            elements: vec![Type::Bytes { length: 32 }; 2],
                        })
                {
                    return None;
                }
                let (value, opening) = (value.value, opening.value);
                self.bind(
                    syn::parse_quote!(runtime::persistent_commit(#value, #opening)),
                    Type::Bytes { length: 32 },
                    steps,
                )
            }
            Expr::PersistentHash { value } if self.scalar_body_depth > 0 || self.unit_actions => {
                let value = self.expression(value, scope, steps)?;
                if value.ty
                    != (Type::Tuple {
                        elements: vec![
                            Type::Bytes { length: 32 };
                            if self.scalar_body_depth > 0 { 3 } else { 2 }
                        ],
                    })
                {
                    return None;
                }
                let value = value.value;
                self.bind(
                    syn::parse_quote!(runtime::persistent_hash(#value)),
                    Type::Bytes { length: 32 },
                    steps,
                )
            }
            Expr::TransientCommit { value, opening } if self.unit_actions => {
                let value = self.expression(value, scope, steps)?;
                let opening = self.expression(opening, scope, steps)?;
                if !unit_actions::field_structure(&value.ty) || opening.ty != Type::Field {
                    return None;
                }
                let (value, opening) = (value.value, opening.value);
                self.bind(
                    syn::parse_quote!(runtime::transient_commit(#value, #opening)),
                    Type::Field,
                    steps,
                )
            }
            Expr::DegradeToTransient { value } if self.composite_domain.shielded_helpers() => {
                let value = self.expression(value, scope, steps)?;
                if value.ty != (Type::Bytes { length: 32 }) {
                    return None;
                }
                let value = value.value;
                self.bind(
                    syn::parse_quote!(runtime::degrade_to_transient(#value)),
                    Type::Field,
                    steps,
                )
            }
            Expr::TransientHash { value } if self.composite_domain.shielded_helpers() => {
                let value = self.expression(value, scope, steps)?;
                if value.ty
                    != (Type::Tuple {
                        elements: vec![Type::Field, Type::Field],
                    })
                {
                    return None;
                }
                let value = value.value;
                self.bind(
                    syn::parse_quote!(runtime::transient_hash(#value)),
                    Type::Field,
                    steps,
                )
            }
            Expr::UpgradeFromTransient { value } if self.composite_domain.shielded_helpers() => {
                let value = self.expression(value, scope, steps)?;
                if value.ty != Type::Field {
                    return None;
                }
                let value = value.value;
                self.bind(
                    syn::parse_quote!(runtime::upgrade_from_transient(#value)),
                    Type::Bytes { length: 32 },
                    steps,
                )
            }
            _ => None,
        }
    }
}
