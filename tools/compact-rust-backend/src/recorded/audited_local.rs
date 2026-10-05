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

//! A closed recording boundary for native helpers with no public effects.
//! The native renderer owns expression semantics; this audit ensures a helper
//! cannot silently perform an unrecorded ledger operation before we call it.

use super::*;

fn local_expr(value: &Expr, witnesses: &HashMap<&str, &WitnessDeclaration>) -> bool {
    match value {
        Expr::Unit
        | Expr::Default { .. }
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::Parameter { .. }
        | Expr::EnumVariant { .. } => true,
        Expr::UnsignedCast { value, .. }
        | Expr::FieldCast { value }
        | Expr::FieldToBytes32 { value }
        | Expr::Coerce { value, .. }
        | Expr::StructField { value, .. }
        | Expr::TupleIndex { value, .. }
        | Expr::TransientHash { value }
        | Expr::PersistentHash { value }
        | Expr::Keccak256 { value }
        | Expr::DegradeToTransient { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::HashToCurve { value }
        | Expr::JubjubPointX { value }
        | Expr::JubjubPointY { value }
        | Expr::EcNeg { value }
        | Expr::JubjubScalarFromNative { value } => local_expr(value, witnesses),
        Expr::Tuple { elements } | Expr::Vector { elements, .. } => {
            elements.iter().all(|item| local_expr(item, witnesses))
        }
        Expr::StructLiteral { fields, .. } => fields.iter().all(|item| local_expr(item, witnesses)),
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            local_expr(condition, witnesses)
                && local_expr(then, witnesses)
                && local_expr(otherwise, witnesses)
        }
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .all(|binding| local_expr(&binding.value, witnesses))
                && local_expr(body, witnesses)
        }
        Expr::Sequence { steps, value } => {
            steps.iter().all(|step| local_expr(step, witnesses)) && local_expr(value, witnesses)
        }
        Expr::Assert { condition, .. } => local_expr(condition, witnesses),
        Expr::Equal { left, right }
        | Expr::NotEqual { left, right }
        | Expr::Compare { left, right, .. }
        | Expr::EcAdd { left, right }
        | Expr::ConstructJubjubPoint { x: left, y: right }
        | Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => {
            local_expr(left, witnesses) && local_expr(right, witnesses)
        }
        Expr::EcMul { point, scalar } => {
            local_expr(point, witnesses) && local_expr(scalar, witnesses)
        }
        Expr::TransientCommit { value, opening } => {
            local_expr(value, witnesses) && local_expr(opening, witnesses)
        }
        Expr::EcMulGenerator { scalar } => local_expr(scalar, witnesses),
        Expr::WitnessCall { name, arguments } => witnesses.get(name.as_str()).is_some_and(|decl| {
            arguments.len() == decl.parameters.len()
                && arguments
                    .iter()
                    .all(|argument| local_expr(argument, witnesses))
        }),
        // Pure calls and folds need their own transitive proof. Reject them,
        // including unknown names, until that audit exists.
        Expr::Call { .. }
        | Expr::VectorFoldCall { .. }
        | Expr::VectorMap { .. }
        | Expr::CreateZswapInput { .. }
        | Expr::CreateZswapOutput { .. }
        | Expr::NativeWitnessCall { .. }
        | Expr::SetMember { .. }
        | Expr::MapMember { .. }
        | Expr::MapLookup { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::ListHead { .. }
        | Expr::MerkleCheckRoot { .. }
        | Expr::HistoricMerkleCheckRoot { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. }
        | Expr::CounterLessThan { .. }
        | Expr::KernelSelf { .. }
        | Expr::SetSize { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::PersistentCommit { .. } => false,
    }
}

fn local_action(
    action: &StateAction,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> bool {
    match action {
        StateAction::Sequence { actions } => actions
            .iter()
            .all(|action| local_action(action, witnesses, circuits, visiting)),
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            local_expr(condition, witnesses)
                && local_action(then, witnesses, circuits, visiting)
                && local_action(otherwise, witnesses, circuits, visiting)
        }
        StateAction::Expression { value } => local_expr(value, witnesses),
        StateAction::Assert { condition, .. } => local_expr(condition, witnesses),
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .all(|binding| local_expr(&binding.value, witnesses))
                && local_action(action, witnesses, circuits, visiting)
        }
        StateAction::CircuitCall { name, arguments } => {
            let Some(callee) = circuits.get(name.as_str()) else {
                return false;
            };
            if !visiting.insert(name.clone()) {
                return false;
            }
            let accepted = callee.result == Type::Unit
                && callee.return_value == StateReturn::Unit
                && arguments.len() == callee.parameters.len()
                && arguments
                    .iter()
                    .all(|argument| local_expr(argument, witnesses))
                && callee
                    .actions
                    .iter()
                    .all(|action| local_action(action, witnesses, circuits, visiting));
            visiting.remove(name);
            accepted
        }
        // Every public operation, native witness built-in, and unproven pure
        // call is outside this boundary. Keep this match exhaustive.
        _ => false,
    }
}

fn uncoerced<'a>(value: &'a Expr, ty: &Type) -> Option<&'a Expr> {
    match value {
        Expr::Coerce { value, ty: actual } if actual == ty => Some(value),
        _ => None,
    }
}

pub(super) fn render(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<RecordingOutcome<syn::Item>>, RenderError> {
    if circuit.internal
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || circuit.parameters.len() != 2
    {
        return Ok(None);
    }
    let [
        StateAction::Assert {
            condition:
                Expr::CellRead {
                    field: guard,
                    index: guard_index,
                },
            message,
        },
        StateAction::CircuitCall {
            name: callee_name,
            arguments,
        },
        tail @ ..,
    ] = circuit.actions.as_slice()
    else {
        return Ok(None);
    };
    let Some(callee) = circuits.get(callee_name.as_str()) else {
        return Ok(None);
    };
    if callee.parameters.len() != 3
        || arguments.len() != 3
        || callee.result != Type::Unit
        || callee.return_value != StateReturn::Unit
    {
        return Ok(None);
    }
    let [first, second, third] = arguments.as_slice() else {
        return Ok(None);
    };
    let [p0, p1] = circuit.parameters.as_slice() else {
        return Ok(None);
    };
    let [c0, c1, c2] = callee.parameters.as_slice() else {
        return Ok(None);
    };
    let first_ok = matches!(uncoerced(first, &c0.ty), Some(Expr::Parameter { name }) if name == &p0.name && p0.ty == c0.ty);
    let second_ok = matches!(uncoerced(second, &c1.ty), Some(Expr::Parameter { name }) if name == &p1.name && p1.ty == c1.ty);
    let Some(Expr::CellRead {
        field: key,
        index: key_index,
    }) = uncoerced(third, &c2.ty)
    else {
        return Ok(None);
    };
    if !first_ok
        || !second_ok
        || c2.ty != Type::JubjubPoint
        || !ledger_fields.get(guard.as_str()).is_some_and(|decl| {
            decl.index == *guard_index
                && decl.physical_path().len() == 1
                && matches!(
                    decl.declaration,
                    LedgerFieldKind::Cell { ty: Type::Boolean }
                )
        })
        || !ledger_fields.get(key.as_str()).is_some_and(|decl| {
            decl.index == *key_index
                && decl.physical_path().len() == 1
                && matches!(
                    decl.declaration,
                    LedgerFieldKind::Cell {
                        ty: Type::JubjubPoint
                    }
                )
        })
    {
        return Ok(None);
    }
    let mut visiting = HashSet::from([circuit.name.clone(), callee_name.clone()]);
    if !callee
        .actions
        .iter()
        .all(|action| local_action(action, witnesses, circuits, &mut visiting))
    {
        return Ok(Some(RecordingOutcome::Unsupported(RecordingGap::action(
            &circuit.actions[1],
            "actions[1]".to_owned(),
        ))));
    }
    let counter = match tail {
        [] => None,
        [StateAction::Let { bindings, action }] => {
            let [binding] = bindings.as_slice() else {
                return Ok(None);
            };
            let StateAction::CounterIncrement {
                field,
                index,
                amount: CounterAmount::Parameter { name },
            } = action.as_ref()
            else {
                return Ok(None);
            };
            if binding.name != *name
                || binding.ty
                    != (Type::Unsigned {
                        max: "65535".into(),
                    })
                || !matches!(&binding.value, Expr::UnsignedLiteral { value, max } if value == "1" && max == "65535")
                || !ledger_fields.get(field.as_str()).is_some_and(|decl| {
                    decl.index == *index
                        && decl.physical_path().len() == 1
                        && matches!(decl.declaration, LedgerFieldKind::Counter)
                })
            {
                return Ok(None);
            }
            Some(ident(field)?)
        }
        _ => return Ok(None),
    };
    let fn_name = ident(&circuit.name)?;
    let callee_name = ident(callee_name)?;
    let guard = ident(guard)?;
    let key = ident(key)?;
    let ty0 = rust_type(&p0.ty)?;
    let ty1 = rust_type(&p1.ty)?;
    let p0 = syn::Ident::new("__compact_param_0", Span::call_site());
    let p1 = syn::Ident::new("__compact_param_1", Span::call_site());
    let counter_step: Vec<syn::Stmt> = counter
        .map(|slot| {
            syn::parse_quote!(
                let frame = crate::ledger_slots::#slot.record_increment(frame, 1_u16)?;
            )
        })
        .into_iter()
        .collect();
    let item = syn::parse_quote! {
        pub fn #fn_name<Private, W: super::TryWitnesses<Private>>(
            context: runtime::context::CircuitContext<Private>,
            witnesses: &W,
            #p0: #ty0,
            #p1: #ty1,
        ) -> Result<runtime::recording::RecordedCircuitResult<Private, ()>, runtime::CompactError> {
            let frame = runtime::recording::RecordingFrame::new(context);
            let (frame, open): (_, bool) = crate::ledger_slots::#guard.record_read(frame)?;
            if !open {
                return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
            }
            let (frame, public_key): (_, runtime::JubjubPoint) =
                crate::ledger_slots::#key.record_read(frame)?;
            let (frame, ()) = frame.call_local(|context| {
                super::#callee_name(context, witnesses, #p0, #p1, public_key)
            })?;
            #(#counter_step)*
            Ok(frame.finish(()))
        }
    };
    Ok(Some(RecordingOutcome::Supported(item)))
}
