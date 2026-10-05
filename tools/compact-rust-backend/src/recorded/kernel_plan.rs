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

//! Bounded Kernel-only recording, independent of public-slot and return plans.

use super::*;
use crate::ir::KernelClaimKind;

#[derive(Clone)]
struct Value {
    ty: Type,
    expr: syn::Expr,
}
type Scope = HashMap<String, Value>;

struct KernelPlan<'a> {
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    next: usize,
    effects: usize,
}

fn operand_type(ty: &Type) -> bool {
    matches!(ty, Type::Boolean | Type::Bytes { length: 32 })
        || *ty
            == (Type::Unsigned {
                max: u64::MAX.to_string(),
            })
}

impl KernelPlan<'_> {
    fn fresh(&mut self) -> syn::Ident {
        let name = syn::Ident::new(
            &format!("__compact_kernel_{}", self.next),
            Span::call_site(),
        );
        self.next += 1;
        name
    }

    fn value(&mut self, expr: &Expr, scope: &Scope, steps: &mut Vec<syn::Stmt>) -> Option<Value> {
        match expr {
            Expr::Parameter { name } => scope.get(name).cloned(),
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
            Expr::KernelClaim { claim, value } => {
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
        self.effects += 1;
        Some(())
    }

    fn action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        match action {
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
                    steps.push(syn::parse_quote!(let #local: #ty = #expr;));
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
    let mut plan = KernelPlan {
        witnesses,
        next: 0,
        effects: 0,
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
    (plan.effects > 0).then_some(typed_plan::TypedPlan {
        steps,
        result: syn::parse_quote!(()),
    })
}
