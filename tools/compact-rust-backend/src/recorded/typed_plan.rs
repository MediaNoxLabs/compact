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
use crate::coerce_expression;
use crate::ir::{KernelClaimKind, ReturnPlan};
mod field_observations;
mod funded_mint;
mod guarded_deposit;
mod immediate_send;
mod phase_reset;
mod reset_payout;
mod shielded_merge;
mod shielded_payout;
mod terminal_returns;
mod unit_actions;

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
    FieldObservations,
    TerminalReturns,
}
impl CompositeDomain {
    fn shielded_send(self) -> bool {
        matches!(
            self,
            Self::ShieldedSend
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
            Self::ShieldedMerge(_) | Self::GuardedShieldedDeposit | Self::FundedShieldedMint
        )
    }

    fn singleton_bridge(self) -> bool {
        matches!(
            self,
            Self::ImmediateShieldedSend
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
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
                | Self::ImmediateShieldedSend
                | Self::ShieldedPayout
                | Self::ActionfulShieldedPayout
                | Self::ResetShieldedPayout
                | Self::ShieldedMerge(_)
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
        )
    }
    fn intents(self) -> bool {
        matches!(
            self,
            Self::Intents
                | Self::ShieldedReceive
                | Self::ShieldedSend
                | Self::ImmediateShieldedSend
                | Self::ShieldedPayout
                | Self::ActionfulShieldedPayout
                | Self::ResetShieldedPayout
                | Self::ShieldedMerge(_)
                | Self::GuardedShieldedDeposit
                | Self::FundedShieldedMint
        )
    }
}

struct Plan<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    next: usize,
    witness_calls: usize,
    kernel_self_reads: usize,
    context_query: bool,
    root_observations: usize,
    tree_writes: usize,
    set_writes: usize,
    counter_writes: usize,
    counter_reads: usize,
    counter_comparisons: usize,
    cell_reads: usize,
    cell_writes: usize,
    field_cell_writes: usize,
    field_cell_slot: Option<(String, u8)>,
    effectful_field_cells: bool,
    read_only_assertions: bool,
    unit_actions: bool,
    phase_reset: bool,
    composite_domain: CompositeDomain,
    intent_effects: usize,
    intent_queries: usize,
    zswap_inputs: usize,
    zswap_outputs: usize,
    counter_hash_helpers: bool,
    scalar_arguments: bool,
    scalar_body_depth: usize,
    scalar_helper_calls: usize,
    scalar_counter_reads: usize,
    active_calls: HashSet<String>,
    stateful_circuits: Option<&'a HashMap<&'a str, &'a StatefulCircuit>>,
    optional_cells: usize,
    opaque_cells: usize,
    historic_roots: usize,
    historic_writes: usize,
    qualified_set_reads: usize,
    qualified_set_writes: usize,
    qualified_cell_writes: usize,
}

impl Plan<'_> {
    fn fresh(&mut self) -> syn::Ident {
        let id = syn::Ident::new(&format!("__compact_plan_{}", self.next), Span::call_site());
        self.next += 1;
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
        let Some(callee) = self.pure.get(name) else {
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
                expression_with_calls(&callee.body, &parameters, self.pure),
                Ok((_, ty)) if ty == callee.result
            );
        visiting.remove(name);
        supported
    }

    fn pure_value(&self, value: &Expr, visiting: &mut HashSet<String>) -> bool {
        match value {
            Expr::UnsignedLiteral { value, max }
                if self.composite_domain.singleton_bridge()
                    && value == "0"
                    && max == &u64::MAX.to_string() =>
            {
                true
            }
            Expr::FieldCast { value } | Expr::UpgradeFromTransient { value }
                if self.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.pure_value(value, visiting)
            }
            Expr::PersistentCommit { value, opening }
                if self.composite_domain == CompositeDomain::FundedShieldedMint =>
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
        let declaration = *self.ledger.get(field)?;
        (declaration.index == index && declaration.physical_path().len() == 1)
            .then_some(declaration)
    }

    fn qualified_cell_field(&self, field: &str, index: u8) -> Option<&LedgerField> {
        let declaration = *self.ledger.get(field)?;
        let path = declaration.physical_path();
        (declaration.index == index
            && path.first() == Some(&index)
            && (1..=2).contains(&path.len()))
        .then_some(declaration)
    }

    fn expression(
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
                    || self.composite_domain.shielded_helpers() =>
            {
                Some(TypedValue {
                    ty: Type::Unit,
                    value: syn::parse_quote!(()),
                })
            }
            Expr::Sequence {
                steps: expressions,
                value,
            } if self.read_only_assertions
                || self.unit_actions
                || self.composite_domain.shielded_helpers() =>
            {
                for expression in expressions {
                    if self.expression(expression, scope, steps)?.ty != Type::Unit {
                        return None;
                    }
                }
                self.expression(value, scope, steps)
            }
            Expr::Assert { condition, message }
                if self.read_only_assertions
                    || self.unit_actions
                    || matches!(
                        self.composite_domain,
                        CompositeDomain::ShieldedPayout
                            | CompositeDomain::ActionfulShieldedPayout
                            | CompositeDomain::ResetShieldedPayout
                    )
                    || self.composite_domain.shielded_merge() =>
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
            Expr::KernelMintShielded { domain, amount }
                if self.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.funded_mint(expression, domain, amount, scope, steps)
            }
            Expr::CreateZswapInput { .. }
            | Expr::CreateZswapOutput { .. }
            | Expr::KernelClaim { .. }
                if self.composite_domain.intents() =>
            {
                let operands: Vec<&Expr> = match expression {
                    Expr::CreateZswapInput { coin } => vec![coin],
                    Expr::CreateZswapOutput { coin, recipient } => vec![coin, recipient],
                    Expr::KernelClaim { value, .. } => vec![value],
                    _ => unreachable!(),
                };
                let operands = operands
                    .into_iter()
                    .map(|operand| {
                        let value = self.expression(operand, scope, steps)?;
                        Some((value.ty, value.value))
                    })
                    .collect::<Option<Vec<_>>>()?;
                let effect = intent_effect::emit(expression, &operands)?;
                self.intent_effects += usize::from(effect.intent);
                self.intent_queries += usize::from(effect.public_query);
                self.zswap_inputs +=
                    usize::from(matches!(expression, Expr::CreateZswapInput { .. }));
                self.zswap_outputs +=
                    usize::from(matches!(expression, Expr::CreateZswapOutput { .. }));
                steps.push(effect.statement);
                self.bind(syn::parse_quote!(()), Type::Unit, steps)
            }
            Expr::KernelSelf { ty } if *ty == contract_address_type() => {
                self.kernel_self_reads += 1;
                let rust_ty = rust_type(ty).ok()?;
                let observed = self.fresh();
                steps.push(syn::parse_quote!(let (frame, #observed) = frame.kernel_self()?;));
                self.bind(
                    syn::parse_quote!(#rust_ty {
                        bytes: runtime::ledger::contract_address_bytes(&#observed)
                    }),
                    ty.clone(),
                    steps,
                )
            }
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
            Expr::Let { bindings, body } => {
                let scoped = self.bindings(bindings, scope, steps)?;
                self.expression(body, &scoped, steps)
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
                    || (!self.composite_domain.values()
                        && !(self.unit_actions && then.ty == Type::Unit)
                        && then.ty != Type::Boolean
                        && !(self.scalar_arguments && then.ty == (Type::Bytes { length: 32 })))
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
            Expr::WitnessCall { name, arguments } => {
                let declaration = *self.witnesses.get(name.as_str())?;
                if self.composite_domain == CompositeDomain::ResetShieldedPayout {
                    return None;
                }
                if self.composite_domain == CompositeDomain::GuardedShieldedDeposit
                    && (declaration.result != (Type::Bytes { length: 32 })
                        || !declaration.parameters.is_empty())
                {
                    return None;
                }
                if self.composite_domain == CompositeDomain::ShieldedPayout
                    && declaration.result != (Type::Bytes { length: 32 })
                {
                    return None;
                }
                if self.composite_domain == CompositeDomain::ActionfulShieldedPayout
                    && !shielded_payout::witness_type(&declaration.result)
                {
                    return None;
                }
                if arguments.len() != declaration.parameters.len()
                    || !(matches!(declaration.result, Type::Unit | Type::OpaqueBytes)
                        || recordable_cell_type(&declaration.result))
                {
                    return None;
                }
                let args = self.arguments(arguments, &declaration.parameters, scope, steps)?;
                let method = ident(name).ok()?;
                self.witness_calls += 1;
                let observed = self.fresh();
                let ty = rust_type(&declaration.result).ok()?;
                steps.push(syn::parse_quote! {
                    let (frame, #observed): (_, #ty) = frame.try_witness_metered(|context, meter| {
                        witnesses.#method(context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(), meter,
                        }), #(#args),*)
                    })?;
                });
                Some(TypedValue {
                    ty: declaration.result.clone(),
                    value: syn::parse_quote!(#observed),
                })
            }
            Expr::NativeWitnessCall {
                builtin: builtin @ crate::ir::NativeWitnessBuiltin::OwnPublicKey,
            } if matches!(
                self.composite_domain,
                CompositeDomain::ShieldedPayout
                    | CompositeDomain::ActionfulShieldedPayout
                    | CompositeDomain::FundedShieldedMint
                    | CompositeDomain::ResetShieldedPayout
            ) =>
            {
                let ty = builtin.result_type();
                let rust_ty = rust_type(&ty).ok()?;
                let observed = self.fresh();
                steps.push(
                    syn::parse_quote!(let (frame, #observed) = frame.own_coin_public_key()?;),
                );
                self.bind(
                    syn::parse_quote!(#rust_ty { bytes: runtime::FixedBytes::new(#observed) }),
                    ty,
                    steps,
                )
            }
            Expr::Tuple { elements }
                if self.context_query
                    || self.scalar_body_depth > 0
                    || self.unit_actions
                    || self.composite_domain.shielded_helpers() =>
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
                            if self.unit_actions { 2 } else { 3 }
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
            Expr::Call { name, arguments } => self.call(name, arguments, scope, steps),
            Expr::CounterLessThan {
                field,
                index,
                threshold,
            } => {
                if self.field(field, *index)?.declaration != LedgerFieldKind::Counter {
                    return None;
                }
                let threshold = self.expression(threshold, scope, steps)?;
                if threshold.ty
                    != (Type::Unsigned {
                        max: u64::MAX.to_string(),
                    })
                {
                    return None;
                }
                let threshold = threshold.value;
                let slot = ident(field).ok()?;
                let observed = self.fresh();
                steps.push(syn::parse_quote!(let (frame, #observed) = crate::ledger_slots::#slot.record_less_than(frame, (#threshold).value() as u64)?;));
                self.counter_comparisons += 1;
                self.bind(syn::parse_quote!(#observed), Type::Boolean, steps)
            }
            Expr::CounterRead { field, index } => {
                if self.field(field, *index)?.declaration != LedgerFieldKind::Counter {
                    return None;
                }
                let slot = ident(field).ok()?;
                let observed = self.fresh();
                steps.push(syn::parse_quote!(let (frame, #observed) = crate::ledger_slots::#slot.record_read(frame)?;));
                self.counter_reads += 1;
                if self.scalar_body_depth > 0 {
                    self.scalar_counter_reads += 1;
                }
                self.bind(syn::parse_quote!(runtime::BoundedUint::<18446744073709551615>::new(#observed as u128)?), Type::Unsigned { max: "18446744073709551615".into() }, steps)
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
            Expr::CellRead { field, index } => {
                let declaration = self.field(field, *index)?;
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return None;
                };
                if self.composite_domain == CompositeDomain::ShieldedPayout
                    && !shielded_payout::cell_type(ty)
                {
                    return None;
                }
                if self.composite_domain == CompositeDomain::ActionfulShieldedPayout
                    && !shielded_payout::actionful_cell_type(ty)
                {
                    return None;
                }
                if self.effectful_field_cells && *ty != Type::Field {
                    return None;
                }
                if self.read_only_assertions && *ty != Type::Boolean {
                    return None;
                }
                if matches!(
                    self.composite_domain,
                    CompositeDomain::GuardedShieldedDeposit | CompositeDomain::FundedShieldedMint
                ) && !guarded_deposit::read_type(ty)
                {
                    return None;
                }
                if self.composite_domain == CompositeDomain::ResetShieldedPayout
                    && !reset_payout::read_type(ty)
                {
                    return None;
                }
                if !cell_type(ty)
                    && !(matches!(
                        self.composite_domain,
                        CompositeDomain::GuardedShieldedDeposit
                            | CompositeDomain::FundedShieldedMint
                    ) && guarded_deposit::read_type(ty))
                    && !(self.composite_domain == CompositeDomain::ResetShieldedPayout
                        && reset_payout::read_type(ty))
                    && !(self.composite_domain == CompositeDomain::ShieldedPayout
                        && shielded_payout::cell_type(ty))
                    && !(self.composite_domain == CompositeDomain::ActionfulShieldedPayout
                        && shielded_payout::actionful_cell_type(ty))
                    && !(self.unit_actions && unit_actions::value_type(ty))
                    && !(self.phase_reset && phase_reset::cell_type(ty))
                    && !(self.read_only_assertions && *ty == Type::Boolean)
                    && !(ty == &Type::Field
                        && (self.effectful_field_cells
                            || self.composite_domain == CompositeDomain::FieldObservations
                            || self.field_cell_slot.as_ref() == Some(&(field.clone(), *index))))
                {
                    return None;
                }
                let ty = ty.clone();
                self.cell_reads += 1;
                if optional_string(&ty) {
                    self.optional_cells += 1;
                }
                self.observe(field, "record_read", vec![], ty, steps)
            }
            Expr::SetMember {
                field,
                index,
                value,
            } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                let ty = ty.clone();
                if !bytes32_key(&ty) && ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                if ty == crate::stateful::qualified_coin_type() {
                    self.qualified_set_reads += 1;
                }
                let value = self.expression(value, scope, steps)?;
                if value.ty != ty {
                    return None;
                }
                self.observe(
                    field,
                    "record_member",
                    vec![value.value],
                    Type::Boolean,
                    steps,
                )
            }
            Expr::SetSize { field, index } | Expr::SetIsEmpty { field, index } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                if *ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                self.qualified_set_reads += 1;
                if matches!(expression, Expr::SetSize { .. }) {
                    let slot = ident(field).ok()?;
                    let name = self.fresh();
                    steps.push(syn::parse_quote!(let (frame, #name) = crate::ledger_slots::#slot.record_size(frame)?;));
                    self.bind(syn::parse_quote!(runtime::BoundedUint::<18446744073709551615>::new(#name as u128)?), Type::Unsigned { max: u64::MAX.to_string() }, steps)
                } else {
                    self.observe(field, "record_is_empty", vec![], Type::Boolean, steps)
                }
            }
            Expr::HistoricMerkleCheckRoot { field, index, root } => {
                if !matches!(&self.field(field, *index)?.declaration, LedgerFieldKind::HistoricMerkleTree { ty, .. } if wrapped_bytes32(ty))
                {
                    return None;
                }
                let root = self.expression(root, scope, steps)?;
                if !matches!(&root.ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == Type::Field))
                {
                    return None;
                }
                self.historic_roots += 1;
                self.observe(
                    field,
                    "record_check_root",
                    vec![root.value],
                    Type::Boolean,
                    steps,
                )
            }
            Expr::MerkleCheckRoot { field, index, root } => {
                if !matches!(
                    self.field(field, *index)?.declaration,
                    LedgerFieldKind::MerkleTree {
                        ty: Type::Bytes { length: 32 },
                        ..
                    }
                ) {
                    return None;
                }
                let root = self.expression(root, scope, steps)?;
                if !matches!(&root.ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == Type::Field))
                {
                    return None;
                }
                self.root_observations += 1;
                self.observe(
                    field,
                    "record_check_root",
                    vec![root.value],
                    Type::Boolean,
                    steps,
                )
            }
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
        let pure = self.pure.get(name).copied();
        let stateful = self
            .stateful_circuits
            .and_then(|map| map.get(name))
            .copied();
        match (pure, stateful) {
            (Some(callee), None) => {
                if self.composite_domain.values() && !self.composite_domain.shielded_helpers() {
                    return None;
                } // preserve ADR0187 admission
                if self.context_query || (self.unit_actions && !self.composite_domain.intents()) {
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
                if self.composite_domain == CompositeDomain::ResetShieldedPayout
                    && phase_reset::helper_signature(callee)
                {
                    if !reset_payout::true_arguments(arguments)
                        || !reset_payout::helper_shape(callee, self.pure)
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
                if self.composite_domain.shielded_helpers() {
                    if self.composite_domain.singleton_bridge() && shielded_unit_signature(callee) {
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
                        && !(self.composite_domain == CompositeDomain::ActionfulShieldedPayout
                            && shielded_payout::action_shape(&callee.actions, self.pure) == Some(1))
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
                if self.composite_domain == CompositeDomain::TerminalReturns {
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
                if self.phase_reset {
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
                if self.unit_actions {
                    if !(self.composite_domain.intents() && shielded_unit_signature(callee))
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
                        if self.composite_domain == CompositeDomain::FieldObservations =>
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
                let scalar = self.counter_hash_helpers && scalar_counter_hash(callee, self.ledger);
                if !self.composite_domain.values() && !scalar {
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
        let old_arguments = self.scalar_arguments;
        self.scalar_arguments = scalar;
        let arguments = self.arguments(arguments, parameters, scope, steps);
        self.scalar_arguments = old_arguments;
        let arguments = arguments?;
        if !self.active_calls.insert(name.to_owned()) {
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
            self.scalar_body_depth += 1;
        }
        let value = if self.composite_domain == CompositeDomain::TerminalReturns
            || (self.composite_domain == CompositeDomain::ActionfulShieldedPayout
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
            self.scalar_body_depth -= 1;
            self.scalar_helper_calls += 1;
        }
        self.active_calls.remove(name);
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
                let LedgerFieldKind::Cell { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                if self.effectful_field_cells && *ty != Type::Field {
                    return None;
                }
                if self.composite_domain == CompositeDomain::FundedShieldedMint
                    && *ty != Type::Boolean
                {
                    return None;
                }
                if self.composite_domain == CompositeDomain::GuardedShieldedDeposit
                    && !guarded_deposit::write_type(ty)
                {
                    return None;
                }
                if self.composite_domain == CompositeDomain::ResetShieldedPayout
                    && !reset_payout::write_type(ty)
                {
                    return None;
                }
                if !cell_type(ty)
                    && !(self.composite_domain == CompositeDomain::ResetShieldedPayout
                        && reset_payout::write_type(ty))
                    && !(self.composite_domain == CompositeDomain::GuardedShieldedDeposit
                        && guarded_deposit::write_type(ty))
                    && !(self.unit_actions && unit_actions::value_type(ty))
                    && !(self.phase_reset && phase_reset::cell_type(ty))
                    && !(ty == &Type::Field
                        && (self.effectful_field_cells
                            || self.field_cell_slot.as_ref() == Some(&(field.clone(), *index))))
                {
                    return None;
                }
                let ty = ty.clone();
                let value = self.expression(value, scope, steps)?;
                if value.ty != ty {
                    return None;
                }
                let value = value.value;
                let slot = ident(field).ok()?;
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_write(frame, #value)?;));
                self.cell_writes += 1;
                if ty == Type::Field {
                    self.field_cell_writes += 1;
                }
                if ty == Type::OpaqueBytes {
                    self.opaque_cells += 1;
                }
                if optional_string(&ty) {
                    self.optional_cells += 1;
                }
            }
            StateAction::CellWriteCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                let LedgerFieldKind::Cell { ty } =
                    &self.qualified_cell_field(field, *index)?.declaration
                else {
                    return None;
                };
                if *ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                let coin = self.expression(coin, scope, steps)?;
                let recipient = self.expression(recipient, scope, steps)?;
                if coin.ty != crate::stateful::shielded_coin_type()
                    || recipient.ty != crate::stateful::shielded_recipient_type()
                {
                    return None;
                }
                let slot = ident(field).ok()?;
                let coin = coin.value;
                let recipient = recipient.value;
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_write_coin(
                    frame,
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
                    runtime::ledger::coin_recipient_from_compact(
                        #recipient.is_left, #recipient.left.bytes, #recipient.right.bytes,
                    ),
                )?;));
                self.qualified_cell_writes += 1;
            }
            StateAction::CounterReset { field, index }
                if self.phase_reset
                    || self.composite_domain == CompositeDomain::ResetShieldedPayout =>
            {
                if self.field(field, *index)?.declaration != LedgerFieldKind::Counter {
                    return None;
                }
                let slot = ident(field).ok()?;
                steps.push(
                    syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;),
                );
                self.counter_writes += 1;
            }
            StateAction::MerkleResetToDefault { field, index }
                if self.phase_reset
                    || self.composite_domain == CompositeDomain::ResetShieldedPayout =>
            {
                if !matches!(
                    self.field(field, *index)?.declaration,
                    LedgerFieldKind::MerkleTree {
                        depth: 10,
                        ty: Type::Bytes { length: 32 }
                    }
                ) {
                    return None;
                }
                let slot = ident(field).ok()?;
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset_to_default(frame)?;));
                self.tree_writes += 1;
            }
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            } => {
                if self.field(field, *index)?.declaration != LedgerFieldKind::Counter {
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
                self.counter_writes += 1;
            }
            StateAction::Assert { condition, message } => {
                let condition = self.expression(condition, scope, steps)?;
                if condition.ty != Type::Boolean {
                    return None;
                }
                let condition = condition.value;
                steps.push(syn::parse_quote! {
                    if !#condition { return Err(runtime::CompactError::AssertionFailed(#message.to_owned())); }
                });
            }
            StateAction::Expression {
                value: value @ Expr::WitnessCall { .. },
            } => {
                if self.expression(value, scope, steps)?.ty != Type::Unit {
                    return None;
                }
            }
            StateAction::Expression { value }
                if self.unit_actions && self.composite_domain.intents() =>
            {
                if self.expression(value, scope, steps)?.ty != Type::Unit {
                    return None;
                }
            }
            StateAction::CircuitCall { name, arguments }
                if (self.unit_actions && self.composite_domain.intents()) || self.phase_reset =>
            {
                if self.call(name, arguments, scope, steps)?.ty != Type::Unit {
                    return None;
                }
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
                let declaration = self.field(field, *index)?.clone();
                match (action, &declaration.declaration) {
                    (
                        StateAction::MerkleInsert { .. },
                        LedgerFieldKind::MerkleTree {
                            ty: Type::Bytes { length: 32 },
                            ..
                        },
                    ) => self.tree_writes += 1,
                    (StateAction::SetInsert { .. }, LedgerFieldKind::Set { ty })
                        if bytes32_key(ty) =>
                    {
                        self.set_writes += 1
                    }
                    (StateAction::SetInsert { .. }, LedgerFieldKind::Set { ty })
                        if *ty == crate::stateful::qualified_coin_type() =>
                    {
                        self.qualified_set_writes += 1
                    }
                    (
                        StateAction::HistoricMerkleInsert { .. },
                        LedgerFieldKind::HistoricMerkleTree { ty, .. },
                    ) if wrapped_bytes32(ty) => self.historic_writes += 1,
                    _ => return None,
                }
                let ty = match &declaration.declaration {
                    LedgerFieldKind::MerkleTree { ty, .. }
                    | LedgerFieldKind::HistoricMerkleTree { ty, .. }
                    | LedgerFieldKind::Set { ty } => ty.clone(),
                    _ => return None,
                };
                let value = self.expression(value, scope, steps)?;
                if value.ty != ty {
                    return None;
                }
                let slot = ident(field).ok()?;
                let value = value.value;
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_insert(frame, #value)?;));
            }
            StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                if *ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                let ty = ty.clone();
                let value = self.expression(value, scope, steps)?;
                if value.ty != ty {
                    return None;
                }
                let slot = ident(field).ok()?;
                let value = value.value;
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_remove(frame, #value)?;));
                self.qualified_set_writes += 1;
            }
            StateAction::SetReset { field, index } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                if self.composite_domain == CompositeDomain::ResetShieldedPayout
                    && *ty != (Type::Bytes { length: 32 })
                {
                    return None;
                }
                if *ty != crate::stateful::qualified_coin_type()
                    && !((self.phase_reset
                        || self.composite_domain == CompositeDomain::ResetShieldedPayout)
                        && *ty == (Type::Bytes { length: 32 }))
                {
                    return None;
                }
                let slot = ident(field).ok()?;
                steps.push(
                    syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;),
                );
                if *ty == crate::stateful::qualified_coin_type() {
                    self.qualified_set_writes += 1;
                } else {
                    self.set_writes += 1;
                }
            }
            StateAction::SetInsertCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                if *ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                let coin = self.expression(coin, scope, steps)?;
                let recipient = self.expression(recipient, scope, steps)?;
                if coin.ty != crate::stateful::shielded_coin_type()
                    || recipient.ty != crate::stateful::shielded_recipient_type()
                {
                    return None;
                }
                let slot = ident(field).ok()?;
                let coin = coin.value;
                let recipient = recipient.value;
                steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_insert_coin(
                    frame,
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
                    runtime::ledger::coin_recipient_from_compact(
                        #recipient.is_left, #recipient.left.bytes, #recipient.right.bytes,
                    ),
                )?;));
                self.qualified_set_writes += 1;
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

// Bounded admission is separate from the existing single-slot root-Let profile.
fn effectful_value(value: &Expr, witnesses: &HashMap<&str, &WitnessDeclaration>) -> bool {
    match value {
        Expr::Parameter { .. }
        | Expr::FieldLiteral { .. }
        | Expr::Boolean { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::CellRead { .. } => true,
        Expr::Add { left, right } | Expr::Equal { left, right } => {
            effectful_value(left, witnesses) && effectful_value(right, witnesses)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            effectful_value(condition, witnesses)
                && effectful_value(then, witnesses)
                && effectful_value(otherwise, witnesses)
        }
        Expr::Let { bindings, body } => {
            effectful_bindings(bindings, witnesses) && effectful_value(body, witnesses)
        }
        Expr::Coerce {
            value,
            ty: Type::Field | Type::Boolean,
        } => effectful_value(value, witnesses),
        Expr::FieldCast { value } => effectful_value(value, witnesses),
        Expr::WitnessCall { name, arguments } => witnesses.get(name.as_str()).is_some_and(|w| {
            w.result == Type::Field
                && w.parameters.iter().all(|p| p.ty == Type::Field)
                && arguments.iter().all(|a| effectful_value(a, witnesses))
        }),
        _ => false,
    }
}
fn effectful_bindings(
    bindings: &[LocalBinding],
    witnesses: &HashMap<&str, &WitnessDeclaration>,
) -> bool {
    bindings.iter().all(|b| {
        (matches!(b.ty, Type::Field | Type::Boolean)
            || matches!(
                (&b.ty, &b.value),
                (Type::Unsigned { .. }, Expr::UnsignedLiteral { .. })
            ))
            && effectful_value(&b.value, witnesses)
    })
}
fn effectful_action(action: &StateAction, witnesses: &HashMap<&str, &WitnessDeclaration>) -> bool {
    match action {
        StateAction::CellWrite { value, .. } => effectful_value(value, witnesses),
        StateAction::Sequence { actions } => actions.iter().all(|a| effectful_action(a, witnesses)),
        StateAction::Let { bindings, action } => {
            effectful_bindings(bindings, witnesses) && effectful_action(action, witnesses)
        }
        _ => false,
    }
}
fn effectful_body(
    body: &ReturnPlan,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    branches: &mut usize,
) -> bool {
    match body {
        ReturnPlan::Value { value } => effectful_value(value, witnesses),
        ReturnPlan::Let { bindings, result } => {
            effectful_bindings(bindings, witnesses) && effectful_body(result, witnesses, branches)
        }
        ReturnPlan::Sequence { actions, result } => {
            actions.iter().all(|a| effectful_action(a, witnesses))
                && effectful_body(result, witnesses, branches)
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            *branches += 1;
            effectful_value(condition, witnesses)
                && effectful_body(then, witnesses, branches)
                && effectful_body(otherwise, witnesses, branches)
        }
    }
}

pub(super) fn lower_effectful<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
) -> Option<TypedPlan> {
    let StateReturn::Effectful { body } = &circuit.return_value else {
        return None;
    };
    let mut branches = 0;
    if !circuit.actions.is_empty()
        || circuit.result != Type::Field
        || !circuit.parameters.iter().all(|p| p.ty == Type::Field)
        || !effectful_body(body, witnesses, &mut branches)
        || branches == 0
    {
        return None;
    }
    let mut plan = Plan {
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: true,
        read_only_assertions: false,
        unit_actions: false,
        phase_reset: false,
        composite_domain: CompositeDomain::None,
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: None,
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
    };
    let scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{i}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote!(#name),
                },
            )
        })
        .collect();
    let mut steps = Vec::new();
    let result = plan.return_plan(body, &scope, &mut steps)?;
    if result.ty != circuit.result
        || plan.cell_reads == 0
        || plan.cell_writes == 0
        || plan.cell_writes != plan.field_cell_writes
        || plan.optional_cells != 0
        || plan.opaque_cells != 0
    {
        return None;
    }
    Some(TypedPlan {
        steps,
        result: result.value,
    })
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
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: true,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: false,
        read_only_assertions: false,
        unit_actions: false,
        phase_reset: false,
        composite_domain: CompositeDomain::None,
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: None,
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
    };
    let mut steps = Vec::new();
    let result = plan.expression(value, &Scope::new(), &mut steps)?;
    (result.ty == circuit.result && plan.kernel_self_reads > 0 && plan.witness_calls == 0)
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
                    && (p.ty == crate::stateful::shielded_coin_type()
                        || p.ty == crate::stateful::qualified_coin_type()
                        || p.ty == crate::stateful::shielded_recipient_type()
                        || p.ty == (Type::Bytes { length: 32 })))
        })
        && matches!(&circuit.return_value, StateReturn::Expression { value } if composite_value(value,witnesses,circuits,visiting,intents))
}

fn shielded_unit_signature(circuit: &StatefulCircuit) -> bool {
    circuit.result == Type::Unit
        && circuit.return_value == StateReturn::Unit
        && matches!(circuit.parameters.as_slice(), [parameter] if parameter.ty == crate::stateful::shielded_coin_type())
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
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: false,
        read_only_assertions: false,
        unit_actions: true,
        phase_reset: false,
        composite_domain: CompositeDomain::ShieldedReceive,
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: Some(circuits),
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
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
    (plan.kernel_self_reads == 1
        && plan.intent_effects == 1
        && plan.intent_queries == 1
        && plan.witness_calls == 0
        && plan.cell_reads == 0
        && plan.cell_writes == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0
        && plan.counter_reads == 0
        && plan.counter_writes == 0)
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
    if change.name != "change"
        || sent.name != "sent"
        || sent.ty != crate::stateful::shielded_coin_type()
    {
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
    shielded_value(
        value,
        pure,
        circuits,
        visiting,
        CompositeDomain::ShieldedSend,
    )
}

fn shielded_value(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    domain: CompositeDomain,
) -> bool {
    let visit = |part: &Expr, visiting: &mut HashSet<String>| {
        shielded_value(part, pure, circuits, visiting, domain)
    };
    match value {
        Expr::NativeWitnessCall {
            builtin: crate::ir::NativeWitnessBuiltin::OwnPublicKey,
        } if domain == CompositeDomain::FundedShieldedMint => true,
        Expr::KernelMintShielded {
            domain: token_domain,
            amount,
        } if domain == CompositeDomain::FundedShieldedMint => {
            visit(token_domain, visiting) && visit(amount, visiting)
        }
        Expr::CounterRead { .. } if domain == CompositeDomain::ResetShieldedPayout => true,
        Expr::CounterLessThan { threshold, .. }
            if domain == CompositeDomain::ResetShieldedPayout =>
        {
            visit(threshold, visiting)
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
            arguments.iter().all(|value| visit(value, visiting))
        }
        Expr::Assert { condition, .. }
            if matches!(
                domain,
                CompositeDomain::ShieldedPayout
                    | CompositeDomain::ActionfulShieldedPayout
                    | CompositeDomain::ResetShieldedPayout
            ) || domain.shielded_merge() =>
        {
            visit(condition, visiting)
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
        Expr::StructLiteral { fields, .. } | Expr::Tuple { elements: fields } => {
            fields.iter().all(|part| visit(part, visiting))
        }
        Expr::StructField { value, .. }
        | Expr::Coerce { value, .. }
        | Expr::UnsignedCast { value, .. }
        | Expr::DegradeToTransient { value }
        | Expr::TransientHash { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::CreateZswapInput { coin: value }
        | Expr::KernelClaim { value, .. } => visit(value, visiting),
        Expr::UnsignedAdd { left, right, .. } if domain.shielded_merge() => {
            visit(left, visiting) && visit(right, visiting)
        }
        Expr::CreateZswapOutput { coin, recipient }
        | Expr::Equal {
            left: coin,
            right: recipient,
        }
        | Expr::UnsignedSubtract {
            left: coin,
            right: recipient,
            ..
        } => visit(coin, visiting) && visit(recipient, visiting),
        Expr::If {
            condition,
            then,
            otherwise,
        } => visit(condition, visiting) && visit(then, visiting) && visit(otherwise, visiting),
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .all(|binding| visit(&binding.value, visiting))
                && visit(body, visiting)
        }
        Expr::Sequence { steps, value } => {
            steps.iter().all(|step| visit(step, visiting)) && visit(value, visiting)
        }
        Expr::Call { name, arguments } => {
            if !arguments.iter().all(|argument| visit(argument, visiting)) {
                return false;
            }
            if pure.contains_key(name.as_str()) {
                return !circuits.contains_key(name.as_str());
            }
            let Some(callee) = circuits.get(name.as_str()) else {
                return false;
            };
            if domain == CompositeDomain::ResetShieldedPayout
                && phase_reset::helper_signature(callee)
            {
                return reset_payout::true_arguments(arguments)
                    && reset_payout::helper_shape(callee, pure);
            }
            let StateReturn::Expression { value } = &callee.return_value else {
                return false;
            };
            if !callee.actions.is_empty()
                && !(domain == CompositeDomain::ActionfulShieldedPayout
                    && shielded_payout::action_shape(&callee.actions, pure) == Some(1))
            {
                return false;
            }
            if !visiting.insert(name.clone()) {
                return false;
            }
            let valid = visit(value, visiting);
            visiting.remove(name);
            valid
        }
        _ => false,
    }
}

fn shielded_plan<'a>(
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    domain: CompositeDomain,
) -> Plan<'a> {
    Plan {
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: false,
        read_only_assertions: false,
        unit_actions: domain.singleton_bridge(),
        phase_reset: false,
        composite_domain: domain,
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: Some(circuits),
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
    }
}

pub(super) fn lower_shielded_send<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    if !circuit.actions.is_empty()
        || !shielded_send_result(&circuit.result)
        || circuit
            .parameters
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != circuit.parameters.len()
        || !shielded_send_value(
            value,
            pure,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
        )
    {
        return None;
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
    let result = plan.expression(value, &scope, &mut steps)?;
    (result.ty == circuit.result
        && plan.zswap_inputs > 0
        && plan.zswap_outputs > 0
        && plan.intent_queries > 0
        && plan.witness_calls == 0
        && plan.cell_reads == 0
        && plan.cell_writes == 0
        && plan.counter_reads == 0
        && plan.counter_comparisons == 0
        && plan.counter_writes == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0)
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
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
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: false,
        read_only_assertions: false,
        unit_actions: false,
        phase_reset: false,
        composite_domain: domain,
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: Some(circuits),
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
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
            || (plan.intent_effects > 0 && plan.kernel_self_reads + plan.intent_queries > 0))
        && (domain != CompositeDomain::FieldObservations || plan.cell_reads > 0))
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
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: field_cell_slot.clone(),
        effectful_field_cells: false,
        read_only_assertions: assertion_entry,
        unit_actions: false,
        phase_reset: false,
        composite_domain: CompositeDomain::None,
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: circuit.parameters.is_empty() && circuit.result == Type::Unit,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: Some(circuits),
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
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
    let ordinary = plan.opaque_cells == 0
        && plan.historic_roots == 0
        && plan.historic_writes == 0
        && plan.qualified_cell_writes == 0;
    let membership = ordinary
        && circuit.result == Type::Unit
        && plan.root_observations > 0
        && plan.set_writes > 0
        && plan.cell_writes == 0
        && plan.optional_cells == 0
        && (plan.counter_reads == 0
            || (plan.scalar_helper_calls > 0
                && plan.counter_reads == plan.scalar_counter_reads
                && plan.scalar_counter_reads == plan.scalar_helper_calls
                && plan.kernel_self_reads == 0
                && plan.counter_comparisons == 0))
        && if enum_entry {
            plan.tree_writes > 0 && plan.counter_writes == 0
        } else {
            circuit.parameters.is_empty() && plan.counter_writes > 0 && plan.tree_writes == 0
        };
    let cell_lifecycle = ordinary
        && plan.root_observations == 0
        && plan.set_writes == 0
        && plan.tree_writes == 0
        && plan.cell_writes > 0
        && plan.optional_cells > 0
        && plan.counter_reads > 0
        && ((opaque_entry && circuit.result == Type::Unit && plan.counter_writes == 0)
            || (circuit.parameters.is_empty()
                && circuit.result == Type::OpaqueString
                && plan.counter_writes > 0));
    let historic_spend = spend_entry
        && circuit.result == Type::Unit
        && plan.qualified_cell_writes == 0
        && plan.root_observations == 0
        && plan.tree_writes == 0
        && plan.optional_cells == 0
        && plan.counter_reads == 0
        && plan.counter_writes == 0
        && plan.historic_roots > 0
        && plan.historic_writes > 0
        && plan.set_writes > 0
        && plan.opaque_cells > 0
        && plan.cell_writes == plan.opaque_cells;
    let counter_comparison = counter_entry
        && plan.witness_calls == 0
        && ordinary
        && matches!(circuit.result, Type::Boolean | Type::Unit)
        && plan.counter_comparisons > 0
        && plan.counter_writes == 0
        && plan.tree_writes == 0
        && plan.root_observations == 0
        && plan.set_writes == 0
        && plan.cell_writes == 0
        && plan.optional_cells == 0;
    let qualified_set_lifecycle = circuit.result == Type::Unit
        && plan.qualified_set_writes > 0
        && plan.qualified_cell_writes == 0
        && plan.root_observations == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0
        && plan.counter_writes == 0
        && plan.counter_reads == 0
        && plan.cell_writes == 0
        && plan.historic_roots == 0
        && plan.historic_writes == 0
        && ((circuit.parameters.is_empty() && plan.qualified_set_reads > 0)
            || matches!(circuit.parameters.as_slice(), [coin, recipient]
                if coin.ty == crate::stateful::shielded_coin_type()
                    && recipient.ty == crate::stateful::shielded_recipient_type()));
    let qualified_cell_replacement = circuit.result == Type::Unit
        && matches!(circuit.parameters.as_slice(), [coin, recipient]
            if coin.ty == crate::stateful::shielded_coin_type()
                && recipient.ty == crate::stateful::shielded_recipient_type())
        && plan.qualified_cell_writes == 1
        && plan.qualified_set_reads == 0
        && plan.qualified_set_writes == 0
        && plan.root_observations == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0
        && plan.counter_writes == 0
        && plan.counter_reads == 0
        && plan.cell_writes == 0
        && plan.historic_roots == 0
        && plan.historic_writes == 0;
    let field_cell_root = field_cell_slot.is_some()
        && plan.witness_calls == 0
        && ordinary
        && plan.qualified_set_reads == 0
        && plan.qualified_set_writes == 0
        && plan.qualified_cell_writes == 0
        && plan.cell_reads == 1
        && plan.cell_writes == 1
        && plan.field_cell_writes == 1
        && plan.root_observations == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0
        && plan.counter_reads == 0
        && plan.counter_comparisons == 0
        && plan.counter_writes == 0
        && plan.optional_cells == 0;
    (if plan.scalar_helper_calls > 0 {
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
mod tests {
    use super::*;

    #[test]
    fn membership_scalar_helpers_keep_counter_provenance_and_declaration_scope() {
        fn planned(value: &serde_json::Value) -> Option<TypedPlan> {
            let c: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
            lower(
                &c.stateful_circuits[0],
                &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
                &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
                &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
                &c.stateful_circuits
                    .iter()
                    .map(|s| (s.name.as_str(), s))
                    .collect(),
            )
        }
        use serde_json::json;
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/micro-dao-reveal-schema20-ir.json"
        ))
        .unwrap();
        let positive = planned(&source).unwrap();
        let steps = positive.steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert_eq!(tokens.matches("round . record_read").count(), 2);
        assert_eq!(tokens.matches("runtime :: persistent_hash").count(), 2);
        assert_eq!(tokens.matches("pure_circuits :: path_root").count(), 1);
        let renamed = source
            .to_string()
            .replace("reveal_nullifier", "membership_key")
            .replace("commit_with_sk", "membership_commitment");
        assert!(planned(&serde_json::from_str(&renamed).unwrap()).is_some());
        let rejected = |path: &str, replacement: serde_json::Value| {
            let mut changed = source.clone();
            *changed.pointer_mut(path).unwrap() = replacement;
            assert!(planned(&changed).is_none(), "accepted mutation at {path}");
        };
        rejected("/stateful_circuits/1/result", json!({"kind":"boolean"}));
        rejected("/stateful_circuits/1/parameters", json!([]));
        rejected(
            "/stateful_circuits/1/parameters/0/ty",
            json!({"kind":"field"}),
        );
        rejected(
            "/stateful_circuits/1/actions",
            json!([{"kind":"counter_increment","field":"round","index":6,"amount":{"kind":"literal","value":1}}]),
        );
        rejected(
            "/stateful_circuits/1/return_value/value/value/elements/1/value/value/index",
            json!(5),
        );
        rejected(
            "/stateful_circuits/1/return_value/value/value/elements/1/value/value",
            json!({"kind":"witness_call","name":"local_secret_key","arguments":[]}),
        );
        rejected(
            "/stateful_circuits/1/return_value/value/value/elements/2",
            json!({"kind":"parameter","name":"caller_only"}),
        );
        rejected(
            "/stateful_circuits/1/return_value/value/value/elements",
            json!([]),
        );
        rejected(
            "/stateful_circuits/1/return_value/value",
            json!({"kind":"call","name":"reveal_nullifier","arguments":[{"kind":"parameter","name":"sk"}]}),
        );
        let mut ambiguous = source.clone();
        ambiguous["circuits"].as_array_mut().unwrap().push(json!({"name":"reveal_nullifier","parameters":[],"result":{"kind":"bytes","length":32},"body":{"kind":"bytes_literal","bytes":vec![0;32]}}));
        assert!(planned(&ambiguous).is_none());
        let mut open = source.clone();
        open["stateful_circuits"][0]["actions"].as_array_mut().unwrap().insert(0,json!({"kind":"let","bindings":[{"name":"unrelated_read","ty":{"kind":"unsigned","max":"18446744073709551615"},"value":{"kind":"counter_read","field":"round","index":6}}],"action":{"kind":"sequence","actions":[]}}));
        assert!(
            planned(&open).is_none(),
            "root Counter read must not acquire helper provenance"
        );
        let mut declaration = source.clone();
        let slot = declaration["ledger_fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == "round")
            .unwrap();
        slot["declaration"] =
            json!({"kind":"cell","ty":{"kind":"unsigned","max":"18446744073709551615"}});
        assert!(planned(&declaration).is_none());
        fn mutate_bytes_branch(value: &mut serde_json::Value) -> bool {
            if value["kind"] == "if" && value["then"]["kind"] == "bytes_literal" {
                value["otherwise"] = json!({"kind":"boolean","value":false});
                return true;
            }
            match value {
                serde_json::Value::Object(m) => m.values_mut().any(mutate_bytes_branch),
                serde_json::Value::Array(a) => a.iter_mut().any(mutate_bytes_branch),
                _ => false,
            }
        }
        let mut branch = source.clone();
        assert!(mutate_bytes_branch(&mut branch["stateful_circuits"][0]));
        assert!(planned(&branch).is_none());
    }

    #[test]
    fn context_queries_audit_pure_types_scope_cycles_and_effect_order() {
        fn planned(value: &serde_json::Value) -> Option<TypedPlan> {
            let c: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
            lower_context_query(
                &c.stateful_circuits[0],
                &HashMap::new(),
                &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
                &c.circuits.iter().map(|c| (c.name.as_str(), c)).collect(),
            )
        }
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/token-query-schema20-ir.json")).unwrap();
        let positive = planned(&source).unwrap();
        let steps = positive.steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert_eq!(tokens.matches("kernel_self").count(), 1);
        assert_eq!(tokens.matches("persistent_commit").count(), 1);
        assert!(tokens.find("100u8").unwrap() < tokens.find("kernel_self").unwrap());
        assert!(tokens.find("kernel_self").unwrap() < tokens.find("persistent_commit").unwrap());
        // Renaming is metadata, and the original unrelated witnesses stay retained.
        assert!(!source["witnesses"].as_array().unwrap().is_empty());
        let renamed = source
            .to_string()
            .replace("tokenType", "derive_resource")
            .replace("dao_voting_token", "resource_id");
        assert!(planned(&serde_json::from_str(&renamed).unwrap()).is_some());
        let rejected = |path: &str, replacement: serde_json::Value| {
            let mut changed = source.clone();
            *changed.pointer_mut(path).unwrap() = replacement;
            assert!(planned(&changed).is_none(), "accepted mutation at {path}");
        };
        use serde_json::json;
        rejected("/circuits/0/body/opening/bytes", json!([1, 2]));
        rejected("/circuits/0/body/value/elements", json!([]));
        rejected("/circuits/0/parameters/1/name", json!("domain_sep"));
        rejected(
            "/stateful_circuits/0/return_value/value/arguments/1/value/ty",
            json!({"kind":"bytes","length":32}),
        );
        rejected(
            "/circuits/0/body",
            json!({"kind":"create_zswap_output","coin":{"kind":"parameter","name":"domain_sep"},"recipient":{"kind":"parameter","name":"contractAddress"}}),
        );
        rejected("/circuits/0/result", json!({"kind":"boolean"}));
        rejected(
            "/circuits/0/parameters/0/ty",
            json!({"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}),
        );
        rejected("/circuits/0/parameters", json!([]));
        rejected("/circuits/0/body/value/elements/1/field", json!("wrong"));
        rejected("/circuits/0/body/value/elements/1/index", json!(1));
        rejected(
            "/circuits/0/body",
            json!({"kind":"call","name":"tokenType","arguments":[]}),
        );
        rejected(
            "/circuits/0/body",
            json!({"kind":"parameter","name":"caller_only"}),
        );
        rejected(
            "/circuits/0/body",
            json!({"kind":"witness_call","name":"local_secret_key","arguments":[]}),
        );
        rejected(
            "/circuits/0/body",
            json!({"kind":"cell_read","field":"organizer","index":0}),
        );
        rejected(
            "/circuits/0/body",
            source["stateful_circuits"][0]["return_value"]["value"]["arguments"][1]["value"]
                .clone(),
        );
        rejected(
            "/stateful_circuits/0/parameters",
            json!([{"name":"extra","ty":{"kind":"bytes","length":32}}]),
        );
        rejected(
            "/stateful_circuits/0/actions",
            json!([{"kind":"cell_write","field":"organizer","index":0,"value":{"kind":"bytes_literal","bytes":vec![0;32]}}]),
        );
        rejected(
            "/stateful_circuits/0/return_value/value",
            json!({"kind":"bytes_literal","bytes":vec![0;32]}),
        );
        // Caller locals cannot leak into the callee. Both caller arguments still
        // evaluate once in order, including a query whose result is discarded.
        let mut ordered = source.clone();
        let address = source["circuits"][0]["parameters"][1]["ty"].clone();
        let root = &mut ordered["stateful_circuits"][0]["return_value"]["value"];
        let first = root["arguments"][0].clone();
        root["arguments"][0] = json!({"kind":"let","bindings":[{"name":"caller_only","ty":address.clone(),"value":{"kind":"kernel_self","ty":address}}],"body":first});
        let p = planned(&ordered).unwrap();
        let steps = p.steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        let positions: Vec<_> = tokens
            .match_indices("kernel_self")
            .map(|(n, _)| n)
            .collect();
        assert_eq!(positions.len(), 2);
        assert!(positions[0] < tokens.find("100u8").unwrap());
        assert!(tokens.find("100u8").unwrap() < positions[1]);
        assert!(positions[1] < tokens.find("persistent_commit").unwrap());
        ordered["circuits"][0]["body"] = json!({"kind":"parameter","name":"caller_only"});
        assert!(planned(&ordered).is_none());
    }

    #[test]
    fn composite_helpers_are_typed_scoped_acyclic_and_evaluate_arguments_once() {
        fn planned(contract: &crate::ir::Contract, index: usize) -> Option<TypedPlan> {
            let ledger = contract
                .ledger_fields
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect();
            let witnesses = contract
                .witnesses
                .iter()
                .map(|w| (w.name.as_str(), w))
                .collect();
            let circuits = contract
                .stateful_circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect();
            lower_composite(
                &contract.stateful_circuits[index],
                &ledger,
                &witnesses,
                &HashMap::new(),
                &circuits,
            )
        }
        let source: crate::ir::Contract =
            serde_json::from_str(include_str!("../../tests/stateful-struct-schema20-ir.json"))
                .unwrap();
        for index in 0..3 {
            assert!(planned(&source, index).is_some());
        }
        assert!(planned(&source, 3).is_some());
        let mut extra_effect = source.clone();
        extra_effect.stateful_circuits[0]
            .actions
            .push(StateAction::CellWrite {
                field: "marker".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "1".into() },
            });
        assert!(planned(&extra_effect, 2).is_none());
        let mut cycle = source.clone();
        cycle.stateful_circuits[0].return_value = StateReturn::Expression {
            value: Expr::Call {
                name: "snapshot".into(),
                arguments: vec![Expr::Boolean { value: true }],
            },
        };
        assert!(planned(&cycle, 0).is_none());
        assert!(planned(&cycle, 2).is_none());
        let mut wrong_signature = source.clone();
        wrong_signature.stateful_circuits[0].parameters.clear();
        assert!(planned(&wrong_signature, 2).is_none());
        let mut wrong_result = source.clone();
        wrong_result.stateful_circuits[0].result = contract_address_type();
        assert!(planned(&wrong_result, 2).is_none());
        let mut mismatched_branch = source.clone();
        let StateReturn::Expression {
            value: Expr::If { otherwise, .. },
        } = &mut mismatched_branch.stateful_circuits[0].return_value
        else {
            unreachable!()
        };
        **otherwise = Expr::Default {
            ty: contract_address_type(),
        };
        assert!(planned(&mismatched_branch, 0).is_none());
        let mut leakage = source.clone();
        let caller_value = match leakage.stateful_circuits[2].return_value.clone() {
            StateReturn::Expression { value } => value,
            _ => unreachable!(),
        };
        let snapshot_type = leakage.stateful_circuits[0].result.clone();
        leakage.stateful_circuits[2].return_value = StateReturn::Expression {
            value: Expr::Let {
                bindings: vec![LocalBinding {
                    name: "caller_only".into(),
                    ty: snapshot_type.clone(),
                    value: Expr::Default { ty: snapshot_type },
                }],
                body: Box::new(caller_value),
            },
        };
        leakage.stateful_circuits[0].return_value = StateReturn::Expression {
            value: Expr::Parameter {
                name: "caller_only".into(),
            },
        };
        assert!(planned(&leakage, 2).is_none());

        let mut ordered = source;
        ordered.stateful_circuits[0]
            .parameters
            .push(crate::ir::Parameter {
                name: "unused_second".into(),
                ty: Type::Boolean,
            });
        let StateReturn::Expression {
            value: Expr::StructLiteral { fields, .. },
        } = &mut ordered.stateful_circuits[2].return_value
        else {
            unreachable!()
        };
        let Expr::Call { arguments, .. } = &mut fields[0] else {
            unreachable!()
        };
        *arguments = [71, 72]
            .into_iter()
            .map(|tag| Expr::Let {
                bindings: vec![LocalBinding {
                    name: "caller_argument".into(),
                    ty: Type::Unsigned {
                        max: u64::MAX.to_string(),
                    },
                    value: Expr::WitnessCall {
                        name: "next_value".into(),
                        arguments: vec![Expr::UnsignedLiteral {
                            max: "255".into(),
                            value: tag.to_string(),
                        }],
                    },
                }],
                body: Box::new(Expr::Boolean { value: true }),
            })
            .collect();
        let plan = planned(&ordered, 2).unwrap();
        let steps = plan.steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert_eq!(tokens.matches("71u128").count(), 1);
        assert_eq!(tokens.matches("72u128").count(), 1);
        assert_eq!(tokens.matches("next_value").count(), 5);
        assert!(tokens.find("71u128").unwrap() < tokens.find("72u128").unwrap());
        assert!(tokens.find("72u128").unwrap() < tokens.find("(1u128)").unwrap());
    }

    #[test]
    fn local_coercion_uses_its_actual_type_before_a_helper_boundary() {
        let ledger = HashMap::new();
        let witnesses = HashMap::new();
        let pure = HashMap::new();
        let mut plan = Plan {
            ledger: &ledger,
            witnesses: &witnesses,
            pure: &pure,
            next: 0,
            witness_calls: 0,
            kernel_self_reads: 0,
            context_query: false,
            root_observations: 0,
            tree_writes: 0,
            set_writes: 0,
            counter_writes: 0,
            counter_reads: 0,
            counter_comparisons: 0,
            cell_reads: 0,
            cell_writes: 0,
            field_cell_writes: 0,
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            intent_effects: 0,
            intent_queries: 0,
            zswap_inputs: 0,
            zswap_outputs: 0,
            counter_hash_helpers: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            scalar_helper_calls: 0,
            scalar_counter_reads: 0,
            active_calls: HashSet::new(),
            stateful_circuits: None,
            optional_cells: 0,
            opaque_cells: 0,
            historic_roots: 0,
            historic_writes: 0,
            qualified_set_reads: 0,
            qualified_set_writes: 0,
            qualified_cell_writes: 0,
        };
        let actual = Type::Unsigned { max: "255".into() };
        let target = Type::Unsigned {
            max: "65535".into(),
        };
        let scope = HashMap::from([(
            "n".into(),
            TypedValue {
                ty: actual,
                value: syn::parse_quote!(small),
            },
        )]);
        let expression = Expr::Coerce {
            value: Box::new(Expr::Parameter { name: "n".into() }),
            ty: target.clone(),
        };
        let mut steps = Vec::new();
        let value = plan.expression(&expression, &scope, &mut steps).unwrap();
        assert_eq!(value.ty, target);
        let item: syn::Item = syn::parse_quote!(fn probe(small:runtime::BoundedUint<255>)->Result<(),runtime::CompactError>{#(#steps)* Ok(())});
        let file = syn::File {
            shebang: None,
            attrs: vec![],
            items: vec![item],
        };
        let text = prettyplease::unparse(&file);
        assert!(
            text.split_whitespace()
                .collect::<String>()
                .contains("cast_unsigned::<255,65535,>(small)?"),
            "{text}"
        );
        let narrowing = Expr::Coerce {
            value: Box::new(Expr::Parameter { name: "n".into() }),
            ty: Type::Unsigned { max: "15".into() },
        };
        assert!(
            plan.expression(&narrowing, &scope, &mut Vec::new())
                .is_none()
        );
    }

    #[test]
    fn counter_parameters_require_the_actual_u16_bound() {
        let source = include_str!("../../tests/election-schema13-ir.json");
        let contract: crate::ir::Contract = serde_json::from_str(source).unwrap();
        let ledger = contract
            .ledger_fields
            .iter()
            .map(|field| (field.id.as_str(), field))
            .collect();
        let witnesses = HashMap::new();
        let pure = HashMap::new();
        let mut plan = Plan {
            ledger: &ledger,
            witnesses: &witnesses,
            pure: &pure,
            next: 0,
            witness_calls: 0,
            kernel_self_reads: 0,
            context_query: false,
            root_observations: 0,
            tree_writes: 0,
            set_writes: 0,
            counter_writes: 0,
            counter_reads: 0,
            counter_comparisons: 0,
            cell_reads: 0,
            cell_writes: 0,
            field_cell_writes: 0,
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            intent_effects: 0,
            intent_queries: 0,
            zswap_inputs: 0,
            zswap_outputs: 0,
            counter_hash_helpers: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            scalar_helper_calls: 0,
            scalar_counter_reads: 0,
            active_calls: HashSet::new(),
            stateful_circuits: None,
            optional_cells: 0,
            opaque_cells: 0,
            historic_roots: 0,
            historic_writes: 0,
            qualified_set_reads: 0,
            qualified_set_writes: 0,
            qualified_cell_writes: 0,
        };
        let action = StateAction::CounterIncrement {
            field: "tally_yes".into(),
            index: 3,
            amount: CounterAmount::Parameter {
                name: "amount".into(),
            },
        };
        for (max, expected) in [("65535", true), ("255", false), ("65536", false)] {
            let scope = HashMap::from([(
                "amount".into(),
                TypedValue {
                    ty: Type::Unsigned { max: max.into() },
                    value: syn::parse_quote!(amount),
                },
            )]);
            assert_eq!(
                plan.action(&action, &scope, &mut Vec::new()).is_some(),
                expected
            );
        }
    }

    #[test]
    fn witness_eligibility_is_local_but_counts_all_audited_branches() {
        fn admitted(contract: &crate::ir::Contract, index: usize) -> bool {
            let ledger = contract
                .ledger_fields
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect();
            let witnesses = contract
                .witnesses
                .iter()
                .map(|w| (w.name.as_str(), w))
                .collect();
            lower(
                &contract.stateful_circuits[index],
                &ledger,
                &witnesses,
                &HashMap::new(),
                &HashMap::new(),
            )
            .is_some()
        }
        let mut field: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/root-let-action-return-schema17-ir.json"
        ))
        .unwrap();
        field["witnesses"] =
            serde_json::json!([{"name":"unrelated", "parameters":[], "result":{"kind":"field"}}]);
        assert!(admitted(&serde_json::from_value(field.clone()).unwrap(), 0));
        field["stateful_circuits"][0]["actions"][0]["action"]["actions"][0]["bindings"][0]["value"]
            ["right"] =
            serde_json::json!({"kind":"witness_call", "name":"unrelated", "arguments":[]});
        let mut field: crate::ir::Contract = serde_json::from_value(field).unwrap();
        field.schema_version = crate::ir::SCHEMA_VERSION;
        assert!(!admitted(&field, 0));
        // The legacy profile still rejects witnesses; the terminal-return
        // domain now admits this fully typed zero-argument Field witness.
        let rendered = crate::render_with_capabilities(&field).unwrap();
        assert!(rendered.capabilities.circuits[0].recorded);
        let source: String = rendered
            .source
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        assert!(source.contains("frame.try_witness_metered"));
        assert!(source.contains("witnesses.unrelated("));

        let mut counters: crate::ir::Contract = serde_json::from_str(include_str!(
            "../../tests/counter-less-than-schema15-ir.json"
        ))
        .unwrap();
        counters.witnesses.push(WitnessDeclaration {
            source: None,
            name: "unused_bool".into(),
            parameters: vec![],
            result: Type::Boolean,
        });
        for index in 0..counters.stateful_circuits.len() {
            assert!(admitted(&counters, index));
        }
        // A runtime-false branch is still part of the audited circuit domain.
        let StateReturn::Expression { value } = &mut counters.stateful_circuits[2].return_value
        else {
            unreachable!()
        };
        let Expr::If {
            condition,
            otherwise,
            ..
        } = value
        else {
            unreachable!()
        };
        **condition = Expr::Boolean { value: true };
        **otherwise = Expr::WitnessCall {
            name: "unused_bool".into(),
            arguments: vec![],
        };
        assert!(!admitted(&counters, 2));
        counters.witnesses.push(WitnessDeclaration {
            source: None,
            name: "used_threshold".into(),
            parameters: vec![],
            result: Type::Unsigned {
                max: u64::MAX.to_string(),
            },
        });
        let StateReturn::Expression {
            value: Expr::CounterLessThan { threshold, .. },
        } = &mut counters.stateful_circuits[0].return_value
        else {
            unreachable!()
        };
        **threshold = Expr::WitnessCall {
            name: "used_threshold".into(),
            arguments: vec![],
        };
        assert!(!admitted(&counters, 0));
        counters.schema_version = crate::ir::SCHEMA_VERSION;
        let capabilities = crate::render_with_capabilities(&counters)
            .unwrap()
            .capabilities;
        assert!(!capabilities.circuits[0].recorded);
        assert!(!capabilities.circuits[2].recorded);
    }

    #[test]
    fn field_cell_root_let_keeps_typed_scope_and_rejects_extra_effects() {
        fn admitted(value: &serde_json::Value) -> bool {
            let contract: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
            let ledger = contract
                .ledger_fields
                .iter()
                .map(|field| (field.id.as_str(), field))
                .collect();
            let witnesses = contract
                .witnesses
                .iter()
                .map(|witness| (witness.name.as_str(), witness))
                .collect();
            let pure = HashMap::new();
            lower(
                &contract.stateful_circuits[0],
                &ledger,
                &witnesses,
                &pure,
                &HashMap::new(),
            )
            .is_some()
        }
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/root-let-action-return-schema17-ir.json"
        ))
        .unwrap();
        assert!(admitted(&source));

        let mut wrong_type = source.clone();
        wrong_type["stateful_circuits"][0]["actions"][0]["bindings"][0]["ty"]["kind"] =
            "boolean".into();
        assert!(!admitted(&wrong_type));

        let mut wrong_index = source.clone();
        wrong_index["stateful_circuits"][0]["actions"][0]["bindings"][0]["value"]["index"] =
            1.into();
        assert!(!admitted(&wrong_index));

        let mut wrong_return = source.clone();
        wrong_return["stateful_circuits"][0]["return_value"]["value"]["name"] = "before".into();
        assert!(!admitted(&wrong_return));

        let mut shadowed_return = source.clone();
        shadowed_return["stateful_circuits"][0]["actions"][0]["bindings"][0]["name"] =
            "echo".into();
        assert!(!admitted(&shadowed_return));

        let mut extra_read = source.clone();
        extra_read["stateful_circuits"][0]["actions"][0]["action"]["actions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"kind":"let", "bindings":[{
                "name":"other", "ty":{"kind":"field"},
                "value":{"kind":"cell_read", "field":"stored", "index":0}
            }], "action":{"kind":"sequence", "actions":[]}}));
        assert!(!admitted(&extra_read));

        let mut leaked_local = source.clone();
        leaked_local["stateful_circuits"][0]["actions"][0]["action"]["actions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"kind":"cell_write", "field":"stored",
                "index":0, "value":{"kind":"parameter", "name":"after"}}));
        assert!(!admitted(&leaked_local));

        let mut wrong_write_slot = source;
        wrong_write_slot["stateful_circuits"][0]["actions"][0]["action"]["actions"][0]["action"]
            ["actions"][0]["index"] = 1.into();
        assert!(!admitted(&wrong_write_slot));
    }
}

#[cfg(test)]
mod shielded_receive_tests {
    use super::*;
    use serde_json::{Value, json};

    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/shielded-receive-schema20-ir.json"
        ))
        .unwrap()
    }

    fn admitted(value: &Value) -> Option<TypedPlan> {
        let contract: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
        let circuit = contract
            .stateful_circuits
            .iter()
            .find(|circuit| circuit.name == "accept")?;
        lower_shielded_receive(
            circuit,
            &contract
                .ledger_fields
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect(),
            &contract
                .witnesses
                .iter()
                .map(|w| (w.name.as_str(), w))
                .collect(),
            &contract
                .circuits
                .iter()
                .map(|p| (p.name.as_str(), p))
                .collect(),
            &contract
                .stateful_circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect(),
        )
    }

    #[test]
    fn receive_wrapper_records_exactly_one_output_and_claim_with_audited_pure_calls() {
        let source = source();
        let plan = admitted(&source).expect("unchanged receive helper must be admitted");
        let steps = plan.steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert_eq!(tokens.matches("kernel_self").count(), 1);
        assert_eq!(tokens.matches("create_zswap_output").count(), 1);
        assert_eq!(tokens.matches("CoinReceive").count(), 1);
        assert_eq!(tokens.matches("pure_circuits :: coinCommitment").count(), 1);
        assert!(tokens.find("kernel_self").unwrap() < tokens.find("create_zswap_output").unwrap());
        assert!(tokens.find("create_zswap_output").unwrap() < tokens.find("CoinReceive").unwrap());

        let mut renamed = source.clone();
        renamed["stateful_circuits"][1]["name"] = "another_entry".into();
        let contract: crate::ir::Contract = serde_json::from_value(renamed).unwrap();
        assert!(
            lower_shielded_receive(
                &contract.stateful_circuits[1],
                &HashMap::new(),
                &HashMap::new(),
                &contract
                    .circuits
                    .iter()
                    .map(|p| (p.name.as_str(), p))
                    .collect(),
                &contract
                    .stateful_circuits
                    .iter()
                    .map(|c| (c.name.as_str(), c))
                    .collect(),
            )
            .is_some()
        );
    }

    #[test]
    fn receive_rejects_unused_queries_nested_arguments_and_impure_helpers() {
        let source = source();
        let mut unused_counter = source.clone();
        unused_counter["ledger_fields"] = json!([{
            "id":"low", "index":0, "path":[0], "declaration":{"kind":"counter"}
        }]);
        unused_counter["stateful_circuits"][0]["actions"][0]["bindings"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "name":"unused", "ty":{"kind":"boolean"},
                "value":{"kind":"counter_less_than", "field":"low", "index":0,
                    "threshold":{"kind":"unsigned_literal", "value":"1",
                        "max":"18446744073709551615"}}
            }));
        assert!(admitted(&unused_counter).is_none());

        let mut nested_member = source.clone();
        nested_member["ledger_fields"] = json!([{
            "id":"keys", "index":0, "path":[0],
            "declaration":{"kind":"set", "ty":{"kind":"bytes", "length":32}}
        }]);
        let argument = nested_member["stateful_circuits"][1]["actions"][0]["arguments"][0].clone();
        nested_member["stateful_circuits"][1]["actions"][0]["arguments"][0] = json!({
            "kind":"let", "bindings":[{
                "name":"unused", "ty":{"kind":"boolean"},
                "value":{"kind":"set_member", "field":"keys", "index":0,
                    "value":{"kind":"bytes_literal", "bytes":vec![0;32]}}
            }], "body":argument
        });
        assert!(admitted(&nested_member).is_none());

        let mut impure = source.clone();
        let helper = impure["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinCommitment")
            .unwrap();
        let old_body = helper["body"].clone();
        helper["body"] = json!({"kind":"let", "bindings":[{
            "name":"hidden", "ty":{"kind":"unit"},
            "value":{"kind":"create_zswap_output", "coin":{"kind":"parameter", "name":"coin"},
                "recipient":{"kind":"parameter", "name":"recipient"}}
        }], "body":old_body});
        assert!(admitted(&impure).is_none());

        let mut wrong_pure_result = source.clone();
        let helper = wrong_pure_result["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinCommitment")
            .unwrap();
        helper["result"] = json!({"kind":"boolean"});
        assert!(admitted(&wrong_pure_result).is_none());

        let mut duplicate_pure_formal = source.clone();
        let helper = duplicate_pure_formal["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinCommitment")
            .unwrap();
        helper["parameters"][1]["name"] = "coin".into();
        assert!(admitted(&duplicate_pure_formal).is_none());

        let mut pure_cycle = source.clone();
        let helper = pure_cycle["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinCommitment")
            .unwrap();
        let left = helper["parameters"][0]["name"].clone();
        let right = helper["parameters"][1]["name"].clone();
        helper["body"] = json!({"kind":"call", "name":"coinCommitment",
            "arguments":[{"kind":"parameter", "name":left},
                {"kind":"parameter", "name":right}]});
        assert!(admitted(&pure_cycle).is_none());

        let mut recursive = source.clone();
        recursive["stateful_circuits"][1]["actions"][0]["name"] = "accept".into();
        assert!(admitted(&recursive).is_none());

        let mut wrong_claim = source.clone();
        wrong_claim["stateful_circuits"][0]["actions"][0]["action"]["actions"][1]["action"]["value"]
            ["claim"] = "coin_spend".into();
        assert!(admitted(&wrong_claim).is_none());
    }
}

#[cfg(test)]
mod shielded_send_tests {
    use super::*;
    use serde_json::{Value, json};

    fn source() -> Value {
        serde_json::from_str(include_str!("../../tests/shielded-send-schema20-ir.json")).unwrap()
    }

    fn admitted(source: &Value) -> Vec<bool> {
        let contract: crate::ir::Contract = serde_json::from_value(source.clone()).unwrap();
        let ledger = contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect();
        let witnesses = contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect();
        let pure = contract
            .circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let circuits = contract
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        contract
            .stateful_circuits
            .iter()
            .map(|c| lower_shielded_send(c, &ledger, &witnesses, &pure, &circuits).is_some())
            .collect()
    }

    #[test]
    fn unchanged_send_helper_and_wrappers_have_typed_ordered_plans() {
        let source = source();
        assert_eq!(admitted(&source), vec![true; 5]);
        let contract: crate::ir::Contract = serde_json::from_value(source).unwrap();
        let ledger = HashMap::new();
        let witnesses = HashMap::new();
        let pure = contract
            .circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let circuits = contract
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let plan = lower_shielded_send(
            &contract.stateful_circuits[1],
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
        .unwrap();
        let statements = &plan.steps;
        let steps = quote::quote!(#(#statements)*).to_string();
        assert!(
            steps.find("create_zswap_input").unwrap() < steps.find("subtract_unsigned").unwrap()
        );
        assert!(
            steps.find("subtract_unsigned").unwrap() < steps.find("create_zswap_output").unwrap()
        );
        assert!(steps.contains("transient_hash") && steps.contains("upgrade_from_transient"));
    }

    #[test]
    fn hidden_queries_in_unused_bindings_branches_and_arguments_are_refused() {
        let source = source();
        let mut unused = source.clone();
        let original = unused["stateful_circuits"][1]["return_value"]["value"].take();
        unused["stateful_circuits"][1]["return_value"]["value"] = json!({
            "kind":"let", "bindings":[{"name":"hidden", "ty":{"kind":"boolean"},
                "value":{"kind":"set_member", "field":"coins", "index":0,
                    "value":{"kind":"parameter", "name":"input"}}}], "body":original
        });
        unused["ledger_fields"] = json!([{"id":"coins", "index":0,
            "declaration":{"kind":"set", "ty": source["stateful_circuits"][0]["parameters"][0]["ty"]}}]);
        assert!(!admitted(&unused)[1]);

        let mut branch = source.clone();
        let original = branch["stateful_circuits"][1]["return_value"]["value"].take();
        branch["stateful_circuits"][1]["return_value"]["value"] = json!({
            "kind":"if", "condition":{"kind":"boolean", "value":true},
            "then":original.clone(),
            "otherwise":{"kind":"let", "bindings":[{"name":"hidden", "ty":{"kind":"unsigned", "max":"18446744073709551615"},
                "value":{"kind":"set_size", "field":"coins", "index":0}}], "body": original}
        });
        branch["ledger_fields"] = unused["ledger_fields"].clone();
        assert!(!admitted(&branch)[1]);

        let mut argument = source;
        let original =
            argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0].take();
        argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0] = json!({
            "kind":"let", "bindings":[{"name":"hidden", "ty":{"kind":"boolean"},
                "value":{"kind":"set_member", "field":"coins", "index":0,
                    "value":{"kind":"parameter", "name":"input"}}}], "body":original
        });
        argument["ledger_fields"] = unused["ledger_fields"].clone();
        assert!(!admitted(&argument)[1]);
    }

    #[test]
    fn recursive_impure_and_malformed_helpers_are_refused() {
        let source = source();
        let mut recursive = source.clone();
        recursive["stateful_circuits"][0]["return_value"]["value"] = json!({
            "kind":"call", "name":"sendShielded", "arguments":[
                {"kind":"parameter", "name":"input"},
                {"kind":"parameter", "name":"recipient"},
                {"kind":"parameter", "name":"value"}]
        });
        assert!(!admitted(&recursive)[1]);

        let mut impure = source.clone();
        let helper = impure["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinCommitment")
            .unwrap();
        helper["body"] = json!({"kind":"cell_read", "field":"hidden", "index":0});
        assert!(!admitted(&impure)[1]);

        let mut malformed = source;
        malformed["stateful_circuits"][0]["result"] = json!({"kind":"boolean"});
        assert!(!admitted(&malformed)[1]);
    }
}

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
