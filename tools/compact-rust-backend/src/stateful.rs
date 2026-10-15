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
//!
//! This module owns the pending stack, lexical scopes, and branch restoration.
//! Leaf action and final return domains receive explicit borrowed lowering state.

mod actions;
mod expression;
mod facade;
mod returns;

pub(crate) use expression::NativeExpressionContext;
pub(crate) use expression::render_state_expression_observed;
pub(crate) use facade::render_contract_method;

// Preserve existing internal paths for shared call-graph analysis.
#[cfg(test)]
pub(crate) use crate::circuit_analysis::expression_requires_witness;
pub(crate) use crate::circuit_analysis::{
    circuit_uses_witness, expression_contains_stateful_call, expression_requires_witness_observed,
};

use crate::circuit_analysis::{
    circuit_emits_native_private_output_observed, circuit_uses_witness_observed,
};
use crate::ir::{
    LedgerField, PureCircuit, ReturnPlan, StateAction, StateReturn, StatefulCircuit, Type,
    WitnessDeclaration,
};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use crate::{RenderError, discard_expression, rust_type};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
pub(crate) fn render_stateful_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<syn::Item, RenderError> {
    render_stateful_circuit_observed(
        circuit,
        ledger_fields,
        witnesses,
        circuits,
        stateful_circuits,
        None,
    )
}

/// Existing assembly checks observe their original source operands before error
/// cloning. Errors propagated from deeper originless helpers remain unmapped.
pub(crate) fn render_stateful_circuit_observed(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Item, RenderError> {
    if matches!(circuit.return_value, StateReturn::Effectful { .. }) && !circuit.actions.is_empty()
    {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::node(Owner::StateReturn(&circuit.return_value)));
        }
        return Err(RenderError::MalformedReturnPlan);
    }
    let environment = NativeExpressionContext {
        witnesses,
        circuits,
        stateful_circuits,
        ledger_fields,
        observer,
    };
    let frame = if observer.is_some() {
        crate::native_frame::render_if_supported_observed(circuit, &environment)
    } else {
        crate::native_frame::render_if_supported(
            circuit,
            ledger_fields,
            witnesses,
            stateful_circuits,
        )
    }?;
    if let Some(item) = frame {
        return Ok(item);
    }
    let name = crate::native_header::identifier(
        &circuit.name,
        RefusalSite::field(Owner::Stateful(circuit), "name"),
        observer,
    )?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        crate::native_header::identifier(
            &parameter.name,
            RefusalSite::field(Owner::Parameter(parameter), "name"),
            observer,
        )?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(Owner::Parameter(parameter), "name"));
            }
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = crate::native_header::carrier(&parameter.ty, observer)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }
    let mut statements = Vec::<syn::Stmt>::new();
    let mut uses_witness = circuit_uses_witness_observed(
        circuit,
        stateful_circuits,
        &mut HashSet::new(),
        None,
        observer,
    )?;
    let has_native_private_output = circuit_emits_native_private_output_observed(
        circuit,
        stateful_circuits,
        &mut HashSet::new(),
        None,
        observer,
    )?;
    let mut next_temp = 0;
    let mut next_local = 0;
    let result_ty = crate::native_header::carrier(&circuit.result, observer)?;
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
                        let (rendered, actual, effect) = render_state_expression_observed(
                            value,
                            &local_parameters,
                            &mut effect_statements,
                            &mut next_temp,
                            &mut query_effect,
                            &environment,
                        )?;
                        if actual != circuit.result {
                            if let Some(observer) = observer {
                                observer.refused(RefusalSite::node(Owner::Expr(value)));
                            }
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
                            crate::native_header::identifier(
                                &binding.name,
                                RefusalSite::field(Owner::Binding(binding), "name"),
                                observer,
                            )?;
                            let mut binding_statements = Vec::new();
                            let mut query_effect = false;
                            let (value, actual, effect) = render_state_expression_observed(
                                &binding.value,
                                &local_parameters,
                                &mut binding_statements,
                                &mut next_temp,
                                &mut query_effect,
                                &environment,
                            )?;
                            if actual != binding.ty {
                                if let Some(observer) = observer {
                                    observer
                                        .refused(RefusalSite::node(Owner::Expr(&binding.value)));
                                }
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
                        let (rendered, actual, effect) = render_state_expression_observed(
                            condition,
                            &local_parameters,
                            &mut condition_statements,
                            &mut next_temp,
                            &mut query_effect,
                            &environment,
                        )?;
                        if actual != Type::Boolean {
                            if let Some(observer) = observer {
                                observer.refused(RefusalSite::node(Owner::Expr(condition)));
                            }
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
                    crate::native_header::identifier(
                        &binding.name,
                        RefusalSite::field(Owner::Binding(binding), "name"),
                        observer,
                    )?;
                    let mut binding_statements = Vec::new();
                    let mut query_effect = false;
                    let (value, actual, effect) = render_state_expression_observed(
                        &binding.value,
                        &local_parameters,
                        &mut binding_statements,
                        &mut next_temp,
                        &mut query_effect,
                        &environment,
                    )?;
                    if actual != binding.ty {
                        if let Some(observer) = observer {
                            observer.refused(RefusalSite::node(Owner::Expr(&binding.value)));
                        }
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
                let (rendered, actual, effect) = render_state_expression_observed(
                    condition,
                    &local_parameters,
                    &mut condition_statements,
                    &mut next_temp,
                    &mut query_effect,
                    &environment,
                )?;
                if actual != Type::Boolean {
                    if let Some(observer) = observer {
                        observer.refused(RefusalSite::node(Owner::Expr(condition)));
                    }
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
        if observer.is_some() {
            actions::lower_observed(
                action,
                &parameters,
                &environment,
                actions::ActionOutput {
                    statements: &mut statements,
                    next_temp: &mut next_temp,
                    uses_witness: &mut uses_witness,
                },
            )
        } else {
            actions::lower(
                action,
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                stateful_circuits,
                &mut statements,
                &mut next_temp,
                &mut uses_witness,
            )
        }?;
    }
    let return_expr = if observer.is_some() {
        returns::lower_observed(
            circuit,
            &parameters,
            &return_parameters,
            &environment,
            &result_ty,
            returns::ReturnOutput {
                statements: &mut statements,
                next_temp: &mut next_temp,
                uses_witness: &mut uses_witness,
            },
        )
    } else {
        returns::lower(
            circuit,
            &parameters,
            &return_parameters,
            witnesses,
            circuits,
            stateful_circuits,
            ledger_fields,
            &result_ty,
            &mut statements,
            &mut next_temp,
            &mut uses_witness,
        )
    }?;
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

#[cfg(test)]
mod diagnostic_tests;

#[cfg(test)]
mod local_binding_diagnostic_tests;

#[cfg(test)]
mod header_diagnostic_tests;
