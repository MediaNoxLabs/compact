// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Bounded intent recording and inlined Unit helper effects.

use super::*;
use crate::ir::KernelClaimKind;

#[derive(Clone)]
struct Value {
    ty: Type,
    expr: syn::Expr,
}
type Scope = HashMap<String, Value>;

struct ZswapPlan<'a> {
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    next: usize,
    effects: usize,
    public_queries: usize,
    fields: &'a HashMap<&'a str, &'a LedgerField>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    active: HashSet<String>,
}

fn operand_type(ty: &Type) -> bool {
    *ty == crate::stateful::shielded_coin_type()
        || *ty == crate::stateful::qualified_coin_type()
        || *ty == crate::stateful::shielded_recipient_type()
        || matches!(ty, Type::Boolean | Type::Bytes { length: 32 })
        || *ty
            == (Type::Unsigned {
                max: u64::MAX.to_string(),
            })
}

impl ZswapPlan<'_> {
    fn fresh(&mut self) -> syn::Ident {
        let name = syn::Ident::new(&format!("__compact_zswap_{}", self.next), Span::call_site());
        self.next += 1;
        name
    }

    fn value(&mut self, expr: &Expr, scope: &Scope, steps: &mut Vec<syn::Stmt>) -> Option<Value> {
        match expr {
            Expr::Parameter { name } => scope.get(name).cloned(),
            Expr::Coerce { value, ty } => {
                let value = self.value(value, scope, steps)?;
                (value.ty == *ty && operand_type(ty)).then_some(value)
            }
            Expr::WitnessCall { name, arguments } => {
                let witness = *self.witnesses.get(name.as_str())?;
                if !arguments.is_empty()
                    || !witness.parameters.is_empty()
                    || !operand_type(&witness.result)
                {
                    return None;
                }
                let method = ident(name).ok()?;
                let observed = self.fresh();
                let ty = rust_type(&witness.result).ok()?;
                steps.push(syn::parse_quote! {
                    let (frame, #observed): (_, #ty) = frame.try_witness_metered(|context, meter| {
                        witnesses.#method(context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(), meter,
                        }))
                    })?;
                });
                Some(Value {
                    ty: witness.result.clone(),
                    expr: syn::parse_quote!(#observed),
                })
            }
            _ => None,
        }
    }

    fn effect(&mut self, expr: &Expr, scope: &Scope, steps: &mut Vec<syn::Stmt>) -> Option<()> {
        match expr {
            Expr::CreateZswapInput { coin } => {
                let coin = self.value(coin, scope, steps)?;
                if coin.ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                let coin = coin.expr;
                steps.push(syn::parse_quote! { let frame = frame.create_zswap_input(
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()).qualify(#coin.mt_index.value() as u64)
                ); });
                self.effects += 1;
            }
            Expr::CreateZswapOutput { coin, recipient } => {
                let coin = self.value(coin, scope, steps)?;
                if coin.ty != crate::stateful::shielded_coin_type() {
                    return None;
                }
                let recipient = self.value(recipient, scope, steps)?;
                if recipient.ty != crate::stateful::shielded_recipient_type() {
                    return None;
                }
                let (coin, recipient) = (coin.expr, recipient.expr);
                steps.push(syn::parse_quote! { let frame = frame.create_zswap_output(
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
                    runtime::ledger::coin_recipient_from_compact(#recipient.is_left, #recipient.left.bytes, #recipient.right.bytes)
                )?; });
                self.effects += 1;
            }
            Expr::KernelClaim { claim, value } => {
                self.public_queries += 1;
                let value = self.value(value, scope, steps)?;
                if value.ty != (Type::Bytes { length: 32 }) {
                    return None;
                }
                let value = value.expr;
                let (variant, carrier) = match claim {
                    KernelClaimKind::Nullifier => ("Nullifier", "CoinNullifier"),
                    KernelClaimKind::CoinSpend => ("CoinSpend", "CoinCommitment"),
                    KernelClaimKind::CoinReceive => ("CoinReceive", "CoinCommitment"),
                };
                let variant = ident(variant).ok()?;
                let carrier = ident(carrier).ok()?;
                steps.push(syn::parse_quote! {
                    let frame = frame.kernel_claim(runtime::ledger::KernelClaim::#variant(
                        runtime::ledger::#carrier(runtime::ledger::HashOutput((#value).into_array()))
                    ))?;
                });
            }
            Expr::KernelMintShielded { domain, amount } => {
                self.public_queries += 1;
                // Materialize domain before evaluating amount's witness.
                let domain = self.value(domain, scope, steps)?;
                if domain.ty != (Type::Bytes { length: 32 }) {
                    return None;
                }
                let amount = self.value(amount, scope, steps)?;
                if amount.ty
                    != (Type::Unsigned {
                        max: u64::MAX.to_string(),
                    })
                {
                    return None;
                }
                let (domain, amount) = (domain.expr, amount.expr);
                steps.push(syn::parse_quote! {
                    let frame = frame.kernel_mint_shielded(
                        runtime::ledger::HashOutput((#domain).into_array()), (#amount).value() as u64,
                    )?;
                });
            }
            _ => return None,
        }
        Some(())
    }

    fn action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        match action {
            StateAction::CellWriteCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                self.public_queries += 1;
                let declaration = *self.fields.get(field.as_str())?;
                if declaration.index != *index {
                    return None;
                }
                if declaration.declaration
                    != (LedgerFieldKind::Cell {
                        ty: crate::stateful::qualified_coin_type(),
                    })
                {
                    return None;
                }
                let coin = self.value(coin, scope, steps)?;
                let recipient = self.value(recipient, scope, steps)?;
                if coin.ty != crate::stateful::shielded_coin_type()
                    || recipient.ty != crate::stateful::shielded_recipient_type()
                {
                    return None;
                }
                let (coin, recipient) = (coin.expr, recipient.expr);
                let slot = ident(field).ok()?;
                steps.push(syn::parse_quote! { let frame = crate::ledger_slots::#slot.record_write_coin(frame,
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
                    runtime::ledger::coin_recipient_from_compact(#recipient.is_left, #recipient.left.bytes, #recipient.right.bytes)
                )?; });
            }
            StateAction::CircuitCall { name, arguments } => {
                let callee = *self.circuits.get(name.as_str())?;
                if callee.result != Type::Unit
                    || arguments.len() != callee.parameters.len()
                    || !self.active.insert(name.clone())
                {
                    return None;
                }
                let mut scoped = Scope::new();
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let value = self.value(argument, scope, steps)?;
                    if value.ty != parameter.ty || !operand_type(&value.ty) {
                        return None;
                    }
                    let local = self.fresh();
                    let ty = rust_type(&parameter.ty).ok()?;
                    let expr = value.expr;
                    steps.push(syn::parse_quote!(let #local: #ty = (#expr).clone();));
                    if scoped
                        .insert(
                            parameter.name.clone(),
                            Value {
                                ty: parameter.ty.clone(),
                                expr: syn::parse_quote!(#local),
                            },
                        )
                        .is_some()
                    {
                        return None;
                    }
                }
                for action in &callee.actions {
                    self.action(action, &scoped, steps)?;
                }
                match &callee.return_value {
                    StateReturn::Unit => (),
                    StateReturn::Expression { value } => self.effect(value, &scoped, steps)?,
                    _ => return None,
                }
                self.active.remove(name);
            }
            StateAction::Expression { value } => self.effect(value, scope, steps)?,
            StateAction::Sequence { actions } => {
                for action in actions {
                    self.action(action, scope, steps)?;
                }
            }
            StateAction::Let { bindings, action } => {
                let mut scoped = scope.clone();
                for binding in bindings {
                    let value = self.value(&binding.value, &scoped, steps)?;
                    if value.ty != binding.ty || !operand_type(&binding.ty) {
                        return None;
                    }
                    let local = self.fresh();
                    let ty = rust_type(&binding.ty).ok()?;
                    let expr = value.expr;
                    steps.push(syn::parse_quote!(let #local: #ty = (#expr).clone();));
                    scoped.insert(
                        binding.name.clone(),
                        Value {
                            ty: binding.ty.clone(),
                            expr: syn::parse_quote!(#local),
                        },
                    );
                }
                self.action(action, &scoped, steps)?;
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = self.value(condition, scope, steps)?;
                if condition.ty != Type::Boolean {
                    return None;
                }
                let condition = condition.expr;
                let mut then_steps = Vec::new();
                let mut else_steps = Vec::new();
                self.action(then, scope, &mut then_steps)?;
                self.action(otherwise, scope, &mut else_steps)?;
                steps.push(syn::parse_quote! {
                    #[allow(clippy::let_and_return, reason = "branch frames preserve Kernel effect order")]
                    let frame = if #condition { #(#then_steps)* frame } else { #(#else_steps)* frame };
                });
            }
            _ => return None,
        }
        Some(())
    }
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    fields: &'a HashMap<&'a str, &'a LedgerField>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
) -> Option<typed_plan::TypedPlan> {
    if circuit.result != Type::Unit || circuit.internal {
        return None;
    }
    let mut scope = Scope::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        if !operand_type(&parameter.ty) {
            return None;
        }
        let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if scope
            .insert(
                parameter.name.clone(),
                Value {
                    ty: parameter.ty.clone(),
                    expr: syn::parse_quote!(#name),
                },
            )
            .is_some()
        {
            return None;
        }
    }
    let mut plan = ZswapPlan {
        witnesses,
        next: 0,
        effects: 0,
        public_queries: 0,
        fields,
        circuits,
        active: HashSet::from([circuit.name.clone()]),
    };
    let mut steps = Vec::new();
    for action in &circuit.actions {
        plan.action(action, &scope, &mut steps)?;
    }
    match &circuit.return_value {
        StateReturn::Unit => (),
        StateReturn::Expression { value } => plan.effect(value, &scope, &mut steps)?,
        _ => return None,
    }
    // Intent-only exports have no public query and are not proof-required.
    // A query in either branch admits the circuit; an empty selected branch
    // still reaches the existing EmptyTranscript preparation boundary.
    (plan.effects > 0 && plan.public_queries > 0).then_some(typed_plan::TypedPlan {
        steps,
        result: syn::parse_quote!(()),
    })
}
