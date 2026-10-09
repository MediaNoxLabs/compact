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

//! Typed recording plans with bounded membership and optional-Cell admission.
//! Actual value types and branch-local frames preserve scope and effect order.

use super::*;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
use crate::ir::{KernelClaimKind, ReturnPlan};
mod actions;
mod admission;
mod aggregates;
mod arithmetic;
mod composition;
mod control;
mod crypto;
mod declarations;
#[cfg(test)]
mod effectful_reference;
mod effects;
#[cfg(test)]
pub(super) use effectful_reference::lower_effectful;
mod evaluation;
mod evidence;
mod leaves;
mod observations;
use admission::AdmissionPolicy;
pub(super) use composition::CompositionRejection;
use declarations::Declarations;
use evaluation::EvaluationState;
use evidence::EffectEvidence;
mod field_observations;
mod funded_mint;
mod guarded_deposit;
mod immediate_send;
mod jubjub_values;
mod phase_reset;
mod reset_payout;
mod shielded_merge;
mod shielded_payout;
mod start_funding;
mod terminal_returns;
mod unit_actions;
mod voting_commit;

#[derive(Clone)]
struct TypedValue {
    ty: Type,
    value: syn::Expr,
}

type Scope = HashMap<String, TypedValue>;

// Admission owns the value/effect domain. ShieldedReceive intentionally has
// intents without composite-value helper routing; phase_reset is independent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompositeDomain {
    None,
    Values,
    Intents,
    ShieldedReceive,
    ShieldedSend,
    ImmediateShieldedSend,
    ShieldedPayout,
    ActionfulShieldedPayout,
    ResetShieldedPayout,
    ShieldedMerge(shielded_merge::Inputs),
    GuardedShieldedDeposit,
    FundedShieldedMint,
    VotingCommit,
    StartFunding,
    FieldObservations,
    TerminalReturns,
    JubjubValues,
    UnitComposition,
}
impl CompositeDomain {
    fn shielded_send(self) -> bool {
        matches!(
            self,
            Self::ShieldedSend
                | Self::VotingCommit
                | Self::ImmediateShieldedSend
                | Self::ShieldedPayout
                | Self::ActionfulShieldedPayout
                | Self::ResetShieldedPayout
        )
    }

    fn shielded_helpers(self) -> bool {
        self.shielded_send() || self.shielded_merge()
    }
    fn shielded_merge(self) -> bool {
        matches!(
            self,
            Self::ShieldedMerge(_)
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
                | Self::StartFunding
        )
    }

    fn singleton_bridge(self) -> bool {
        matches!(
            self,
            Self::ImmediateShieldedSend
                | Self::VotingCommit
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
                | Self::StartFunding
                | Self::ShieldedMerge(shielded_merge::Inputs::ReceivedRight)
        )
    }

    fn values(self) -> bool {
        matches!(
            self,
            Self::Values
                | Self::Intents
                | Self::FieldObservations
                | Self::ShieldedSend
                | Self::VotingCommit
                | Self::ImmediateShieldedSend
                | Self::ShieldedPayout
                | Self::ActionfulShieldedPayout
                | Self::ResetShieldedPayout
                | Self::ShieldedMerge(_)
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
                | Self::StartFunding
        )
    }
    fn intents(self) -> bool {
        matches!(
            self,
            Self::Intents
                | Self::ShieldedReceive
                | Self::ShieldedSend
                | Self::VotingCommit
                | Self::ImmediateShieldedSend
                | Self::ShieldedPayout
                | Self::ActionfulShieldedPayout
                | Self::ResetShieldedPayout
                | Self::ShieldedMerge(_)
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
                | Self::StartFunding
        )
    }
}

struct Plan<'a> {
    declarations: Declarations<'a>,
    evidence: EffectEvidence,
    admission: AdmissionPolicy<'a>,
    evaluation: EvaluationState,
}

impl Plan<'_> {
    fn fresh(&mut self) -> syn::Ident {
        let id = syn::Ident::new(
            &format!("__compact_plan_{}", self.evaluation.next),
            Span::call_site(),
        );
        self.evaluation.next += 1;
        id
    }

    fn bind(
        &mut self,
        value: syn::Expr,
        ty: Type,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let rust_ty = rust_type(&ty).ok()?;
        let name = self.fresh();
        steps.push(syn::parse_quote!(let #name: #rust_ty = #value;));
        Some(TypedValue {
            ty,
            value: syn::parse_quote!(#name),
        })
    }

    fn pure_call(&self, name: &str, visiting: &mut HashSet<String>) -> bool {
        let Some(callee) = self.declarations.pure.get(name) else {
            return false;
        };
        if !visiting.insert(name.to_owned()) {
            return false;
        }
        let parameters: HashMap<_, _> = callee
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                (
                    parameter.name.as_str(),
                    (
                        &parameter.ty,
                        syn::Ident::new(
                            &format!("__compact_pure_audit_{index}"),
                            Span::call_site(),
                        ),
                    ),
                )
            })
            .collect();
        let supported = parameters.len() == callee.parameters.len()
            && self.pure_value(&callee.body, visiting)
            && matches!(
                expression_with_calls(&callee.body, &parameters, self.declarations.pure),
                Ok((_, ty)) if ty == callee.result
            );
        visiting.remove(name);
        supported
    }

    fn pure_value(&self, value: &Expr, visiting: &mut HashSet<String>) -> bool {
        match value {
            Expr::UnsignedLiteral { value, max }
                if self.admission.composite_domain.singleton_bridge()
                    && value == "0"
                    && max == &u64::MAX.to_string() =>
            {
                true
            }
            Expr::FieldCast { value } | Expr::UpgradeFromTransient { value }
                if self.admission.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.pure_value(value, visiting)
            }
            Expr::PersistentCommit { value, opening }
                if self.admission.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.pure_value(value, visiting) && self.pure_value(opening, visiting)
            }
            Expr::Default {
                ty: Type::OpaqueString,
            } => true,
            Expr::Default {
                ty: Type::Struct { .. },
            } => true,
            Expr::Parameter { .. }
            | Expr::BytesLiteral { .. }
            | Expr::FieldLiteral { .. }
            | Expr::Boolean { .. }
            | Expr::EnumVariant { .. } => true,
            Expr::StructField { value, .. }
            | Expr::Coerce { value, .. }
            | Expr::PersistentHash { value }
            | Expr::TransientHash { value }
            | Expr::DegradeToTransient { value } => self.pure_value(value, visiting),
            Expr::Tuple { elements }
            | Expr::StructLiteral {
                fields: elements, ..
            } => elements
                .iter()
                .all(|value| self.pure_value(value, visiting)),
            Expr::Equal { left, right } => {
                self.pure_value(left, visiting) && self.pure_value(right, visiting)
            }
            Expr::TransientCommit { value, opening } => {
                self.pure_value(value, visiting) && self.pure_value(opening, visiting)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                self.pure_value(condition, visiting)
                    && self.pure_value(then, visiting)
                    && self.pure_value(otherwise, visiting)
            }
            Expr::Let { bindings, body } => {
                bindings
                    .iter()
                    .all(|binding| self.pure_value(&binding.value, visiting))
                    && self.pure_value(body, visiting)
            }
            Expr::Call { name, arguments } => {
                arguments
                    .iter()
                    .all(|value| self.pure_value(value, visiting))
                    && self.pure_call(name, visiting)
            }
            Expr::VectorFoldCall {
                name,
                initial,
                source,
                ..
            } => {
                self.pure_value(initial, visiting)
                    && self.pure_value(source, visiting)
                    && self.pure_call(name, visiting)
            }
            _ => false,
        }
    }

    fn field(&self, field: &str, index: u8) -> Option<&LedgerField> {
        let declaration = *self.declarations.ledger.get(field)?;
        (declaration.index == index
            && (declaration.physical_path().len() == 1
                || (self.admission.composition_calls.is_some()
                    && composition::slot_path(declaration))))
        .then_some(declaration)
    }

    fn qualified_cell_field(&self, field: &str, index: u8) -> Option<&LedgerField> {
        let declaration = *self.declarations.ledger.get(field)?;
        let path = declaration.physical_path();
        (declaration.index == index
            && path.first() == Some(&index)
            && (1..=2).contains(&path.len()))
        .then_some(declaration)
    }

    // Keep this recursive Map probe out of the already deep expression frame.
    // The scoped query and type checks are identical to the inline branch.
    fn composition_map_member(
        &mut self,
        field: &str,
        index: u8,
        key: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        if !composition::string_key_map(&self.field(field, index)?.declaration) {
            return None;
        }
        let key = self.expression(key, scope, steps)?;
        if key.ty != Type::OpaqueString {
            return None;
        }
        self.observe(
            field,
            "record_member",
            vec![key.value],
            Type::Boolean,
            steps,
        )
    }

    fn composition_map_lookup(
        &mut self,
        field: &str,
        index: u8,
        key: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let LedgerFieldKind::Map {
            key: Type::OpaqueString,
            value,
        } = &self.field(field, index)?.declaration
        else {
            return None;
        };
        let ty = value.clone();
        if !composition::checked_product_map(&self.field(field, index)?.declaration) {
            return None;
        }
        let key = self.expression(key, scope, steps)?;
        if key.ty != Type::OpaqueString {
            return None;
        }
        self.observe(field, "record_lookup", vec![key.value], ty, steps)
    }

    // Recursion returns through this small dispatcher; helpers keep the same Plan and scope.
    fn expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::Sequence { .. } | Expr::Assert { .. } | Expr::Let { .. } | Expr::If { .. } => {
                self.control_expression(expression, scope, steps)
            }
            Expr::StructLiteral { .. } | Expr::StructField { .. } | Expr::Tuple { .. } => {
                self.aggregate_expression(expression, scope, steps)
            }
            Expr::UnsignedCast { .. }
            | Expr::UnsignedMultiply { .. }
            | Expr::NotEqual { .. }
            | Expr::JubjubPointX { .. }
            | Expr::JubjubPointY { .. }
            | Expr::UnsignedAdd { .. }
            | Expr::UnsignedSubtract { .. }
            | Expr::Coerce { .. }
            | Expr::Equal { .. }
            | Expr::Add { .. }
            | Expr::FieldCast { .. }
            | Expr::FieldToBytes32 { .. } => self.arithmetic_expression(expression, scope, steps),
            Expr::JubjubScalarFromNative { .. }
            | Expr::EcMulGenerator { .. }
            | Expr::EcMul { .. }
            | Expr::EcAdd { .. } => jubjub_values::expression(self, expression, scope, steps),
            Expr::PersistentCommit { .. }
            | Expr::PersistentHash { .. }
            | Expr::TransientCommit { .. }
            | Expr::DegradeToTransient { .. }
            | Expr::TransientHash { .. }
            | Expr::UpgradeFromTransient { .. } => self.crypto_expression(expression, scope, steps),
            Expr::CounterLessThan { .. }
            | Expr::CounterRead { .. }
            | Expr::CellRead { .. }
            | Expr::SetMember { .. }
            | Expr::MapMember { .. }
            | Expr::MapLookup { .. }
            | Expr::SetSize { .. }
            | Expr::SetIsEmpty { .. }
            | Expr::HistoricMerkleCheckRoot { .. }
            | Expr::MerkleCheckRoot { .. } => self.observation_expression(expression, scope, steps),
            Expr::KernelMintShielded { .. }
            | Expr::CreateZswapInput { .. }
            | Expr::CreateZswapOutput { .. }
            | Expr::KernelClaim { .. }
            | Expr::KernelSelf { .. }
            | Expr::WitnessCall { .. }
            | Expr::NativeWitnessCall { .. }
            | Expr::Call { .. } => self.effect_expression(expression, scope, steps),
            Expr::Unit
            | Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. }
            | Expr::FieldLiteral { .. }
            | Expr::UnsignedLiteral { .. }
            | Expr::Default { .. } => self.leaf_expression(expression, scope, steps),
            _ => None,
        }
    }

    // Resolve declaration kind first; profile policy then determines whether
    // its audited body is inlined or a proven-pure generated helper is called.
    fn call(
        &mut self,
        name: &str,
        arguments: &[Expr],
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        if let Some(calls) = &self.admission.composition_calls {
            let call = *calls.get(name)?;
            return match call {
                composition::AuditedCall::PureValue(callee) => {
                    let args = self.arguments(arguments, &callee.parameters, scope, steps)?;
                    let method = ident(name).ok()?;
                    self.bind(
                        syn::parse_quote!(crate::pure_circuits::#method(#(#args),*)?),
                        callee.result.clone(),
                        steps,
                    )
                }
                composition::AuditedCall::PureUnitGuard(callee) => {
                    let args = self.arguments(arguments, &callee.parameters, scope, steps)?;
                    let method = ident(name).ok()?;
                    steps.push(syn::parse_quote!(crate::pure_circuits::#method(#(#args),*)?;));
                    Some(TypedValue {
                        ty: Type::Unit,
                        value: syn::parse_quote!(()),
                    })
                }
                composition::AuditedCall::LocalUnit(callee) => {
                    let args = self.arguments(arguments, &callee.parameters, scope, steps)?;
                    let method = ident(name).ok()?;
                    let uses_witness = circuit_uses_witness(
                        callee,
                        self.declarations.stateful_circuits?,
                        &mut HashSet::new(),
                    )
                    .ok()?;
                    let witness_arg = uses_witness.then(|| syn::parse_quote!(witnesses));
                    let mut call_args = vec![syn::parse_quote!(context)];
                    call_args.extend(witness_arg);
                    call_args.extend(args);
                    steps.push(syn::parse_quote!(let (frame, ()) = frame.call_local(|context| super::#method(#(#call_args),*))?;));
                    Some(TypedValue {
                        ty: Type::Unit,
                        value: syn::parse_quote!(()),
                    })
                }
                composition::AuditedCall::RecordedUnit(callee) => self.inline_call(
                    name,
                    &callee.parameters,
                    &callee.result,
                    &Expr::Unit,
                    &callee.actions,
                    arguments,
                    scope,
                    steps,
                    false,
                ),
                composition::AuditedCall::ReadOnlyBoolean(callee) => {
                    let value = composition::boolean_result(callee)?;
                    self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        &value,
                        &[],
                        arguments,
                        scope,
                        steps,
                        false,
                    )
                }
            };
        }
        let pure = self.declarations.pure.get(name).copied();
        let stateful = self
            .declarations
            .stateful_circuits
            .and_then(|map| map.get(name))
            .copied();
        match (pure, stateful) {
            (Some(callee), None) => {
                if self.admission.composite_domain.values()
                    && !self.admission.composite_domain.shielded_helpers()
                {
                    return None;
                } // preserve ADR0187 admission
                if self.evaluation.context_query
                    || (self.admission.unit_actions && !self.admission.composite_domain.intents())
                {
                    self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        &callee.body,
                        &[],
                        arguments,
                        scope,
                        steps,
                        false,
                    )
                } else {
                    if !self.pure_call(name, &mut HashSet::new()) {
                        return None;
                    }
                    let args = self.arguments(arguments, &callee.parameters, scope, steps)?;
                    let method = ident(name).ok()?;
                    self.bind(
                        syn::parse_quote!(crate::pure_circuits::#method(#(#args),*)?),
                        callee.result.clone(),
                        steps,
                    )
                }
            }
            (None, Some(callee)) => {
                if self.admission.composite_domain == CompositeDomain::ResetShieldedPayout
                    && phase_reset::helper_signature(callee)
                {
                    if !reset_payout::true_arguments(arguments)
                        || !reset_payout::helper_shape(callee, self.declarations.pure)
                    {
                        return None;
                    }
                    return self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        &Expr::Unit,
                        &callee.actions,
                        arguments,
                        scope,
                        steps,
                        false,
                    );
                }
                if self.admission.composite_domain == CompositeDomain::VotingCommit
                    && callee.actions.is_empty()
                    && let StateReturn::Expression { value } = &callee.return_value
                {
                    let scalar = scalar_counter_hash(callee, self.declarations.ledger);
                    let context = callee.parameters.is_empty()
                        && callee.result == (Type::Bytes { length: 32 })
                        && context_query_value(
                            value,
                            self.declarations.pure,
                            &mut HashSet::new(),
                            true,
                        );
                    if scalar || context {
                        let old = self.evaluation.context_query;
                        self.evaluation.context_query = context;
                        let result = self.inline_call(
                            name,
                            &callee.parameters,
                            &callee.result,
                            value,
                            &[],
                            arguments,
                            scope,
                            steps,
                            scalar,
                        );
                        self.evaluation.context_query = old;
                        return result;
                    }
                }
                if self.admission.composite_domain.shielded_helpers() {
                    if self.admission.composite_domain.singleton_bridge()
                        && shielded_unit_signature(callee)
                    {
                        return self.inline_call(
                            name,
                            &callee.parameters,
                            &callee.result,
                            &Expr::Unit,
                            &callee.actions,
                            arguments,
                            scope,
                            steps,
                            false,
                        );
                    }
                    let StateReturn::Expression { value } = &callee.return_value else {
                        return None;
                    };
                    if !callee.actions.is_empty()
                        && !(self.admission.composite_domain
                            == CompositeDomain::ActionfulShieldedPayout
                            && shielded_payout::action_shape(
                                &callee.actions,
                                self.declarations.pure,
                            ) == Some(1))
                    {
                        return None;
                    }
                    return self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        value,
                        &callee.actions,
                        arguments,
                        scope,
                        steps,
                        false,
                    );
                }
                if self.admission.composite_domain == CompositeDomain::TerminalReturns {
                    let StateReturn::Expression { value } = &callee.return_value else {
                        return None;
                    };
                    if callee.result != Type::Field {
                        return None;
                    }
                    return self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        value,
                        &callee.actions,
                        arguments,
                        scope,
                        steps,
                        false,
                    );
                }
                if self.admission.phase_reset {
                    if !phase_reset::helper_signature(callee)
                        || !phase_reset::false_arguments(arguments)
                    {
                        return None;
                    }
                    return self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        &Expr::Unit,
                        &callee.actions,
                        arguments,
                        scope,
                        steps,
                        false,
                    );
                }
                if self.admission.unit_actions {
                    if !(self.admission.composite_domain.intents()
                        && shielded_unit_signature(callee))
                        && !unit_actions::helper_signature(callee)
                    {
                        return None;
                    }
                    return self.inline_call(
                        name,
                        &callee.parameters,
                        &callee.result,
                        &Expr::Unit,
                        &callee.actions,
                        arguments,
                        scope,
                        steps,
                        false,
                    );
                }
                let value = match &callee.return_value {
                    StateReturn::Expression { value } => value.clone(),
                    StateReturn::CellRead { field, index }
                        if self.admission.composite_domain
                            == CompositeDomain::FieldObservations =>
                    {
                        Expr::CellRead {
                            field: field.clone(),
                            index: *index,
                        }
                    }
                    _ => return None,
                };
                if !callee.actions.is_empty() {
                    return None;
                }
                let scalar = self.admission.counter_hash_helpers
                    && scalar_counter_hash(callee, self.declarations.ledger);
                if !self.admission.composite_domain.values() && !scalar {
                    return None;
                }
                self.inline_call(
                    name,
                    &callee.parameters,
                    &callee.result,
                    &value,
                    &[],
                    arguments,
                    scope,
                    steps,
                    scalar,
                )
            }
            _ => None, // absent or ambiguous declaration; never guess a helper kind
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn inline_call(
        &mut self,
        name: &str,
        parameters: &[crate::ir::Parameter],
        result: &Type,
        body: &Expr,
        actions: &[StateAction],
        arguments: &[Expr],
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
        scalar: bool,
    ) -> Option<TypedValue> {
        if parameters
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != parameters.len()
        {
            return None;
        }
        // Arguments belong to the caller; even repeated finite calls evaluate
        // before entering the callee's cycle guard and isolated parameter scope.
        let old_arguments = self.evaluation.scalar_arguments;
        self.evaluation.scalar_arguments = scalar;
        let arguments = self.arguments(arguments, parameters, scope, steps);
        self.evaluation.scalar_arguments = old_arguments;
        let arguments = arguments?;
        if !self.evaluation.active_calls.insert(name.to_owned()) {
            return None;
        }
        let isolated = parameters
            .iter()
            .zip(arguments)
            .map(|(p, value)| {
                (
                    p.name.clone(),
                    TypedValue {
                        ty: p.ty.clone(),
                        value,
                    },
                )
            })
            .collect();
        if scalar {
            self.evaluation.scalar_body_depth += 1;
        }
        let value = if self.admission.composite_domain == CompositeDomain::TerminalReturns
            || (self.admission.composite_domain == CompositeDomain::ActionfulShieldedPayout
                && !actions.is_empty())
        {
            self.return_plan(&terminal_returns::adapt(actions, body), &isolated, steps)
        } else {
            (|| {
                for action in actions {
                    self.action(action, &isolated, steps)?;
                }
                self.expression(body, &isolated, steps)
            })()
        };
        if scalar {
            self.evaluation.scalar_body_depth -= 1;
            self.evidence.scalar_helper_calls += 1;
        }
        self.evaluation.active_calls.remove(name);
        let value = value?;
        (value.ty == *result).then_some(value)
    }

    fn arguments(
        &mut self,
        arguments: &[Expr],
        formals: &[crate::ir::Parameter],
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<Vec<syn::Expr>> {
        if arguments.len() != formals.len() {
            return None;
        }
        let mut args = Vec::new();
        for (argument, formal) in arguments.iter().zip(formals) {
            let value = self.expression(argument, scope, steps)?;
            if value.ty != formal.ty {
                return None;
            }
            args.push(value.value);
        }
        Some(args)
    }

    fn observe(
        &mut self,
        field: &str,
        operation: &str,
        args: Vec<syn::Expr>,
        ty: Type,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let slot = ident(field).ok()?;
        let method = ident(operation).ok()?;
        let observed = self.fresh();
        let rust_ty = rust_type(&ty).ok()?;
        steps.push(syn::parse_quote! {
            let (frame, #observed): (_, #rust_ty) = crate::ledger_slots::#slot.#method(frame, #(#args),*)?;
        });
        Some(TypedValue {
            ty,
            value: syn::parse_quote!(#observed),
        })
    }

    fn bindings(
        &mut self,
        bindings: &[LocalBinding],
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<Scope> {
        let mut scoped = scope.clone();
        for binding in bindings {
            let value = self.expression(&binding.value, &scoped, steps)?;
            if value.ty != binding.ty {
                return None;
            }
            // Evaluate once at the lexical declaration, including pure aliases.
            let value = self.bind(value.value, value.ty, steps)?;
            scoped.insert(binding.name.clone(), value);
        }
        Some(scoped)
    }

    fn return_plan(
        &mut self,
        body: &ReturnPlan,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match body {
            ReturnPlan::Value { value } => self.expression(value, scope, steps),
            ReturnPlan::Sequence { actions, result } => {
                for action in actions {
                    self.action(action, scope, steps)?;
                }
                self.return_plan(result, scope, steps)
            }
            ReturnPlan::Let { bindings, result } => {
                let scoped = self.bindings(bindings, scope, steps)?;
                self.return_plan(result, &scoped, steps)
            }
            ReturnPlan::Conditional {
                condition,
                then,
                otherwise,
            } => {
                let condition = self.expression(condition, scope, steps)?;
                if condition.ty != Type::Boolean {
                    return None;
                }
                let mut then_steps = Vec::new();
                let mut else_steps = Vec::new();
                let then = self.return_plan(then, scope, &mut then_steps)?;
                let otherwise = self.return_plan(otherwise, scope, &mut else_steps)?;
                if then.ty != otherwise.ty {
                    return None;
                }
                let result_ty = rust_type(&then.ty).ok()?;
                let observed = self.fresh();
                let (condition, then_value, else_value) =
                    (condition.value, then.value, otherwise.value);
                steps.push(syn::parse_quote! {
                    let (frame, #observed): (_, #result_ty) = if #condition {
                        #(#then_steps)*
                        (frame, #then_value)
                    } else {
                        #(#else_steps)*
                        (frame, #else_value)
                    };
                });
                Some(TypedValue {
                    ty: then.ty,
                    value: syn::parse_quote!(#observed),
                })
            }
        }
    }

    fn action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        match action {
            StateAction::Sequence { actions } => {
                for action in actions {
                    self.action(action, scope, steps)?;
                }
            }
            StateAction::Let { bindings, action } => {
                let scoped = self.bindings(bindings, scope, steps)?;
                self.action(action, &scoped, steps)?;
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = self.expression(condition, scope, steps)?;
                if condition.ty != Type::Boolean {
                    return None;
                }
                let mut then_steps = Vec::new();
                let mut else_steps = Vec::new();
                self.action(then, scope, &mut then_steps)?;
                self.action(otherwise, scope, &mut else_steps)?;
                let condition = condition.value;
                steps.push(syn::parse_quote! {
                    #[allow(clippy::let_and_return, reason = "uniform branch frames preserve ordered recording steps")]
                    let frame = if #condition {
                        #(#then_steps)*
                        frame
                    } else {
                        #(#else_steps)*
                        frame
                    };
                });
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                self.lower_cell_write(field, *index, value, scope, steps)?;
            }
            StateAction::CellWriteCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                self.lower_cell_coin_write(field, *index, coin, recipient, scope, steps)?;
            }
            StateAction::CounterReset { field, index }
                if self.admission.phase_reset
                    || self.admission.composite_domain == CompositeDomain::ResetShieldedPayout =>
            {
                self.lower_counter_reset(field, *index, steps)?;
            }
            StateAction::MerkleResetToDefault { field, index }
                if self.admission.phase_reset
                    || self.admission.composite_domain == CompositeDomain::ResetShieldedPayout =>
            {
                self.lower_merkle_reset(field, *index, steps)?;
            }
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            } => {
                self.lower_counter_increment(field, *index, amount, scope, steps)?;
            }
            StateAction::Assert { condition, message } => {
                self.lower_assertion(condition, message, scope, steps)?;
            }
            StateAction::Expression {
                value: value @ Expr::WitnessCall { .. },
            } => {
                self.lower_unit_expression(value, scope, steps)?;
            }
            StateAction::Expression { value }
                if self.admission.unit_actions && self.admission.composite_domain.intents() =>
            {
                self.lower_unit_expression(value, scope, steps)?;
            }
            StateAction::PureCall { name, arguments }
                if self.admission.composition_calls.is_some() =>
            {
                self.lower_pure_guard_call(name, arguments, scope, steps)?;
            }
            StateAction::CircuitCall { name, arguments }
                if (self.admission.unit_actions && self.admission.composite_domain.intents())
                    || self.admission.phase_reset
                    || self.admission.composition_calls.is_some() =>
            {
                self.lower_unit_circuit_call(name, arguments, scope, steps)?;
            }
            StateAction::MerkleInsert {
                field,
                index,
                value,
            }
            | StateAction::HistoricMerkleInsert {
                field,
                index,
                value,
            }
            | StateAction::SetInsert {
                field,
                index,
                value,
            } => {
                self.lower_collection_insert(action, field, *index, value, scope, steps)?;
            }
            StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                self.lower_set_remove(field, *index, value, scope, steps)?;
            }
            StateAction::MapInsert {
                field,
                index,
                key,
                value,
            } if self.admission.composition_calls.is_some() => {
                self.lower_map_insert(field, *index, key, value, scope, steps)?;
            }
            StateAction::MapRemove { field, index, key }
                if self.admission.composition_calls.is_some() =>
            {
                self.lower_map_remove(field, *index, key, scope, steps)?;
            }
            StateAction::SetReset { field, index } => {
                self.lower_set_reset(field, *index, steps)?;
            }
            StateAction::SetInsertCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                self.lower_set_coin_insert(field, *index, coin, recipient, scope, steps)?;
            }
            _ => return None,
        }
        Some(())
    }
}

// Bounded scalar helper domain: one canonical Counter query in a three-byte
// value hash. The matcher checks declared slots/types and never circuit names.
fn scalar_counter_hash(circuit: &StatefulCircuit, ledger: &HashMap<&str, &LedgerField>) -> bool {
    let bytes = Type::Bytes { length: 32 };
    if circuit.result != bytes
        || !circuit.actions.is_empty()
        || circuit.parameters.is_empty()
        || circuit.parameters.len() > 2
        || circuit.parameters.iter().any(|p| p.ty != bytes)
        || circuit
            .parameters
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != circuit.parameters.len()
    {
        return false;
    }
    let StateReturn::Expression {
        value: Expr::PersistentHash { value },
    } = &circuit.return_value
    else {
        return false;
    };
    let Expr::Tuple { elements } = &**value else {
        return false;
    };
    if elements.len() != 3 {
        return false;
    }
    let mut reads = 0;
    let valid = elements.iter().all(|element| match element {
        Expr::BytesLiteral { bytes } => bytes.len() == 32,
        Expr::Parameter { name } => circuit.parameters.iter().any(|p| &p.name == name),
        Expr::FieldToBytes32 { value } => {
            let Expr::FieldCast { value } = &**value else {
                return false;
            };
            let Expr::CounterRead { field, index } = &**value else {
                return false;
            };
            reads += 1;
            ledger.get(field.as_str()).is_some_and(|slot| {
                slot.index == *index && slot.declaration == LedgerFieldKind::Counter
            })
        }
        _ => false,
    });
    valid && reads == 1
}

fn optional_string(ty: &Type) -> bool {
    matches!(ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [present, value] if present.ty == Type::Boolean && value.ty == Type::OpaqueString))
}

fn contract_address_type() -> Type {
    Type::Struct {
        name: "ContractAddress".into(),
        fields: vec![crate::ir::StructField {
            name: "bytes".into(),
            ty: Type::Bytes { length: 32 },
        }],
    }
}

fn wrapped_bytes32(ty: &Type) -> bool {
    matches!(ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == (Type::Bytes { length: 32 })))
}
fn bytes32_key(ty: &Type) -> bool {
    *ty == (Type::Bytes { length: 32 }) || wrapped_bytes32(ty)
}
fn cell_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Enum { .. } | Type::Bytes { length: 32 } | Type::OpaqueBytes
    ) || optional_string(ty)
}

pub(super) struct TypedPlan {
    pub steps: Vec<syn::Stmt>,
    pub result: syn::Expr,
}

// Read-only assertion bodies have their own admission boundary. General calls,
// writes, cryptographic effects and non-Boolean witness results remain outside it.
fn assertion_value(
    value: &Expr,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    assertions: &mut usize,
) -> bool {
    match value {
        Expr::Unit
        | Expr::Boolean { .. }
        | Expr::Parameter { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. } => true,
        Expr::CounterLessThan { threshold, .. } => {
            assertion_value(threshold, witnesses, assertions)
        }
        Expr::Coerce { value, ty } if assertion_type(ty) => {
            assertion_value(value, witnesses, assertions)
        }
        Expr::Assert { condition, .. } => {
            *assertions += 1;
            assertion_value(condition, witnesses, assertions)
        }
        Expr::Sequence { steps, value } => {
            steps
                .iter()
                .all(|step| assertion_value(step, witnesses, assertions))
                && assertion_value(value, witnesses, assertions)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            assertion_value(condition, witnesses, assertions)
                && assertion_value(then, witnesses, assertions)
                && assertion_value(otherwise, witnesses, assertions)
        }
        Expr::Let { bindings, body } => {
            bindings.iter().all(|binding| {
                assertion_type(&binding.ty)
                    && assertion_value(&binding.value, witnesses, assertions)
            }) && assertion_value(body, witnesses, assertions)
        }
        Expr::WitnessCall { name, arguments } => witnesses.get(name.as_str()).is_some_and(|w| {
            w.result == Type::Boolean
                && w.parameters
                    .iter()
                    .all(|p| p.ty == (Type::Unsigned { max: "255".into() }))
                && arguments
                    .iter()
                    .all(|arg| assertion_value(arg, witnesses, assertions))
        }),
        _ => false,
    }
}
fn assertion_type(ty: &Type) -> bool {
    matches!(ty, Type::Unit | Type::Boolean)
        || matches!(ty, Type::Unsigned { max } if max == "255" || max == "18446744073709551615")
}

// The context query profile is intentionally independent of ledger-slot and
// composite-return admission. Pure callees are audited before typed inlining.
fn context_query_type(ty: &Type) -> bool {
    *ty == (Type::Bytes { length: 32 }) || *ty == contract_address_type()
}
fn context_query_value(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    visiting: &mut HashSet<String>,
    allow_context: bool,
) -> bool {
    match value {
        Expr::Parameter { .. } => true,
        Expr::BytesLiteral { bytes } => bytes.len() == 32,
        Expr::KernelSelf { ty } => allow_context && *ty == contract_address_type(),
        Expr::StructField { value, .. } => {
            context_query_value(value, pure, visiting, allow_context)
        }
        Expr::Coerce { value, ty } => {
            context_query_type(ty) && context_query_value(value, pure, visiting, allow_context)
        }
        Expr::Tuple { elements } => elements
            .iter()
            .all(|v| context_query_value(v, pure, visiting, allow_context)),
        Expr::PersistentCommit { value, opening } => {
            context_query_value(value, pure, visiting, allow_context)
                && context_query_value(opening, pure, visiting, allow_context)
        }
        Expr::Let { bindings, body } => {
            bindings.iter().all(|b| {
                context_query_type(&b.ty)
                    && context_query_value(&b.value, pure, visiting, allow_context)
            }) && context_query_value(body, pure, visiting, allow_context)
        }
        Expr::Call { name, arguments } => {
            if !arguments
                .iter()
                .all(|a| context_query_value(a, pure, visiting, allow_context))
                || !visiting.insert(name.clone())
            {
                return false;
            }
            let valid = pure.get(name.as_str()).is_some_and(|callee| {
                callee.result == (Type::Bytes { length: 32 })
                    && callee
                        .parameters
                        .iter()
                        .map(|p| &p.name)
                        .collect::<HashSet<_>>()
                        .len()
                        == callee.parameters.len()
                    && callee.parameters.iter().all(|p| context_query_type(&p.ty))
                    && context_query_value(&callee.body, pure, visiting, false)
            });
            visiting.remove(name);
            valid
        }
        _ => false,
    }
}
pub(super) fn lower_context_query<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
) -> Option<TypedPlan> {
    if !circuit.parameters.is_empty()
        || !circuit.actions.is_empty()
        || circuit.result != (Type::Bytes { length: 32 })
    {
        return None;
    }
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    if !context_query_value(value, pure, &mut HashSet::new(), true) {
        return None;
    }
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: None,
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: true,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let mut steps = Vec::new();
    let result = plan.expression(value, &Scope::new(), &mut steps)?;
    (result.ty == circuit.result
        && plan.evidence.kernel_self_reads > 0
        && plan.evidence.witness_calls == 0)
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
}

fn composite_type(ty: &Type, intents: bool) -> bool {
    match ty {
        Type::Unit if intents => true,
        Type::Boolean | Type::Bytes { length: 32 } => true,
        Type::Unsigned { max } => matches!(
            crate::unsigned_maximum(max),
            Ok(crate::UnsignedMaximum::Small(_))
        ),
        Type::Struct { fields, .. } => {
            !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| composite_type(&field.ty, intents))
        }
        _ => false,
    }
}
fn composite_value(
    value: &Expr,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    intents: bool,
) -> bool {
    match value {
        Expr::CreateZswapInput { coin } if intents => {
            composite_value(coin, witnesses, circuits, visiting, intents)
        }
        Expr::CreateZswapOutput { coin, recipient } if intents => [coin, recipient]
            .iter()
            .all(|v| composite_value(v, witnesses, circuits, visiting, intents)),
        Expr::KernelClaim { value, .. } if intents => {
            composite_value(value, witnesses, circuits, visiting, intents)
        }
        Expr::Parameter { .. } | Expr::Boolean { .. } => true,
        Expr::UnsignedLiteral { max, .. } => {
            composite_type(&Type::Unsigned { max: max.clone() }, intents)
        }
        Expr::KernelSelf { ty } => *ty == contract_address_type(),
        Expr::Default {
            ty: ty @ Type::Struct { .. },
        } => composite_type(ty, intents),
        Expr::StructLiteral { ty, fields } => {
            composite_type(ty, intents)
                && fields
                    .iter()
                    .all(|field| composite_value(field, witnesses, circuits, visiting, intents))
        }
        Expr::Coerce { value, ty } => {
            composite_type(ty, intents)
                && composite_value(value, witnesses, circuits, visiting, intents)
        }
        Expr::UnsignedCast { value, max } => {
            composite_type(&Type::Unsigned { max: max.clone() }, intents)
                && composite_value(value, witnesses, circuits, visiting, intents)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => [condition, then, otherwise]
            .iter()
            .all(|v| composite_value(v, witnesses, circuits, visiting, intents)),
        Expr::Let { bindings, body } => {
            bindings.iter().all(|binding| {
                composite_type(&binding.ty, intents)
                    && composite_value(&binding.value, witnesses, circuits, visiting, intents)
            }) && composite_value(body, witnesses, circuits, visiting, intents)
        }
        Expr::WitnessCall { name, arguments } => witnesses.get(name.as_str()).is_some_and(|w| {
            w.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                })
                && w.parameters
                    .iter()
                    .all(|p| p.ty == (Type::Unsigned { max: "255".into() }))
                && arguments
                    .iter()
                    .all(|a| composite_value(a, witnesses, circuits, visiting, intents))
        }),
        Expr::Call { name, arguments } => {
            if !arguments
                .iter()
                .all(|a| composite_value(a, witnesses, circuits, visiting, intents))
                || !visiting.insert(name.clone())
            {
                return false;
            }
            let valid = circuits.get(name.as_str()).is_some_and(|callee| {
                composite_circuit(callee, witnesses, circuits, visiting, intents)
            });
            visiting.remove(name);
            valid
        }
        _ => false,
    }
}
fn composite_circuit(
    circuit: &StatefulCircuit,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    intents: bool,
) -> bool {
    circuit.actions.is_empty()
        && matches!(circuit.result, Type::Struct { .. })
        && composite_type(&circuit.result, intents)
        && circuit.parameters.iter().all(|p| {
            p.ty == Type::Boolean
                || (intents
                    && (p.ty == shielded_coin_type()
                        || p.ty == qualified_coin_type()
                        || p.ty == shielded_recipient_type()
                        || p.ty == (Type::Bytes { length: 32 })))
        })
        && matches!(&circuit.return_value, StateReturn::Expression { value } if composite_value(value,witnesses,circuits,visiting,intents))
}

fn shielded_unit_signature(circuit: &StatefulCircuit) -> bool {
    circuit.result == Type::Unit
        && circuit.return_value == StateReturn::Unit
        && matches!(circuit.parameters.as_slice(), [parameter] if parameter.ty == shielded_coin_type())
        && !circuit.actions.is_empty()
}

fn shielded_receive_value(value: &Expr, pure: &HashMap<&str, &PureCircuit>) -> bool {
    match value {
        Expr::Parameter { .. }
        | Expr::Boolean { .. }
        | Expr::BytesLiteral { .. }
        | Expr::FieldLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Default {
            ty: Type::Struct { .. },
        } => true,
        Expr::KernelSelf { ty } => *ty == contract_address_type(),
        Expr::StructLiteral { fields, .. } | Expr::Tuple { elements: fields } => fields
            .iter()
            .all(|field| shielded_receive_value(field, pure)),
        Expr::StructField { value, .. } | Expr::Coerce { value, .. } => {
            shielded_receive_value(value, pure)
        }
        Expr::Equal { left, right } => {
            shielded_receive_value(left, pure) && shielded_receive_value(right, pure)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => [condition, then, otherwise]
            .iter()
            .all(|part| shielded_receive_value(part, pure)),
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .all(|binding| shielded_receive_value(&binding.value, pure))
                && shielded_receive_value(body, pure)
        }
        Expr::Call { name, arguments } => {
            pure.contains_key(name.as_str())
                && arguments
                    .iter()
                    .all(|argument| shielded_receive_value(argument, pure))
        }
        // Every other node may query, witness, mutate, open a commitment, or
        // introduce a nested effect in an unselected branch or unused local.
        _ => false,
    }
}

// Only an ordered Unit body containing output creation and its receive claim
// enters this profile. The typed Plan then checks every operand, binding,
// pure declaration and final effect count; no helper/source name is privileged.
fn shielded_receive_action(
    action: &StateAction,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    active: &mut HashSet<String>,
) -> bool {
    match action {
        StateAction::Sequence { actions } => actions
            .iter()
            .all(|action| shielded_receive_action(action, pure, circuits, active)),
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .all(|binding| shielded_receive_value(&binding.value, pure))
                && shielded_receive_action(action, pure, circuits, active)
        }
        StateAction::Expression {
            value: Expr::CreateZswapOutput { coin, recipient },
        } => shielded_receive_value(coin, pure) && shielded_receive_value(recipient, pure),
        StateAction::Expression {
            value:
                Expr::KernelClaim {
                    claim: KernelClaimKind::CoinReceive,
                    value,
                },
        } => shielded_receive_value(value, pure),
        StateAction::CircuitCall { name, arguments } => {
            if !arguments
                .iter()
                .all(|argument| shielded_receive_value(argument, pure))
            {
                return false;
            }
            let Some(callee) = circuits.get(name.as_str()) else {
                return false;
            };
            if !shielded_unit_signature(callee) || !active.insert(name.clone()) {
                return false;
            }
            let valid = callee
                .actions
                .iter()
                .all(|action| shielded_receive_action(action, pure, circuits, active));
            active.remove(name);
            valid
        }
        _ => false,
    }
}

pub(super) fn lower_shielded_receive<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    if !shielded_unit_signature(circuit)
        || !matches!(
            circuit.actions.as_slice(),
            [StateAction::CircuitCall { .. }]
        )
        || !circuit.actions.iter().all(|action| {
            shielded_receive_action(
                action,
                pure,
                circuits,
                &mut HashSet::from([circuit.name.clone()]),
            )
        })
    {
        return None;
    }
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: Some(circuits),
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: true,
            phase_reset: false,
            composite_domain: CompositeDomain::ShieldedReceive,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let parameter = &circuit.parameters[0];
    let scope = Scope::from([(
        parameter.name.clone(),
        TypedValue {
            ty: parameter.ty.clone(),
            value: syn::parse_quote!(__compact_param_0),
        },
    )]);
    let mut steps = Vec::new();
    for action in &circuit.actions {
        plan.action(action, &scope, &mut steps)?;
    }
    (plan.evidence.kernel_self_reads == 1
        && plan.evidence.intent_effects == 1
        && plan.evidence.intent_queries == 1
        && plan.evidence.witness_calls == 0
        && plan.evidence.cell_reads == 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.counter_writes == 0)
        .then_some(TypedPlan {
            steps,
            result: syn::parse_quote!(()),
        })
}

pub(super) fn lower_terminal_returns<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    terminal_returns::lower(circuit, ledger, witnesses, pure, circuits)
}

// A send result is a typed pair of the sent coin and optional change. The
// admission predicate describes the public type, not a helper/source name.
fn shielded_send_result(ty: &Type) -> bool {
    let Type::Struct { fields, .. } = ty else {
        return false;
    };
    let [change, sent] = fields.as_slice() else {
        return false;
    };
    if change.name != "change" || sent.name != "sent" || sent.ty != shielded_coin_type() {
        return false;
    }
    matches!(&change.ty, Type::Struct { fields, .. }
        if matches!(fields.as_slice(), [present, value]
            if present.name == "is_some" && present.ty == Type::Boolean
                && value.name == "value" && value.ty == sent.ty))
}

// Inspect every expression, including unused locals, arguments and unselected
// branches. The Plan verifies the exact types and evaluates the ordered body;
// this audit closes admission over its deliberately smaller effect domain.
fn shielded_send_value(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> bool {
    shielded_send_value_checked(value, pure, circuits, visiting).is_ok()
}

fn shielded_send_value_checked(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> Result<(), RecordingGap> {
    let mut failure = None;
    if shielded_value_at(
        value,
        pure,
        circuits,
        visiting,
        CompositeDomain::ShieldedSend,
        "return_value.value",
        &mut failure,
    ) {
        Ok(())
    } else {
        Err(failure.expect("rejected shielded value has a first failed expression"))
    }
}

fn shielded_value(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    domain: CompositeDomain,
) -> bool {
    shielded_value_at(
        value,
        pure,
        circuits,
        visiting,
        domain,
        "return_value.value",
        &mut None,
    )
}

fn shielded_value_at(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    domain: CompositeDomain,
    path: &str,
    failure: &mut Option<RecordingGap>,
) -> bool {
    let mut visit = |part: &Expr, suffix: &str, visiting: &mut HashSet<String>| {
        shielded_value_at(
            part,
            pure,
            circuits,
            visiting,
            domain,
            &format!("{path}.{suffix}"),
            failure,
        )
    };
    let valid =
        match value {
            Expr::NativeWitnessCall {
                builtin: crate::ir::NativeWitnessBuiltin::OwnPublicKey,
            } if domain == CompositeDomain::FundedShieldedMint => true,
            Expr::KernelMintShielded {
                domain: token_domain,
                amount,
            } if domain == CompositeDomain::FundedShieldedMint => {
                visit(token_domain, "domain", visiting) && visit(amount, "amount", visiting)
            }
            Expr::CounterRead { .. } if domain == CompositeDomain::ResetShieldedPayout => true,
            Expr::CounterLessThan { threshold, .. }
                if domain == CompositeDomain::ResetShieldedPayout =>
            {
                visit(threshold, "threshold", visiting)
            }
            Expr::CellRead { .. } | Expr::NativeWitnessCall { .. }
                if matches!(
                    domain,
                    CompositeDomain::ShieldedPayout
                        | CompositeDomain::ActionfulShieldedPayout
                        | CompositeDomain::ResetShieldedPayout
                ) =>
            {
                true
            }
            Expr::WitnessCall { arguments, .. }
                if matches!(
                    domain,
                    CompositeDomain::ShieldedPayout
                        | CompositeDomain::ActionfulShieldedPayout
                        | CompositeDomain::ResetShieldedPayout
                ) =>
            {
                arguments
                    .iter()
                    .enumerate()
                    .all(|(index, value)| visit(value, &format!("arguments[{index}]"), visiting))
            }
            Expr::Assert { condition, .. }
                if matches!(
                    domain,
                    CompositeDomain::ShieldedPayout
                        | CompositeDomain::ActionfulShieldedPayout
                        | CompositeDomain::ResetShieldedPayout
                ) || domain.shielded_merge() =>
            {
                visit(condition, "condition", visiting)
            }
            Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::BytesLiteral { .. }
            | Expr::FieldLiteral { .. }
            | Expr::UnsignedLiteral { .. }
            | Expr::EnumVariant { .. }
            | Expr::Unit
            | Expr::Default {
                ty: Type::Struct { .. },
            } => true,
            Expr::KernelSelf { ty } => *ty == contract_address_type(),
            Expr::StructLiteral { fields, .. } => fields
                .iter()
                .enumerate()
                .all(|(index, part)| visit(part, &format!("fields[{index}]"), visiting)),
            Expr::Tuple { elements } => elements
                .iter()
                .enumerate()
                .all(|(index, part)| visit(part, &format!("elements[{index}]"), visiting)),
            Expr::StructField { value, .. }
            | Expr::Coerce { value, .. }
            | Expr::UnsignedCast { value, .. }
            | Expr::DegradeToTransient { value }
            | Expr::TransientHash { value }
            | Expr::UpgradeFromTransient { value }
            | Expr::KernelClaim { value, .. } => visit(value, "value", visiting),
            Expr::CreateZswapInput { coin } => visit(coin, "coin", visiting),
            Expr::UnsignedAdd { left, right, .. } if domain.shielded_merge() => {
                visit(left, "left", visiting) && visit(right, "right", visiting)
            }
            Expr::CreateZswapOutput { coin, recipient } => {
                visit(coin, "coin", visiting) && visit(recipient, "recipient", visiting)
            }
            Expr::Equal { left, right } | Expr::UnsignedSubtract { left, right, .. } => {
                visit(left, "left", visiting) && visit(right, "right", visiting)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                visit(condition, "condition", visiting)
                    && visit(then, "then", visiting)
                    && visit(otherwise, "otherwise", visiting)
            }
            Expr::Let { bindings, body } => {
                bindings.iter().enumerate().all(|(index, binding)| {
                    visit(
                        &binding.value,
                        &format!("bindings[{index}].value"),
                        visiting,
                    )
                }) && visit(body, "body", visiting)
            }
            Expr::Sequence { steps, value } => {
                steps
                    .iter()
                    .enumerate()
                    .all(|(index, step)| visit(step, &format!("steps[{index}]"), visiting))
                    && visit(value, "value", visiting)
            }
            Expr::Call { name, arguments } => {
                if !arguments.iter().enumerate().all(|(index, argument)| {
                    visit(argument, &format!("arguments[{index}]"), visiting)
                }) {
                    false
                } else if pure.contains_key(name.as_str()) {
                    !circuits.contains_key(name.as_str())
                } else if let Some(callee) = circuits.get(name.as_str()) {
                    if domain == CompositeDomain::ResetShieldedPayout
                        && phase_reset::helper_signature(callee)
                    {
                        reset_payout::true_arguments(arguments)
                            && reset_payout::helper_shape(callee, pure)
                    } else if let StateReturn::Expression { value } = &callee.return_value {
                        if !callee.actions.is_empty()
                            && !(domain == CompositeDomain::ActionfulShieldedPayout
                                && shielded_payout::action_shape(&callee.actions, pure) == Some(1))
                        {
                            false
                        } else if visiting.insert(name.clone()) {
                            let valid = visit(
                                value,
                                &format!("callee[{name}].return_value.value"),
                                visiting,
                            );
                            visiting.remove(name);
                            valid
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
    if !valid && failure.is_none() {
        *failure = Some(RecordingGap::expression(value, path.to_owned()));
    }
    valid
}

fn shielded_plan<'a>(
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    domain: CompositeDomain,
) -> Plan<'a> {
    Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: Some(circuits),
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: domain.singleton_bridge(),
            phase_reset: false,
            composite_domain: domain,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    }
}

pub(super) fn lower_shielded_send<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    lower_shielded_send_checked(circuit, ledger, witnesses, pure, circuits).into_option()
}

pub(super) fn lower_shielded_send_checked<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> ProfileAttempt<TypedPlan> {
    let StateReturn::Expression { value } = &circuit.return_value else {
        return ProfileAttempt::NotApplicable;
    };
    if !circuit.actions.is_empty() || !shielded_send_result(&circuit.result) {
        return ProfileAttempt::NotApplicable;
    }
    let rejected_return = || RecordingGap::returned(&circuit.return_value);
    if circuit
        .parameters
        .iter()
        .map(|p| &p.name)
        .collect::<HashSet<_>>()
        .len()
        != circuit.parameters.len()
    {
        return ProfileAttempt::Rejected(rejected_return());
    }
    if let Err(gap) = shielded_send_value_checked(
        value,
        pure,
        circuits,
        &mut HashSet::from([circuit.name.clone()]),
    ) {
        return ProfileAttempt::Rejected(gap);
    }
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::ShieldedSend,
    );
    let scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote! { #name },
                },
            )
        })
        .collect();
    let mut steps = Vec::new();
    let Some(result) = plan.expression(value, &scope, &mut steps) else {
        return ProfileAttempt::Rejected(rejected_return());
    };
    if result.ty == circuit.result
        && plan.evidence.zswap_inputs > 0
        && plan.evidence.zswap_outputs > 0
        && plan.evidence.intent_queries > 0
        && plan.evidence.witness_calls == 0
        && plan.evidence.cell_reads == 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.counter_comparisons == 0
        && plan.evidence.counter_writes == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.set_writes == 0
    {
        ProfileAttempt::Admitted(TypedPlan {
            steps,
            result: result.value,
        })
    } else {
        ProfileAttempt::Rejected(rejected_return())
    }
}

pub(super) fn lower_shielded_merge<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    shielded_merge::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_start_funding<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    start_funding::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_shielded_payout<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    shielded_payout::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_phase_reset<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    phase_reset::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_unit_actions<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    unit_actions::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_composite<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let audit = |intents| {
        composite_circuit(
            circuit,
            witnesses,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
            intents,
        )
    };
    let domain = if audit(false) {
        CompositeDomain::Values
    } else if audit(true) {
        CompositeDomain::Intents
    } else if field_observations::audit(circuit, ledger, pure, circuits) {
        CompositeDomain::FieldObservations
    } else {
        return None;
    };
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: Some(circuits),
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: domain,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote!(#name),
                },
            )
        })
        .collect();
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    let mut steps = Vec::new();
    let result = plan.expression(value, &scope, &mut steps)?;
    (result.ty == circuit.result
        && (domain != CompositeDomain::Intents
            || (plan.evidence.intent_effects > 0
                && plan.evidence.kernel_self_reads + plan.evidence.intent_queries > 0))
        && (domain != CompositeDomain::FieldObservations || plan.evidence.cell_reads > 0))
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let mut assertions = 0;
    let assertion_entry = circuit.actions.is_empty()
        && circuit
            .parameters
            .iter()
            .all(|parameter| parameter.ty == Type::Boolean)
        && (circuit.result == Type::Unit
            || circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }))
        && matches!(&circuit.return_value, StateReturn::Expression { value } if assertion_value(value, witnesses, &mut assertions))
        && assertions > 0;
    let enum_entry = matches!(circuit.parameters.as_slice(), [parameter] if matches!(parameter.ty, Type::Enum { .. }));
    let opaque_entry =
        matches!(circuit.parameters.as_slice(), [parameter] if parameter.ty == Type::OpaqueString);
    let spend_entry = matches!(circuit.parameters.as_slice(), [destination, coin] if matches!(destination.ty, Type::Struct { .. }) && matches!(coin.ty, Type::Struct { .. }));
    let counter_entry = circuit.parameters.iter().all(|parameter| {
        parameter.ty == Type::Boolean
            || parameter.ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                })
    });
    // A root Field read/modify/write may return an independent Field input.
    // Keep this admission structural: one root binding establishes the only
    // eligible Cell slot and the final value names the declared input.
    let field_cell_slot = match (
        circuit.parameters.as_slice(),
        &circuit.result,
        &circuit.return_value,
        circuit.actions.as_slice(),
    ) {
        (
            [parameter],
            Type::Field,
            StateReturn::Expression {
                value: Expr::Parameter { name },
            },
            [StateAction::Let { bindings, .. }],
        ) if parameter.ty == Type::Field && name == &parameter.name => match bindings.as_slice() {
            [
                LocalBinding {
                    name: binding_name,
                    ty: Type::Field,
                    value: Expr::CellRead { field, index },
                },
            ] if binding_name != &parameter.name => Some((field.clone(), *index)),
            _ => None,
        },
        _ => None,
    };
    if !(circuit.parameters.is_empty()
        || enum_entry
        || opaque_entry
        || spend_entry
        || counter_entry
        || field_cell_slot.is_some()
        || assertion_entry)
        || (!matches!(
            circuit.result,
            Type::Unit | Type::OpaqueString | Type::Boolean
        ) && field_cell_slot.is_none()
            && !assertion_entry)
    {
        return None;
    }
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: Some(circuits),
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: field_cell_slot.clone(),
            effectful_field_cells: false,
            read_only_assertions: assertion_entry,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: circuit.parameters.is_empty() && circuit.result == Type::Unit,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let scope: Scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
            (
                parameter.name.clone(),
                TypedValue {
                    ty: parameter.ty.clone(),
                    value: syn::parse_quote!(#name),
                },
            )
        })
        .collect();
    let mut steps = Vec::new();
    let result = match &circuit.return_value {
        StateReturn::Unit if circuit.result == Type::Unit => {
            for action in &circuit.actions {
                plan.action(action, &scope, &mut steps)?;
            }
            syn::parse_quote!(())
        }
        StateReturn::Expression { value } => {
            let result = plan.return_plan(
                &terminal_returns::adapt(&circuit.actions, value),
                &scope,
                &mut steps,
            )?;
            if result.ty != circuit.result {
                return None;
            }
            result.value
        }
        _ => return None,
    };
    let ordinary = plan.evidence.opaque_cells == 0
        && plan.evidence.historic_roots == 0
        && plan.evidence.historic_writes == 0
        && plan.evidence.qualified_cell_writes == 0;
    let membership = ordinary
        && circuit.result == Type::Unit
        && plan.evidence.root_observations > 0
        && plan.evidence.set_writes > 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.optional_cells == 0
        && (plan.evidence.counter_reads == 0
            || (plan.evidence.scalar_helper_calls > 0
                && plan.evidence.counter_reads == plan.evidence.scalar_counter_reads
                && plan.evidence.scalar_counter_reads == plan.evidence.scalar_helper_calls
                && plan.evidence.kernel_self_reads == 0
                && plan.evidence.counter_comparisons == 0))
        && if enum_entry {
            plan.evidence.tree_writes > 0 && plan.evidence.counter_writes == 0
        } else {
            circuit.parameters.is_empty()
                && plan.evidence.counter_writes > 0
                && plan.evidence.tree_writes == 0
        };
    let cell_lifecycle = ordinary
        && plan.evidence.root_observations == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.cell_writes > 0
        && plan.evidence.optional_cells > 0
        && plan.evidence.counter_reads > 0
        && ((opaque_entry && circuit.result == Type::Unit && plan.evidence.counter_writes == 0)
            || (circuit.parameters.is_empty()
                && circuit.result == Type::OpaqueString
                && plan.evidence.counter_writes > 0));
    let historic_spend = spend_entry
        && circuit.result == Type::Unit
        && plan.evidence.qualified_cell_writes == 0
        && plan.evidence.root_observations == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.optional_cells == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.counter_writes == 0
        && plan.evidence.historic_roots > 0
        && plan.evidence.historic_writes > 0
        && plan.evidence.set_writes > 0
        && plan.evidence.opaque_cells > 0
        && plan.evidence.cell_writes == plan.evidence.opaque_cells;
    let counter_comparison = counter_entry
        && plan.evidence.witness_calls == 0
        && ordinary
        && matches!(circuit.result, Type::Boolean | Type::Unit)
        && plan.evidence.counter_comparisons > 0
        && plan.evidence.counter_writes == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.root_observations == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.optional_cells == 0;
    let qualified_set_lifecycle = circuit.result == Type::Unit
        && plan.evidence.qualified_set_writes > 0
        && plan.evidence.qualified_cell_writes == 0
        && plan.evidence.root_observations == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.counter_writes == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.historic_roots == 0
        && plan.evidence.historic_writes == 0
        && ((circuit.parameters.is_empty() && plan.evidence.qualified_set_reads > 0)
            || matches!(circuit.parameters.as_slice(), [coin, recipient]
                if coin.ty == shielded_coin_type()
                    && recipient.ty == shielded_recipient_type()));
    let qualified_cell_replacement = circuit.result == Type::Unit
        && matches!(circuit.parameters.as_slice(), [coin, recipient]
            if coin.ty == shielded_coin_type()
                && recipient.ty == shielded_recipient_type())
        && plan.evidence.qualified_cell_writes == 1
        && plan.evidence.qualified_set_reads == 0
        && plan.evidence.qualified_set_writes == 0
        && plan.evidence.root_observations == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.counter_writes == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.historic_roots == 0
        && plan.evidence.historic_writes == 0;
    let field_cell_root = field_cell_slot.is_some()
        && plan.evidence.witness_calls == 0
        && ordinary
        && plan.evidence.qualified_set_reads == 0
        && plan.evidence.qualified_set_writes == 0
        && plan.evidence.qualified_cell_writes == 0
        && plan.evidence.cell_reads == 1
        && plan.evidence.cell_writes == 1
        && plan.evidence.field_cell_writes == 1
        && plan.evidence.root_observations == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.counter_comparisons == 0
        && plan.evidence.counter_writes == 0
        && plan.evidence.optional_cells == 0;
    (if plan.evidence.scalar_helper_calls > 0 {
        membership
    } else {
        membership
            || cell_lifecycle
            || historic_spend
            || counter_comparison
            || qualified_set_lifecycle
            || qualified_cell_replacement
            || field_cell_root
            || assertion_entry
    })
    .then_some(TypedPlan { steps, result })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod shielded_receive_tests;

#[cfg(test)]
mod shielded_send_tests;

pub(super) fn lower_immediate_shielded_send<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    immediate_send::lower(circuit, ledger, witnesses, pure, circuits)
}

/// Guarded receive, optional historical merge and typed Cell update policy.
pub(super) fn lower_guarded_deposit<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    guarded_deposit::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_funded_mint<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    funded_mint::lower(circuit, ledger, witnesses, pure, circuits)
}

/// Historical payout followed by a closed literal-true reset helper.
pub(super) fn lower_reset_payout<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    reset_payout::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_voting_commit<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    voting_commit::lower(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_unit_composition_checked<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> ProfileAttempt<TypedPlan, composition::CompositionRejection> {
    composition::lower_checked(circuit, ledger, witnesses, pure, circuits)
}

pub(super) fn lower_jubjub_values<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    jubjub_values::lower(circuit, ledger, witnesses, pure, circuits)
}
