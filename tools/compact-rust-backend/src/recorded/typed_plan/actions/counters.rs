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

//! Counter reset and increment effects after parent profile admission.

use crate::ident;
use crate::ir::{CounterAmount, Expr, LedgerFieldKind, Type};
use crate::recorded::typed_plan::{Plan, Scope};
use proc_macro2::Span;

impl Plan<'_> {
    pub(in crate::recorded::typed_plan) fn lower_counter_reset(
        &mut self,
        field: &str,
        index: u8,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        if self.field(field, index)?.declaration != LedgerFieldKind::Counter {
            return None;
        }
        let slot = ident(field).ok()?;
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;));
        self.evidence.counter_writes += 1;
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_counter_increment(
        &mut self,
        field: &str,
        index: u8,
        amount: &CounterAmount,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        if self.field(field, index)?.declaration != LedgerFieldKind::Counter {
            return None;
        }
        let amount: syn::Expr = match amount {
            CounterAmount::Literal { value } => {
                let value = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                syn::parse_quote!(#value)
            }
            CounterAmount::Parameter { name } => {
                let value =
                    self.expression(&Expr::Parameter { name: name.clone() }, scope, steps)?;
                if value.ty
                    != (Type::Unsigned {
                        max: "65535".into(),
                    })
                {
                    return None;
                }
                let value = value.value;
                syn::parse_quote!((#value).value() as u16)
            }
        };
        let slot = ident(field).ok()?;
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_increment(frame, #amount)?;));
        self.evidence.counter_writes += 1;
        Some(())
    }
}
