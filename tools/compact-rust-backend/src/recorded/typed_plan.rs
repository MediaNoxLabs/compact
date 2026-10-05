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
use crate::ir::ReturnPlan;

#[derive(Clone)]
struct TypedValue {
    ty: Type,
    value: syn::Expr,
}

type Scope = HashMap<String, TypedValue>;

struct Plan<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    next: usize,
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
        let supported = self.pure_value(&callee.body, visiting);
        visiting.remove(name);
        supported
    }

    fn pure_value(&self, value: &Expr, visiting: &mut HashSet<String>) -> bool {
        match value {
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
            Expr::KernelSelf { ty } if *ty == contract_address_type() => {
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
                if then.ty != Type::Boolean || otherwise.ty != Type::Boolean {
                    return None;
                }
                let name = self.fresh();
                let (condition, then, otherwise) = (condition.value, then.value, otherwise.value);
                steps.push(syn::parse_quote! {
                    let (frame, #name): (_, bool) = if #condition {
                        #(#then_steps)*
                        (frame, #then)
                    } else {
                        #(#else_steps)*
                        (frame, #otherwise)
                    };
                });
                Some(TypedValue {
                    ty: Type::Boolean,
                    value: syn::parse_quote!(#name),
                })
            }
            Expr::WitnessCall { name, arguments } => {
                let declaration = *self.witnesses.get(name.as_str())?;
                if arguments.len() != declaration.parameters.len()
                    || !(matches!(declaration.result, Type::Unit | Type::OpaqueBytes)
                        || recordable_cell_type(&declaration.result))
                {
                    return None;
                }
                let args = self.arguments(arguments, &declaration.parameters, scope, steps)?;
                let method = ident(name).ok()?;
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
            Expr::Call { name, arguments } => {
                let callee = *self.pure.get(name.as_str())?;
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
                if self.effectful_field_cells && *ty != Type::Field {
                    return None;
                }
                if !cell_type(ty)
                    && !(ty == &Type::Field
                        && (self.effectful_field_cells
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
                if !cell_type(ty)
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
                if *ty != crate::stateful::qualified_coin_type() {
                    return None;
                }
                let slot = ident(field).ok()?;
                steps.push(
                    syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;),
                );
                self.qualified_set_writes += 1;
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

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
) -> Option<TypedPlan> {
    let enum_entry = matches!(circuit.parameters.as_slice(), [parameter] if matches!(parameter.ty, Type::Enum { .. }));
    let opaque_entry =
        matches!(circuit.parameters.as_slice(), [parameter] if parameter.ty == Type::OpaqueString);
    let spend_entry = matches!(circuit.parameters.as_slice(), [destination, coin] if matches!(destination.ty, Type::Struct { .. }) && matches!(coin.ty, Type::Struct { .. }));
    let counter_entry = witnesses.is_empty()
        && circuit.parameters.iter().all(|parameter| {
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
        ) if witnesses.is_empty() && parameter.ty == Type::Field && name == &parameter.name => {
            match bindings.as_slice() {
                [
                    LocalBinding {
                        name: binding_name,
                        ty: Type::Field,
                        value: Expr::CellRead { field, index },
                    },
                ] if binding_name != &parameter.name => Some((field.clone(), *index)),
                _ => None,
            }
        }
        _ => None,
    };
    if !(circuit.parameters.is_empty()
        || enum_entry
        || opaque_entry
        || spend_entry
        || counter_entry
        || field_cell_slot.is_some())
        || (!matches!(
            circuit.result,
            Type::Unit | Type::OpaqueString | Type::Boolean
        ) && field_cell_slot.is_none())
    {
        return None;
    }
    let mut plan = Plan {
        ledger,
        witnesses,
        pure,
        next: 0,
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
    let mut return_scope = scope.clone();
    let mut steps = Vec::new();
    for (index, action) in circuit.actions.iter().enumerate() {
        // Only bindings belonging to the final top-level Let enclose the return.
        // Nested or earlier sibling Lets cannot escape their action scope.
        if index + 1 == circuit.actions.len()
            && let StateAction::Let { bindings, action } = action
        {
            return_scope = plan.bindings(bindings, &scope, &mut steps)?;
            plan.action(action, &return_scope, &mut steps)?;
        } else {
            plan.action(action, &scope, &mut steps)?;
        }
    }
    let result = match &circuit.return_value {
        StateReturn::Unit if circuit.result == Type::Unit => syn::parse_quote!(()),
        StateReturn::Expression { value } => {
            let result = plan.expression(value, &return_scope, &mut steps)?;
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
        && plan.counter_reads == 0
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
    (membership
        || cell_lifecycle
        || historic_spend
        || counter_comparison
        || qualified_set_lifecycle
        || qualified_cell_replacement
        || field_cell_root)
        .then_some(TypedPlan { steps, result })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            lower(&contract.stateful_circuits[0], &ledger, &witnesses, &pure).is_some()
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
