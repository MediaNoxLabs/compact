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

//! Native circuit assembly: lexical action/return sequencing and frame selection.

mod expression;
mod facade;

pub(crate) use expression::render_state_expression;
pub(crate) use facade::render_contract_method;

// Preserve existing internal paths for shared call-graph analysis.
pub(crate) use crate::circuit_analysis::{
    circuit_uses_witness, expression_contains_stateful_call, expression_requires_witness,
};

use crate::circuit_analysis::circuit_emits_native_private_output;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, NativeWitnessBuiltin, PureCircuit,
    ReturnPlan, StateAction, StateReturn, StatefulCircuit, StructField, Type, WitnessDeclaration,
};
use crate::{
    RenderError, discard_expression, expression_with_calls, ident, list_head_result_type, rust_type,
};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

pub(crate) fn render_stateful_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<syn::Item, RenderError> {
    if matches!(circuit.return_value, StateReturn::Effectful { .. }) && !circuit.actions.is_empty()
    {
        return Err(RenderError::MalformedReturnPlan);
    }
    if let Some(item) = crate::native_frame::render_if_supported(
        circuit,
        ledger_fields,
        witnesses,
        stateful_circuits,
    )? {
        return Ok(item);
    }
    let name = ident(&circuit.name)?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }
    let mut statements = Vec::<syn::Stmt>::new();
    let mut uses_witness = circuit_uses_witness(circuit, stateful_circuits, &mut HashSet::new())?;
    let has_native_private_output =
        circuit_emits_native_private_output(circuit, stateful_circuits, &mut HashSet::new())?;
    let mut next_temp = 0;
    let mut next_local = 0;
    let result_ty = rust_type(&circuit.result)?;
    enum Pending<'a> {
        Action(&'a StateAction, bool),
        Plan(&'a ReturnPlan),
        RestoreScope,
        EndThen(usize, &'a StateAction),
        EndElse(usize),
        PlanEndThen(usize, &'a ReturnPlan),
        PlanEndElse(usize),
    }
    struct BranchFrame<'a> {
        condition: syn::Expr,
        parent_statements: Vec<syn::Stmt>,
        then_statements: Vec<syn::Stmt>,
        parameters: HashMap<&'a str, (&'a Type, syn::Ident)>,
    }
    let mut pending = match &circuit.return_value {
        StateReturn::Effectful { body } => vec![Pending::Plan(body)],
        _ => circuit
            .actions
            .iter()
            .rev()
            .enumerate()
            .map(|(index, action)| Pending::Action(action, index == 0))
            .collect(),
    };
    let mut scopes = Vec::new();
    let mut branches = Vec::<Option<BranchFrame>>::new();
    let mut local_parameters = parameters.clone();
    let mut return_parameters = parameters.clone();
    while let Some(pending_action) = pending.pop() {
        let (action, returns_from_scope) = match pending_action {
            Pending::Action(action, returns_from_scope) => (action, returns_from_scope),
            Pending::Plan(plan) => {
                match plan {
                    ReturnPlan::Value { value } => {
                        let mut effect_statements = Vec::new();
                        let mut query_effect = false;
                        let (rendered, actual, effect) = render_state_expression(
                            value,
                            &local_parameters,
                            witnesses,
                            &mut effect_statements,
                            &mut next_temp,
                            circuits,
                            stateful_circuits,
                            ledger_fields,
                            &mut query_effect,
                        )?;
                        if actual != circuit.result {
                            return Err(RenderError::TypeMismatch {
                                expected: circuit.result.clone(),
                                actual,
                            });
                        }
                        uses_witness |= effect;
                        if effect || query_effect {
                            statements.push(syn::parse_quote!(let mut context = context;));
                        }
                        statements.extend(effect_statements);
                        statements.push(syn::parse_quote!(let __compact_effectful_result: #result_ty = #rendered;));
                    }
                    ReturnPlan::Sequence { actions, result } => {
                        pending.push(Pending::Plan(result));
                        pending.extend(
                            actions
                                .iter()
                                .rev()
                                .map(|action| Pending::Action(action, false)),
                        );
                    }
                    ReturnPlan::Let { bindings, result } => {
                        scopes.push(local_parameters.clone());
                        for binding in bindings {
                            ident(&binding.name)?;
                            let mut binding_statements = Vec::new();
                            let mut query_effect = false;
                            let (value, actual, effect) = render_state_expression(
                                &binding.value,
                                &local_parameters,
                                witnesses,
                                &mut binding_statements,
                                &mut next_temp,
                                circuits,
                                stateful_circuits,
                                ledger_fields,
                                &mut query_effect,
                            )?;
                            if actual != binding.ty {
                                return Err(RenderError::TypeMismatch {
                                    expected: binding.ty.clone(),
                                    actual,
                                });
                            }
                            uses_witness |= effect;
                            if effect || query_effect {
                                statements.push(syn::parse_quote!(let mut context = context;));
                            }
                            statements.extend(binding_statements);
                            let local_name = syn::Ident::new(
                                &format!("__compact_return_local_{next_local}"),
                                Span::call_site(),
                            );
                            next_local += 1;
                            let ty = rust_type(&binding.ty)?;
                            statements.push(syn::parse_quote!(let #local_name: #ty = #value;));
                            local_parameters
                                .insert(binding.name.as_str(), (&binding.ty, local_name));
                        }
                        pending.push(Pending::RestoreScope);
                        pending.push(Pending::Plan(result));
                    }
                    ReturnPlan::Conditional {
                        condition,
                        then,
                        otherwise,
                    } => {
                        let mut condition_statements = Vec::new();
                        let mut query_effect = false;
                        let (rendered, actual, effect) = render_state_expression(
                            condition,
                            &local_parameters,
                            witnesses,
                            &mut condition_statements,
                            &mut next_temp,
                            circuits,
                            stateful_circuits,
                            ledger_fields,
                            &mut query_effect,
                        )?;
                        if actual != Type::Boolean {
                            return Err(RenderError::TypeMismatch {
                                expected: Type::Boolean,
                                actual,
                            });
                        }
                        uses_witness |= effect;
                        statements.push(syn::parse_quote!(let mut context = context;));
                        statements.extend(condition_statements);
                        let index = branches.len();
                        branches.push(Some(BranchFrame {
                            condition: rendered,
                            parent_statements: std::mem::take(&mut statements),
                            then_statements: Vec::new(),
                            parameters: local_parameters.clone(),
                        }));
                        pending.push(Pending::PlanEndThen(index, otherwise));
                        pending.push(Pending::Plan(then));
                    }
                }
                continue;
            }
            Pending::RestoreScope => {
                local_parameters = scopes.pop().expect("scope marker has a matching scope");
                continue;
            }
            Pending::EndThen(index, otherwise) => {
                let frame = branches[index].as_mut().expect("open conditional branch");
                frame.then_statements = std::mem::take(&mut statements);
                local_parameters = frame.parameters.clone();
                pending.push(Pending::EndElse(index));
                pending.push(Pending::Action(otherwise, false));
                continue;
            }
            Pending::EndElse(index) => {
                let frame = branches[index].take().expect("open conditional branch");
                let else_statements = std::mem::take(&mut statements);
                local_parameters = frame.parameters;
                statements = frame.parent_statements;
                let condition = frame.condition;
                let then_statements = frame.then_statements;
                statements.push(syn::parse_quote! {
                    context = if #condition {
                        #[allow(unused_mut)]
                        let mut context = context;
                        #(#then_statements)*
                        context
                    } else {
                        #[allow(unused_mut)]
                        let mut context = context;
                        #(#else_statements)*
                        context
                    };
                });
                continue;
            }
            Pending::PlanEndThen(index, otherwise) => {
                let frame = branches[index].as_mut().expect("open return branch");
                frame.then_statements = std::mem::take(&mut statements);
                local_parameters = frame.parameters.clone();
                pending.push(Pending::PlanEndElse(index));
                pending.push(Pending::Plan(otherwise));
                continue;
            }
            Pending::PlanEndElse(index) => {
                let frame = branches[index].take().expect("open return branch");
                let else_statements = std::mem::take(&mut statements);
                local_parameters = frame.parameters;
                statements = frame.parent_statements;
                let condition = frame.condition;
                let then_statements = frame.then_statements;
                statements.push(syn::parse_quote! {
                    let (context, __compact_effectful_result): (_, #result_ty) = if #condition {
                        let mut context = context;
                        #(#then_statements)*
                        (context, __compact_effectful_result)
                    } else {
                        let mut context = context;
                        #(#else_statements)*
                        (context, __compact_effectful_result)
                    };
                });
                continue;
            }
        };
        match action {
            StateAction::Sequence { actions } => {
                pending.extend(actions.iter().rev().enumerate().map(|(index, action)| {
                    Pending::Action(action, returns_from_scope && index == 0)
                }));
                continue;
            }
            StateAction::Let {
                bindings,
                action: inner,
            } => {
                scopes.push(local_parameters.clone());
                for binding in bindings {
                    ident(&binding.name)?;
                    let mut binding_statements = Vec::new();
                    let mut query_effect = false;
                    let (value, actual, effect) = render_state_expression(
                        &binding.value,
                        &local_parameters,
                        witnesses,
                        &mut binding_statements,
                        &mut next_temp,
                        circuits,
                        stateful_circuits,
                        ledger_fields,
                        &mut query_effect,
                    )?;
                    if actual != binding.ty {
                        return Err(RenderError::TypeMismatch {
                            expected: binding.ty.clone(),
                            actual,
                        });
                    }
                    if effect {
                        uses_witness = true;
                    }
                    if effect || query_effect {
                        statements.push(syn::parse_quote!(let mut context = context;));
                    }
                    statements.extend(binding_statements);
                    let local_name = syn::Ident::new(
                        &format!("__compact_action_local_{next_local}"),
                        Span::call_site(),
                    );
                    next_local += 1;
                    let ty = rust_type(&binding.ty)?;
                    statements.push(syn::parse_quote!(let #local_name: #ty = #value;));
                    local_parameters.insert(binding.name.as_str(), (&binding.ty, local_name));
                }
                // The extracted return remains inside the terminal lexical
                // suffix: final top-level action -> final Sequence child ->
                // Let body. Branches and earlier sibling scopes do not escape.
                if returns_from_scope {
                    return_parameters = local_parameters.clone();
                }
                pending.push(Pending::RestoreScope);
                pending.push(Pending::Action(inner, returns_from_scope));
                continue;
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let mut condition_statements = Vec::new();
                let mut query_effect = false;
                let (rendered, actual, effect) = render_state_expression(
                    condition,
                    &local_parameters,
                    witnesses,
                    &mut condition_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if actual != Type::Boolean {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Boolean,
                        actual,
                    });
                }
                uses_witness |= effect;
                statements.push(syn::parse_quote!(let mut context = context;));
                statements.extend(condition_statements);
                let index = branches.len();
                branches.push(Some(BranchFrame {
                    condition: rendered,
                    parent_statements: std::mem::take(&mut statements),
                    then_statements: Vec::new(),
                    parameters: local_parameters.clone(),
                }));
                pending.push(Pending::EndThen(index, otherwise));
                pending.push(Pending::Action(then, false));
                continue;
            }
            _ => {}
        }
        let parameters = local_parameters.clone();
        match action {
            StateAction::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            } => {
                statements.push(syn::parse_quote! {
                    private_transcript_outputs.extend([runtime::fab::AlignedValue::from(
                        context.own_coin_public_key()?
                    )]);
                });
            }
            StateAction::Expression { value } => {
                let mut effect_statements = Vec::new();
                let mut query_effect = false;
                let (rendered, ty, effect) = render_state_expression(
                    value,
                    &parameters,
                    witnesses,
                    &mut effect_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if effect {
                    uses_witness = true;
                }
                if effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(effect_statements);
                statements.extend(discard_expression(rendered, &ty));
            }
            StateAction::PureCall { name, arguments } => {
                let call = Expr::Call {
                    name: name.clone(),
                    arguments: arguments.clone(),
                };
                let (rendered, _) = expression_with_calls(&call, &parameters, circuits)?;
                statements.push(syn::parse_quote!(#rendered;));
            }
            StateAction::CircuitCall {
                name: callee_name,
                arguments,
            } => {
                let callee = stateful_circuits
                    .get(callee_name.as_str())
                    .ok_or_else(|| RenderError::UnknownCircuit(callee_name.clone()))?;
                let callee_uses_witness =
                    circuit_uses_witness(callee, stateful_circuits, &mut HashSet::new())?;
                let callee_emits_native = circuit_emits_native_private_output(
                    callee,
                    stateful_circuits,
                    &mut HashSet::new(),
                )?;
                if arguments.len() != callee.parameters.len() {
                    return Err(RenderError::ArgumentCount {
                        circuit: callee_name.clone(),
                        expected: callee.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                let mut args = Vec::<syn::Expr>::new();
                let mut argument_statements = Vec::new();
                let mut argument_query_effect = false;
                let mut argument_witness_effect = false;
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let (rendered, actual, effect) = render_state_expression(
                        argument,
                        &parameters,
                        witnesses,
                        &mut argument_statements,
                        &mut next_temp,
                        circuits,
                        stateful_circuits,
                        ledger_fields,
                        &mut argument_query_effect,
                    )?;
                    if actual != parameter.ty {
                        return Err(RenderError::TypeMismatch {
                            expected: parameter.ty.clone(),
                            actual,
                        });
                    }
                    let argument_name = syn::Ident::new(
                        &format!("__compact_call_argument_{}", next_temp),
                        Span::call_site(),
                    );
                    next_temp += 1;
                    argument_statements.push(syn::parse_quote!(let #argument_name = #rendered;));
                    args.push(syn::parse_quote!(#argument_name));
                    argument_witness_effect |= effect;
                }
                if argument_query_effect || argument_witness_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(argument_statements);
                uses_witness |= argument_witness_effect;
                let callee_name = ident(callee_name)?;
                if callee_uses_witness {
                    statements.push(syn::parse_quote! {
                        let call_step = #callee_name(context, witnesses, #(#args),*)?;
                    });
                    uses_witness = true;
                } else {
                    statements.push(syn::parse_quote! {
                        let call_step = #callee_name(context, #(#args),*)?;
                    });
                }
                if callee_uses_witness || callee_emits_native {
                    statements.push(syn::parse_quote! {
                        private_transcript_outputs.extend(call_step.private_transcript_outputs);
                    });
                }
                statements.push(syn::parse_quote!(let context = call_step.context;));
                statements.push(syn::parse_quote!(total_cost += call_step.gas_cost;));
            }
            StateAction::Assert { condition, message } => {
                let mut effect_statements = Vec::new();
                let mut query_effect = false;
                let (condition, actual, effect) = render_state_expression(
                    condition,
                    &parameters,
                    witnesses,
                    &mut effect_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if actual != Type::Boolean {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Boolean,
                        actual,
                    });
                }
                if effect {
                    uses_witness = true;
                }
                if effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(effect_statements);
                statements.push(syn::parse_quote! {
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                });
            }
            StateAction::Let { .. } | StateAction::Sequence { .. } | StateAction::If { .. } => {
                unreachable!("control actions were expanded before rendering")
            }
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            }
            | StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                let amount: syn::Expr = match amount {
                    CounterAmount::Literal { value } => {
                        let value = syn::LitInt::new(&value.to_string(), Span::call_site());
                        syn::parse_quote!(#value)
                    }
                    CounterAmount::Parameter { name } => {
                        let (ty, rust_name) = parameters
                            .get(name.as_str())
                            .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
                        let expected = Type::Unsigned {
                            max: "65535".into(),
                        };
                        if *ty != &expected {
                            return Err(RenderError::TypeMismatch {
                                expected,
                                actual: (*ty).clone(),
                            });
                        }
                        syn::parse_quote!(#rust_name.value() as u16)
                    }
                };
                let method = if matches!(action, StateAction::CounterIncrement { .. }) {
                    syn::Ident::new("increment", Span::call_site())
                } else {
                    syn::Ident::new("decrement", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context, #amount)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::CounterReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset(context)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut value_statements = Vec::new();
                let mut query_effect = false;
                let (value, actual, witness_effect) = render_state_expression(
                    value,
                    &parameters,
                    witnesses,
                    &mut value_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                if witness_effect {
                    uses_witness = true;
                }
                if witness_effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(value_statements);
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote!(
                    let step = crate::ledger_slots::#slot.write(context, #value)?;
                ));
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::ListPushFront {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::List { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (value, actual) = expression_with_calls(value, &parameters, circuits)?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.push_front(context, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::ListPopFront { field, index }
            | StateAction::ListReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                let method = if matches!(action, StateAction::ListPopFront { .. }) {
                    "pop_front"
                } else {
                    "reset"
                };
                let method = syn::Ident::new(method, Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::SetInsert {
                field,
                index,
                value,
            }
            | StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (value, actual) = expression_with_calls(value, &parameters, circuits)?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let slot = ident(&declaration.id)?;
                let method = if matches!(action, StateAction::SetInsert { .. }) {
                    syn::Ident::new("insert", Span::call_site())
                } else {
                    syn::Ident::new("remove", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(context, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::SetInsertCoin {
                field,
                index,
                coin,
                recipient,
            }
            | StateAction::CellWriteCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let expected = qualified_coin_type();
                let cell_write = matches!(action, StateAction::CellWriteCoin { .. });
                let expected_kind = if cell_write {
                    LedgerFieldKind::Cell { ty: expected }
                } else {
                    LedgerFieldKind::Set { ty: expected }
                };
                if declaration.declaration != expected_kind || declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut operands = Vec::new();
                let mut query_effect = false;
                let (coin_value, coin_ty, coin_witness) = render_state_expression(
                    coin,
                    &parameters,
                    witnesses,
                    &mut operands,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                let expected_coin = shielded_coin_type();
                if coin_ty != expected_coin {
                    return Err(RenderError::TypeMismatch {
                        expected: expected_coin,
                        actual: coin_ty,
                    });
                }
                let coin_name =
                    syn::Ident::new(&format!("__compact_coin_{}", next_temp), Span::call_site());
                next_temp += 1;
                operands.push(syn::parse_quote!(let #coin_name = #coin_value;));
                let (recipient_value, recipient_ty, recipient_witness) = render_state_expression(
                    recipient,
                    &parameters,
                    witnesses,
                    &mut operands,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                let expected_recipient = shielded_recipient_type();
                if recipient_ty != expected_recipient {
                    return Err(RenderError::TypeMismatch {
                        expected: expected_recipient,
                        actual: recipient_ty,
                    });
                }
                let recipient_name = syn::Ident::new(
                    &format!("__compact_recipient_{}", next_temp),
                    Span::call_site(),
                );
                next_temp += 1;
                operands.push(syn::parse_quote!(let #recipient_name = #recipient_value;));
                if coin_witness || recipient_witness {
                    uses_witness = true;
                }
                if coin_witness || recipient_witness || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(operands);
                let slot = ident(&declaration.id)?;
                let method = ident(if cell_write {
                    "write_coin"
                } else {
                    "insert_coin"
                })?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.#method(
                        context,
                        runtime::ledger::coin_info_from_compact(
                            #coin_name.nonce,
                            #coin_name.color,
                            #coin_name.value.value(),
                        ),
                        runtime::ledger::coin_recipient_from_compact(
                            #recipient_name.is_left,
                            #recipient_name.left.bytes,
                            #recipient_name.right.bytes,
                        ),
                    )?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::SetReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset(context)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapInsert {
                field,
                index,
                key,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
                if &actual_key != key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let (value, actual_value) = expression_with_calls(value, &parameters, circuits)?;
                if &actual_value != value_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: value_ty.clone(),
                        actual: actual_value,
                    });
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.insert(context, #key, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapInsertDefault { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
                if &actual_key != key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.insert_default(context, #key)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapRemove { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Map { key: key_ty, .. } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
                if &actual_key != key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.remove(context, #key)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::MapReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset(context)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::HistoricMerkleResetHistory { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(
                    declaration.declaration,
                    LedgerFieldKind::HistoricMerkleTree { .. }
                ) || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset_history(context)?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::HistoricMerkleResetToDefault { field, index }
            | StateAction::MerkleResetToDefault { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let historic = match declaration.declaration {
                    LedgerFieldKind::HistoricMerkleTree { .. } => true,
                    LedgerFieldKind::MerkleTree { .. } => false,
                    _ => return Err(RenderError::UnknownLedgerField(field.clone())),
                };
                let action_historic =
                    matches!(action, StateAction::HistoricMerkleResetToDefault { .. });
                if declaration.index != *index || historic != action_historic {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.reset_to_default(context)?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::HistoricMerkleInsert {
                field,
                index,
                value,
            }
            | StateAction::MerkleInsert {
                field,
                index,
                value,
            }
            | StateAction::HistoricMerkleInsertIndex {
                field,
                index,
                value,
                ..
            }
            | StateAction::MerkleInsertIndex {
                field,
                index,
                value,
                ..
            }
            | StateAction::HistoricMerkleInsertHash {
                field,
                index,
                hash: value,
            }
            | StateAction::MerkleInsertHash {
                field,
                index,
                hash: value,
            }
            | StateAction::HistoricMerkleInsertHashIndex {
                field,
                index,
                hash: value,
                ..
            }
            | StateAction::MerkleInsertHashIndex {
                field,
                index,
                hash: value,
                ..
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let (ty, historic) = match &declaration.declaration {
                    LedgerFieldKind::HistoricMerkleTree { ty, .. } => (ty, true),
                    LedgerFieldKind::MerkleTree { ty, .. } => (ty, false),
                    _ => return Err(RenderError::UnknownLedgerField(field.clone())),
                };
                let action_historic = matches!(
                    action,
                    StateAction::HistoricMerkleInsert { .. }
                        | StateAction::HistoricMerkleInsertIndex { .. }
                        | StateAction::HistoricMerkleInsertHash { .. }
                        | StateAction::HistoricMerkleInsertHashIndex { .. }
                );
                if declaration.index != *index || historic != action_historic {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut value_statements = Vec::new();
                let mut query_effect = false;
                let (value, actual, value_witness_effect) = render_state_expression(
                    value,
                    &parameters,
                    witnesses,
                    &mut value_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                let is_hash = matches!(
                    action,
                    StateAction::HistoricMerkleInsertHash { .. }
                        | StateAction::HistoricMerkleInsertHashIndex { .. }
                        | StateAction::MerkleInsertHash { .. }
                        | StateAction::MerkleInsertHashIndex { .. }
                );
                let expected = if is_hash {
                    Type::Bytes { length: 32 }
                } else {
                    ty.clone()
                };
                if actual != expected {
                    return Err(RenderError::TypeMismatch { expected, actual });
                }
                let mut witness_effect = value_witness_effect;
                let position = match action {
                    StateAction::HistoricMerkleInsertIndex { position, .. }
                    | StateAction::HistoricMerkleInsertHashIndex { position, .. }
                    | StateAction::MerkleInsertIndex { position, .. }
                    | StateAction::MerkleInsertHashIndex { position, .. } => {
                        let (rendered, actual, effect) = render_state_expression(
                            position,
                            &parameters,
                            witnesses,
                            &mut value_statements,
                            &mut next_temp,
                            circuits,
                            stateful_circuits,
                            ledger_fields,
                            &mut query_effect,
                        )?;
                        let expected = Type::Unsigned {
                            max: u64::MAX.to_string(),
                        };
                        if actual != expected {
                            return Err(RenderError::TypeMismatch { expected, actual });
                        }
                        witness_effect |= effect;
                        Some(rendered)
                    }
                    _ => None,
                };
                uses_witness |= witness_effect;
                if witness_effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(value_statements);
                let slot = ident(&declaration.id)?;
                let method = match (is_hash, position.is_some()) {
                    (false, false) => "insert",
                    (false, true) => "insert_index",
                    (true, false) => "insert_hash",
                    (true, true) => "insert_hash_index",
                };
                let method = syn::Ident::new(method, Span::call_site());
                if let Some(position) = position {
                    statements.push(syn::parse_quote! {
                        let step = crate::ledger_slots::#slot.#method(context, #value, #position)?;
                    });
                } else {
                    statements.push(syn::parse_quote! {
                        let step = crate::ledger_slots::#slot.#method(context, #value)?;
                    });
                }
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
            StateAction::HistoricMerkleInsertIndexDefault {
                field,
                index,
                position,
            }
            | StateAction::MerkleInsertIndexDefault {
                field,
                index,
                position,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let historic = match &declaration.declaration {
                    LedgerFieldKind::HistoricMerkleTree { .. } => true,
                    LedgerFieldKind::MerkleTree { .. } => false,
                    _ => return Err(RenderError::UnknownLedgerField(field.clone())),
                };
                let action_historic =
                    matches!(action, StateAction::HistoricMerkleInsertIndexDefault { .. });
                if declaration.index != *index || historic != action_historic {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let mut value_statements = Vec::new();
                let mut query_effect = false;
                let (position, actual, witness_effect) = render_state_expression(
                    position,
                    &parameters,
                    witnesses,
                    &mut value_statements,
                    &mut next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                let expected = Type::Unsigned {
                    max: u64::MAX.to_string(),
                };
                if actual != expected {
                    return Err(RenderError::TypeMismatch { expected, actual });
                }
                if witness_effect {
                    uses_witness = true;
                }
                if witness_effect || query_effect {
                    statements.push(syn::parse_quote!(let mut context = context;));
                }
                statements.extend(value_statements);
                let slot = ident(&declaration.id)?;
                statements.push(syn::parse_quote! {
                    let step = crate::ledger_slots::#slot.insert_index_default(context, #position)?;
                });
                statements.push(syn::parse_quote!(let context = step.context;));
                statements.push(syn::parse_quote!(total_cost += step.gas_cost;));
            }
        }
    }
    let return_expr: syn::Expr = match &circuit.return_value {
        StateReturn::Effectful { .. } => syn::parse_quote!(__compact_effectful_result),
        StateReturn::Expression { value } => {
            let mut effect_statements = Vec::new();
            let mut query_effect = false;
            let (rendered, actual, effect) = render_state_expression(
                value,
                &return_parameters,
                witnesses,
                &mut effect_statements,
                &mut next_temp,
                circuits,
                stateful_circuits,
                ledger_fields,
                &mut query_effect,
            )?;
            if actual != circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual,
                });
            }
            if effect {
                uses_witness = true;
            }
            if effect || query_effect {
                statements.push(syn::parse_quote!(let mut context = context;));
            }
            statements.extend(effect_statements);
            rendered
        }
        StateReturn::Unit => {
            if circuit.result != Type::Unit {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: Type::Unit,
                });
            }
            syn::parse_quote!(())
        }
        StateReturn::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if ty != &circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: ty.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote!(
                let read_step = crate::ledger_slots::#slot.read(context)?;
            ));
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::CounterRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.read(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger Counter fits Uint<64>"))
        }
        StateReturn::ListLength { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.length(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger List length fits Uint<64>"))
        }
        StateReturn::ListIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.is_empty(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::ListHead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::List { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = list_head_result_type(ty, &circuit.result);
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.head::<#result_ty, _, _>(context)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::HistoricMerkleIsFull { field, index }
        | StateReturn::MerkleIsFull { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let return_historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleIsFull { .. }
            );
            if declaration.index != *index || historic != return_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.is_full(context)?;
            });
            statements.push(syn::parse_quote!(let context = read_step.context;));
            statements.push(syn::parse_quote!(total_cost += read_step.gas_cost;));
            syn::parse_quote!(read_step.result)
        }
        StateReturn::HistoricMerkleCheckRoot { field, index, root }
        | StateReturn::MerkleCheckRoot { field, index, root } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let historic = match declaration.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => true,
                LedgerFieldKind::MerkleTree { .. } => false,
                _ => return Err(RenderError::UnknownLedgerField(field.clone())),
            };
            let return_historic = matches!(
                circuit.return_value,
                StateReturn::HistoricMerkleCheckRoot { .. }
            );
            if declaration.index != *index || historic != return_historic {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let expected = Type::Struct {
                name: "MerkleTreeDigest".into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            };
            let (root, actual) = expression_with_calls(root, &parameters, circuits)?;
            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.check_root(context, #root)?;
            });
            statements.push(syn::parse_quote!(let context = read_step.context;));
            statements.push(syn::parse_quote!(total_cost += read_step.gas_cost;));
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetMember {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let (value, actual) = expression_with_calls(value, &parameters, circuits)?;
            if &actual != ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let slot = ident(&declaration.id)?;
            statements.push(syn::parse_quote! {
                let read_step = crate::ledger_slots::#slot.member(context, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetSize { field, index }
        | StateReturn::SetIsEmpty { field, index }
        | StateReturn::MapSize { field, index }
        | StateReturn::MapIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let is_map = matches!(
                &circuit.return_value,
                StateReturn::MapSize { .. } | StateReturn::MapIsEmpty { .. }
            );
            let declaration_matches = if is_map {
                matches!(declaration.declaration, LedgerFieldKind::Map { .. })
            } else {
                matches!(declaration.declaration, LedgerFieldKind::Set { .. })
            };
            if !declaration_matches || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let is_size = matches!(
                &circuit.return_value,
                StateReturn::SetSize { .. } | StateReturn::MapSize { .. }
            );
            let expected = if is_size {
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                }
            } else {
                Type::Boolean
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            let method = if is_size { "size" } else { "is_empty" };
            let method = syn::Ident::new(method, Span::call_site());
            statements.push(syn::parse_quote!(
                let read_step = crate::ledger_slots::#slot.#method(context)?;
            ));
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            if is_size {
                let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
                syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger collection size fits Uint<64>"))
            } else {
                syn::parse_quote!(read_step.result)
            }
        }
        StateReturn::MapMember { field, index, key }
        | StateReturn::MapLookup { field, index, key } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Map {
                key: key_ty,
                value: value_ty,
            } = &declaration.declaration
            else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let (key, actual_key) = expression_with_calls(key, &parameters, circuits)?;
            if &actual_key != key_ty {
                return Err(RenderError::TypeMismatch {
                    expected: key_ty.clone(),
                    actual: actual_key,
                });
            }
            let is_member = matches!(&circuit.return_value, StateReturn::MapMember { .. });
            let expected = if is_member {
                Type::Boolean
            } else {
                value_ty.clone()
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let slot = ident(&declaration.id)?;
            if is_member {
                statements.push(syn::parse_quote! {
                    let read_step = crate::ledger_slots::#slot.member(context, #key)?;
                });
            } else {
                statements.push(syn::parse_quote! {
                    let read_step = crate::ledger_slots::#slot.lookup(context, #key)?;
                });
            }
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
    };
    let transcript_init: syn::Stmt = if uses_witness || has_native_private_output {
        syn::parse_quote!(let mut private_transcript_outputs = Vec::new();)
    } else {
        syn::parse_quote!(let private_transcript_outputs = Vec::new();)
    };
    let cost_init: syn::Stmt =
        syn::parse_quote!(let mut total_cost = runtime::context::RunningCost::default(););
    let visibility: syn::Visibility = if circuit.internal {
        syn::parse_quote!(pub(crate))
    } else {
        syn::parse_quote!(pub)
    };
    let (result_statements, final_result): (Vec<syn::Stmt>, syn::FieldValue) = if circuit.result
        == Type::Unit
        && !matches!(&return_expr, syn::Expr::Tuple(tuple) if tuple.elems.is_empty())
    {
        (
            discard_expression(return_expr, &Type::Unit),
            syn::parse_quote!(result: ()),
        )
    } else {
        (
            vec![syn::parse_quote!(let result = #return_expr;)],
            syn::parse_quote!(result),
        )
    };
    let item: syn::Item = if uses_witness {
        syn::parse_quote! {
            #visibility fn #name<Private, W: TryWitnesses<Private>>(
                context: runtime::context::CircuitContext<Private>,
                witnesses: &W,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
                #cost_init
                #transcript_init
                #(#statements)*
                #(#result_statements)*
                Ok(runtime::context::CircuitResult {
                    context,
                    #final_result,
                    gas_cost: total_cost,
                    private_transcript_outputs,
                })
            }
        }
    } else {
        syn::parse_quote! {
            #visibility fn #name<Private>(
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
                #cost_init
                #transcript_init
                #(#statements)*
                #(#result_statements)*
                Ok(runtime::context::CircuitResult {
                    context,
                    #final_result,
                    gas_cost: total_cost,
                    private_transcript_outputs,
                })
            }
        }
    };
    Ok(item)
}
