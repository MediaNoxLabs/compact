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

//! IR-only call dependencies, user-witness requirements and native private outputs.
//!
//! These are separate facts: a native builtin can emit a private transcript
//! value without requiring the user's witness interface. Declaration validation
//! and syntax emission remain with their existing owners.

use crate::RenderError;
use crate::ir::{Expr, ReturnPlan, StateAction, StateReturn, StatefulCircuit};
use std::collections::{HashMap, HashSet};

fn action_calls_named(action: &StateAction, name: &str) -> bool {
    match action {
        StateAction::CircuitCall {
            name: callee,
            arguments,
        } => {
            callee == name
                || arguments
                    .iter()
                    .any(|arg| expression_calls_named(arg, name))
        }
        StateAction::PureCall { arguments, .. } => arguments
            .iter()
            .any(|arg| expression_calls_named(arg, name)),
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .any(|binding| expression_calls_named(&binding.value, name))
                || action_calls_named(action, name)
        }
        StateAction::Sequence { actions } => actions
            .iter()
            .any(|action| action_calls_named(action, name)),
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            expression_calls_named(condition, name)
                || action_calls_named(then, name)
                || action_calls_named(otherwise, name)
        }
        StateAction::Expression { value }
        | StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::MerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::MapInsertDefault { key: value, .. }
        | StateAction::MapRemove { key: value, .. } => expression_calls_named(value, name),
        StateAction::Assert { condition, .. } => expression_calls_named(condition, name),
        StateAction::MapInsert { key, value, .. } => {
            expression_calls_named(key, name) || expression_calls_named(value, name)
        }
        StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        } => expression_calls_named(coin, name) || expression_calls_named(recipient, name),
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => expression_calls_named(value, name) || expression_calls_named(position, name),
        StateAction::CounterIncrement { .. }
        | StateAction::NativeWitnessCall { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapReset { .. }
        | StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. } => false,
    }
}

fn expression_calls_named(expression: &Expr, name: &str) -> bool {
    expression_contains(
        expression,
        &|value| matches!(value, Expr::Call { name: callee, .. } if callee == name),
    )
}

fn return_calls_named(value: &StateReturn, name: &str) -> bool {
    match value {
        StateReturn::Effectful { body } => plan_calls_named(body, name),
        StateReturn::Expression { value }
        | StateReturn::SetMember { value, .. }
        | StateReturn::HistoricMerkleCheckRoot { root: value, .. }
        | StateReturn::MerkleCheckRoot { root: value, .. } => expression_calls_named(value, name),
        StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
            expression_calls_named(key, name)
        }
        _ => false,
    }
}

fn plan_calls_named(plan: &ReturnPlan, name: &str) -> bool {
    match plan {
        ReturnPlan::Value { value } => expression_calls_named(value, name),
        ReturnPlan::Sequence { actions, result } => {
            actions
                .iter()
                .any(|action| action_calls_named(action, name))
                || plan_calls_named(result, name)
        }
        ReturnPlan::Let { bindings, result } => {
            bindings
                .iter()
                .any(|binding| expression_calls_named(&binding.value, name))
                || plan_calls_named(result, name)
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            expression_calls_named(condition, name)
                || plan_calls_named(then, name)
                || plan_calls_named(otherwise, name)
        }
    }
}

pub(crate) fn circuit_uses_witness(
    circuit: &StatefulCircuit,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> Result<bool, RenderError> {
    if !visiting.insert(circuit.name.clone()) {
        return Err(RenderError::UnsupportedStatefulCall(circuit.name.clone()));
    }
    let mut effect = circuit_contains_witness(circuit);
    for (name, callee) in circuits {
        if circuit
            .actions
            .iter()
            .any(|action| action_calls_named(action, name))
            || return_calls_named(&circuit.return_value, name)
        {
            effect |= circuit_uses_witness(callee, circuits, visiting)?;
        }
    }
    visiting.remove(&circuit.name);
    Ok(effect)
}

fn action_emits_native_private_output(action: &StateAction) -> bool {
    match action {
        StateAction::NativeWitnessCall { .. } => true,
        StateAction::Sequence { actions } => actions.iter().any(action_emits_native_private_output),
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            expression_contains_native_witness(condition)
                || action_emits_native_private_output(then)
                || action_emits_native_private_output(otherwise)
        }
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .any(|binding| expression_contains_native_witness(&binding.value))
                || action_emits_native_private_output(action)
        }
        StateAction::Expression { value }
        | StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::MerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::MapInsertDefault { key: value, .. }
        | StateAction::MapRemove { key: value, .. } => expression_contains_native_witness(value),
        StateAction::PureCall { arguments, .. } | StateAction::CircuitCall { arguments, .. } => {
            arguments.iter().any(expression_contains_native_witness)
        }
        StateAction::Assert { condition, .. } => expression_contains_native_witness(condition),
        StateAction::MapInsert { key, value, .. } => {
            expression_contains_native_witness(key) || expression_contains_native_witness(value)
        }
        StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        } => {
            expression_contains_native_witness(coin)
                || expression_contains_native_witness(recipient)
        }
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => {
            expression_contains_native_witness(value)
                || expression_contains_native_witness(position)
        }
        _ => false,
    }
}

fn expression_contains_native_witness(expression: &Expr) -> bool {
    expression_contains(expression, &|value| {
        matches!(
            value,
            Expr::NativeWitnessCall { .. }
                | Expr::CreateZswapInput { .. }
                | Expr::CreateZswapOutput { .. }
        )
    })
}

fn return_contains_native_witness(value: &StateReturn) -> bool {
    match value {
        StateReturn::Effectful { body } => plan_contains_native_witness(body),
        StateReturn::Expression { value }
        | StateReturn::SetMember { value, .. }
        | StateReturn::HistoricMerkleCheckRoot { root: value, .. }
        | StateReturn::MerkleCheckRoot { root: value, .. } => {
            expression_contains_native_witness(value)
        }
        StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
            expression_contains_native_witness(key)
        }
        _ => false,
    }
}

fn plan_contains_native_witness(plan: &ReturnPlan) -> bool {
    match plan {
        ReturnPlan::Value { value } => expression_contains_native_witness(value),
        ReturnPlan::Sequence { actions, result } => {
            actions.iter().any(action_emits_native_private_output)
                || plan_contains_native_witness(result)
        }
        ReturnPlan::Let { bindings, result } => {
            bindings
                .iter()
                .any(|binding| expression_contains_native_witness(&binding.value))
                || plan_contains_native_witness(result)
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            expression_contains_native_witness(condition)
                || plan_contains_native_witness(then)
                || plan_contains_native_witness(otherwise)
        }
    }
}

pub(crate) fn circuit_emits_native_private_output(
    circuit: &StatefulCircuit,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
) -> Result<bool, RenderError> {
    if !visiting.insert(circuit.name.clone()) {
        return Err(RenderError::UnsupportedStatefulCall(circuit.name.clone()));
    }
    let mut effect = circuit
        .actions
        .iter()
        .any(action_emits_native_private_output)
        || return_contains_native_witness(&circuit.return_value);
    for (name, callee) in circuits {
        if circuit
            .actions
            .iter()
            .any(|action| action_calls_named(action, name))
            || return_calls_named(&circuit.return_value, name)
        {
            effect |= circuit_emits_native_private_output(callee, circuits, visiting)?;
        }
    }
    visiting.remove(&circuit.name);
    Ok(effect)
}

fn expression_contains(expression: &Expr, predicate: &impl Fn(&Expr) -> bool) -> bool {
    if predicate(expression) {
        return true;
    }
    let visit = |child: &Expr| expression_contains(child, predicate);
    match expression {
        Expr::WitnessCall { arguments, .. } | Expr::Call { arguments, .. } => {
            arguments.iter().any(visit)
        }
        Expr::Tuple { elements }
        | Expr::Vector { elements, .. }
        | Expr::StructLiteral {
            fields: elements, ..
        } => elements.iter().any(visit),
        Expr::VectorMap { source, body, .. } => visit(source) || visit(body),
        Expr::VectorFoldCall {
            initial, source, ..
        } => visit(initial) || visit(source),
        Expr::If {
            condition,
            then,
            otherwise,
        } => visit(condition) || visit(then) || visit(otherwise),
        Expr::Let { bindings, body } => {
            bindings.iter().any(|binding| visit(&binding.value)) || visit(body)
        }
        Expr::Sequence { steps, value } => steps.iter().any(visit) || visit(value),
        Expr::UnsignedCast { value, .. }
        | Expr::FieldCast { value }
        | Expr::FieldToBytes32 { value }
        | Expr::Coerce { value, .. }
        | Expr::StructField { value, .. }
        | Expr::TupleIndex { value, .. }
        | Expr::SetMember { value, .. }
        | Expr::MapMember { key: value, .. }
        | Expr::MapLookup { key: value, .. }
        | Expr::MerkleCheckRoot { root: value, .. }
        | Expr::HistoricMerkleCheckRoot { root: value, .. }
        | Expr::Assert {
            condition: value, ..
        }
        | Expr::TransientHash { value }
        | Expr::PersistentHash { value }
        | Expr::Keccak256 { value }
        | Expr::DegradeToTransient { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::HashToCurve { value }
        | Expr::JubjubPointX { value }
        | Expr::JubjubPointY { value }
        | Expr::EcNeg { value }
        | Expr::EcMulGenerator { scalar: value }
        | Expr::JubjubScalarFromNative { value } => visit(value),
        Expr::Equal { left, right }
        | Expr::NotEqual { left, right }
        | Expr::Compare { left, right, .. }
        | Expr::TransientCommit {
            value: left,
            opening: right,
        }
        | Expr::PersistentCommit {
            value: left,
            opening: right,
        }
        | Expr::EcAdd { left, right }
        | Expr::ConstructJubjubPoint { x: left, y: right }
        | Expr::EcMul {
            point: left,
            scalar: right,
        }
        | Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => visit(left) || visit(right),
        Expr::CounterLessThan { threshold, .. } => visit(threshold),
        Expr::KernelClaim { value, .. } => visit(value),
        Expr::KernelMintShielded { domain, amount } => visit(domain) || visit(amount),
        Expr::CreateZswapInput { coin } => visit(coin),
        Expr::CreateZswapOutput { coin, recipient } => visit(coin) || visit(recipient),
        Expr::Unit
        | Expr::NativeWitnessCall { .. }
        | Expr::Default { .. }
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Parameter { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. }
        | Expr::KernelSelf { .. }
        | Expr::SetSize { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::ListHead { .. } => false,
    }
}

fn expression_contains_witness(expression: &Expr) -> bool {
    expression_contains(expression, &|value| {
        matches!(value, Expr::WitnessCall { .. })
    })
}

pub(crate) fn expression_requires_witness(
    expression: &Expr,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<bool, RenderError> {
    if expression_contains_witness(expression) {
        return Ok(true);
    }
    for (name, callee) in stateful_circuits {
        if expression_calls_named(expression, name)
            && circuit_uses_witness(callee, stateful_circuits, &mut HashSet::new())?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn expression_contains_stateful_call(
    expression: &Expr,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    expression_contains(
        expression,
        &|value| matches!(value, Expr::Call { name, .. } if stateful_circuits.contains_key(name.as_str())),
    )
}

fn action_contains_witness(action: &StateAction) -> bool {
    match action {
        StateAction::Sequence { actions } => actions.iter().any(action_contains_witness),
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            expression_contains_witness(condition)
                || action_contains_witness(then)
                || action_contains_witness(otherwise)
        }
        StateAction::Expression { value }
        | StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::MerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsertIndexDefault {
            position: value, ..
        }
        | StateAction::MapInsertDefault { key: value, .. }
        | StateAction::MapRemove { key: value, .. } => expression_contains_witness(value),
        StateAction::PureCall { arguments, .. } | StateAction::CircuitCall { arguments, .. } => {
            arguments.iter().any(expression_contains_witness)
        }
        StateAction::Assert { condition, .. } => expression_contains_witness(condition),
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .any(|binding| expression_contains_witness(&binding.value))
                || action_contains_witness(action)
        }
        StateAction::MapInsert { key, value, .. } => {
            expression_contains_witness(key) || expression_contains_witness(value)
        }
        StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        } => expression_contains_witness(coin) || expression_contains_witness(recipient),
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => expression_contains_witness(value) || expression_contains_witness(position),
        StateAction::CounterIncrement { .. }
        | StateAction::NativeWitnessCall { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapReset { .. }
        | StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. } => false,
    }
}

pub(crate) fn circuit_contains_witness(circuit: &StatefulCircuit) -> bool {
    circuit.actions.iter().any(action_contains_witness)
        || match &circuit.return_value {
            StateReturn::Effectful { body } => plan_contains_witness(body),
            StateReturn::Expression { value }
            | StateReturn::SetMember { value, .. }
            | StateReturn::HistoricMerkleCheckRoot { root: value, .. }
            | StateReturn::MerkleCheckRoot { root: value, .. } => {
                expression_contains_witness(value)
            }
            StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
                expression_contains_witness(key)
            }
            _ => false,
        }
}

fn plan_contains_witness(plan: &ReturnPlan) -> bool {
    match plan {
        ReturnPlan::Value { value } => expression_contains_witness(value),
        ReturnPlan::Sequence { actions, result } => {
            actions.iter().any(action_contains_witness) || plan_contains_witness(result)
        }
        ReturnPlan::Let { bindings, result } => {
            bindings
                .iter()
                .any(|binding| expression_contains_witness(&binding.value))
                || plan_contains_witness(result)
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            expression_contains_witness(condition)
                || plan_contains_witness(then)
                || plan_contains_witness(otherwise)
        }
    }
}

#[cfg(test)]
mod tests;
