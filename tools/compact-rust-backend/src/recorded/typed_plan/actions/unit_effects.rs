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

//! Assertions, discarded Unit expressions, and admitted Unit call effects.
//!
//! The parent retains the distinct witness-expression and intent-expression guards.
//! Voting calls preserve their original allowance for a discarded shielded result.

use crate::ir::{Expr, Type};
use crate::recorded::typed_plan::{
    CompositeDomain, Plan, Scope, composition, shielded_send_result,
};

impl Plan<'_> {
    pub(in crate::recorded::typed_plan) fn lower_assertion(
        &mut self,
        condition: &Expr,
        message: &str,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let condition = self.expression(condition, scope, steps)?;
        if condition.ty != Type::Boolean {
            return None;
        }
        let condition = condition.value;
        steps.push(syn::parse_quote! {
            if !#condition { return Err(runtime::CompactError::AssertionFailed(#message.to_owned())); }
        });
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_unit_expression(
        &mut self,
        value: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        if self.expression(value, scope, steps)?.ty != Type::Unit {
            return None;
        }
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_pure_guard_call(
        &mut self,
        name: &str,
        arguments: &[Expr],
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        if !matches!(
            self.admission.composition_calls.as_ref()?.get(name),
            Some(composition::AuditedCall::PureUnitGuard(_))
        ) {
            return None;
        }
        let value = self.call(name, arguments, scope, steps)?;
        if value.ty != Type::Unit {
            return None;
        }
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_unit_circuit_call(
        &mut self,
        name: &str,
        arguments: &[Expr],
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let result = self.call(name, arguments, scope, steps)?;
        if result.ty != Type::Unit
            && !(self.admission.composite_domain == CompositeDomain::VotingCommit
                && shielded_send_result(&result.ty))
        {
            return None;
        }
        Some(())
    }
}
