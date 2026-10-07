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

//! Exhaustive borrowed IR traversal. Additions to IR shapes require a visitor review.
use super::{LimitError, Scan, Visit};
use crate::ir::*;
#[derive(Clone, Copy)]
pub(super) enum Node<'a> {
    SourceLocation(&'a SourceLocation),
    Contract(&'a Contract),
    TypeAlias(&'a TypeAlias),
    Constructor(&'a Constructor),
    ConstructorStep(&'a ConstructorStep),
    WitnessDeclaration(&'a WitnessDeclaration),
    LedgerField(&'a LedgerField),
    LedgerFieldKind(&'a LedgerFieldKind),
    StatefulCircuit(&'a StatefulCircuit),
    StateReturn(&'a StateReturn),
    ReturnPlan(&'a ReturnPlan),
    StateAction(&'a StateAction),
    NativeWitnessBuiltin(&'a NativeWitnessBuiltin),
    CounterAmount(&'a CounterAmount),
    PureCircuit(&'a PureCircuit),
    Parameter(&'a Parameter),
    LocalBinding(&'a LocalBinding),
    Type(&'a Type),
    StructField(&'a StructField),
    KernelClaimKind(&'a KernelClaimKind),
    Expr(&'a Expr),
    ComparisonOperator(&'a ComparisonOperator),
    Text(&'a str),
}
pub(super) fn children<'a>(visit: Visit<'a>, scan: &mut Scan<'a>) -> Result<(), LimitError> {
    let Visit {
        node,
        depth: visit_depth,
        owner,
        ..
    } = visit;
    match node {
        Node::Text(s) => scan.text(s)?,
        Node::SourceLocation(value) => {
            let SourceLocation { file, line, column } = value;
            scan.push(Node::Text(file), visit_depth + 1, owner)?;
            let _ = line;
            let _ = column;
        }
        Node::Contract(c) => {
            let Contract {
                schema_version,
                type_aliases,
                ledger_fields,
                constructor,
                witnesses,
                circuits,
                stateful_circuits,
            } = c;
            scan.reserve(
                circuits
                    .len()
                    .saturating_add(stateful_circuits.len())
                    .saturating_add(usize::from(constructor.is_some())),
            )?;
            for (i, c) in circuits.iter().enumerate() {
                scan.push(Node::PureCircuit(c), 1, Some(i))?;
            }
            for (i, c) in stateful_circuits.iter().enumerate() {
                scan.push(Node::StatefulCircuit(c), 1, Some(scan.pure_count + i))?;
            }
            if let Some(c) = constructor {
                scan.push(Node::Constructor(c), 1, Some(scan.callable_count - 1))?;
            }
            let _ = schema_version;
            scan.reserve(type_aliases.len())?;
            for child in type_aliases {
                scan.push(Node::TypeAlias(child), visit_depth + 1, owner)?;
            }
            scan.reserve(ledger_fields.len())?;
            for child in ledger_fields {
                scan.push(Node::LedgerField(child), visit_depth + 1, owner)?;
            }
            scan.reserve(witnesses.len())?;
            for child in witnesses {
                scan.push(Node::WitnessDeclaration(child), visit_depth + 1, owner)?;
            }
        }
        Node::TypeAlias(value) => {
            let TypeAlias { source, name, ty } = value;
            if let Some(child) = source {
                scan.push(Node::SourceLocation(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            scan.push(Node::Type(ty), visit_depth + 1, owner)?;
        }
        Node::Constructor(value) => {
            let Constructor {
                source,
                parameters,
                steps,
            } = value;
            if let Some(child) = source {
                scan.push(Node::SourceLocation(child), visit_depth + 1, owner)?;
            }
            scan.reserve(parameters.len())?;
            for child in parameters {
                scan.push(Node::Parameter(child), visit_depth + 1, owner)?;
            }
            scan.reserve(steps.len())?;
            for child in steps {
                scan.push(Node::ConstructorStep(child), visit_depth + 1, owner)?;
            }
        }
        Node::ConstructorStep(value) => match value {
            ConstructorStep::Expression { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ConstructorStep::Let { bindings, step } => {
                scan.reserve(bindings.len())?;
                for child in bindings {
                    scan.push(Node::LocalBinding(child), visit_depth + 1, owner)?;
                }
                scan.push(Node::ConstructorStep(step), visit_depth + 1, owner)?;
            }
            ConstructorStep::Sequence { steps } => {
                scan.reserve(steps.len())?;
                for child in steps {
                    scan.push(Node::ConstructorStep(child), visit_depth + 1, owner)?;
                }
            }
            ConstructorStep::Assert { condition, message } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.push(Node::Text(message), visit_depth + 1, owner)?;
            }
            ConstructorStep::CellWrite {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ConstructorStep::CounterIncrement {
                field,
                index,
                amount,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::CounterAmount(amount), visit_depth + 1, owner)?;
            }
            ConstructorStep::CounterDecrement {
                field,
                index,
                amount,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::CounterAmount(amount), visit_depth + 1, owner)?;
            }
            ConstructorStep::CounterReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            ConstructorStep::SetInsert {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ConstructorStep::SetRemove {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ConstructorStep::SetReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            ConstructorStep::ListPushFront {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ConstructorStep::ListPopFront { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            ConstructorStep::ListReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            ConstructorStep::MapInsert {
                field,
                index,
                key,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ConstructorStep::MapInsertDefault { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            ConstructorStep::MapRemove { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            ConstructorStep::MapReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            ConstructorStep::ForEach {
                binding,
                values,
                steps,
            } => {
                scan.push(Node::Parameter(binding), visit_depth + 1, owner)?;
                scan.reserve(values.len())?;
                for child in values {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
                scan.reserve(steps.len())?;
                for child in steps {
                    scan.push(Node::ConstructorStep(child), visit_depth + 1, owner)?;
                }
            }
            ConstructorStep::ForEachVector {
                binding,
                source,
                length,
                steps,
            } => {
                scan.push(Node::Parameter(binding), visit_depth + 1, owner)?;
                scan.push(Node::Expr(source), visit_depth + 1, owner)?;
                let _ = length;
                scan.reserve(steps.len())?;
                for child in steps {
                    scan.push(Node::ConstructorStep(child), visit_depth + 1, owner)?;
                }
            }
            ConstructorStep::If {
                condition,
                then_steps,
                otherwise_steps,
            } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.reserve(then_steps.len())?;
                for child in then_steps {
                    scan.push(Node::ConstructorStep(child), visit_depth + 1, owner)?;
                }
                scan.reserve(otherwise_steps.len())?;
                for child in otherwise_steps {
                    scan.push(Node::ConstructorStep(child), visit_depth + 1, owner)?;
                }
            }
        },
        Node::WitnessDeclaration(value) => {
            let WitnessDeclaration {
                source,
                name,
                parameters,
                result,
            } = value;
            if let Some(child) = source {
                scan.push(Node::SourceLocation(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            scan.reserve(parameters.len())?;
            for child in parameters {
                scan.push(Node::Parameter(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Type(result), visit_depth + 1, owner)?;
        }
        Node::LedgerField(value) => {
            let LedgerField {
                id,
                index,
                path,
                source,
                declaration,
            } = value;
            scan.push(Node::Text(id), visit_depth + 1, owner)?;
            let _ = index;
            scan.literal_bytes(path.len())?;
            if let Some(child) = source {
                scan.push(Node::SourceLocation(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::LedgerFieldKind(declaration), visit_depth + 1, owner)?;
        }
        Node::LedgerFieldKind(value) => match value {
            LedgerFieldKind::Counter => {}
            LedgerFieldKind::Cell { ty } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            LedgerFieldKind::Set { ty } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            LedgerFieldKind::List { ty } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            LedgerFieldKind::Map { key, value } => {
                scan.push(Node::Type(key), visit_depth + 1, owner)?;
                scan.push(Node::Type(value), visit_depth + 1, owner)?;
            }
            LedgerFieldKind::MerkleTree { depth, ty } => {
                let _ = depth;
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            LedgerFieldKind::HistoricMerkleTree { depth, ty } => {
                let _ = depth;
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
        },
        Node::StatefulCircuit(value) => {
            let StatefulCircuit {
                source,
                name,
                internal,
                parameters,
                actions,
                result,
                return_value,
            } = value;
            if let Some(child) = source {
                scan.push(Node::SourceLocation(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            let _ = internal;
            scan.reserve(parameters.len())?;
            for child in parameters {
                scan.push(Node::Parameter(child), visit_depth + 1, owner)?;
            }
            scan.reserve(actions.len())?;
            for child in actions {
                scan.push(Node::StateAction(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Type(result), visit_depth + 1, owner)?;
            scan.push(Node::StateReturn(return_value), visit_depth + 1, owner)?;
        }
        Node::StateReturn(value) => match value {
            StateReturn::Unit => {}
            StateReturn::Expression { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateReturn::Effectful { body } => {
                scan.push(Node::ReturnPlan(body), visit_depth + 1, owner)?;
            }
            StateReturn::CellRead { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::CounterRead { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::SetMember {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateReturn::SetSize { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::SetIsEmpty { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::MapMember { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            StateReturn::MapLookup { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            StateReturn::MapSize { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::MapIsEmpty { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::ListLength { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::ListIsEmpty { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::ListHead { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::HistoricMerkleIsFull { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::HistoricMerkleCheckRoot { field, index, root } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(root), visit_depth + 1, owner)?;
            }
            StateReturn::MerkleIsFull { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateReturn::MerkleCheckRoot { field, index, root } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(root), visit_depth + 1, owner)?;
            }
        },
        Node::ReturnPlan(value) => match value {
            ReturnPlan::Value { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            ReturnPlan::Sequence { actions, result } => {
                scan.reserve(actions.len())?;
                for child in actions {
                    scan.push(Node::StateAction(child), visit_depth + 1, owner)?;
                }
                scan.push(Node::ReturnPlan(result), visit_depth + 1, owner)?;
            }
            ReturnPlan::Let { bindings, result } => {
                scan.reserve(bindings.len())?;
                for child in bindings {
                    scan.push(Node::LocalBinding(child), visit_depth + 1, owner)?;
                }
                scan.push(Node::ReturnPlan(result), visit_depth + 1, owner)?;
            }
            ReturnPlan::Conditional {
                condition,
                then,
                otherwise,
            } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.push(Node::ReturnPlan(then), visit_depth + 1, owner)?;
                scan.push(Node::ReturnPlan(otherwise), visit_depth + 1, owner)?;
            }
        },
        Node::StateAction(value) => match value {
            StateAction::Sequence { actions } => {
                scan.reserve(actions.len())?;
                for child in actions {
                    scan.push(Node::StateAction(child), visit_depth + 1, owner)?;
                }
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.push(Node::StateAction(then), visit_depth + 1, owner)?;
                scan.push(Node::StateAction(otherwise), visit_depth + 1, owner)?;
            }
            StateAction::Expression { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::PureCall { name, arguments } => {
                scan.call(owner, name, visit_depth);
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.reserve(arguments.len())?;
                for child in arguments {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            StateAction::CircuitCall { name, arguments } => {
                scan.call(owner, name, visit_depth);
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.reserve(arguments.len())?;
                for child in arguments {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            StateAction::NativeWitnessCall { builtin } => {
                scan.push(Node::NativeWitnessBuiltin(builtin), visit_depth + 1, owner)?;
            }
            StateAction::Assert { condition, message } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.push(Node::Text(message), visit_depth + 1, owner)?;
            }
            StateAction::Let { bindings, action } => {
                scan.reserve(bindings.len())?;
                for child in bindings {
                    scan.push(Node::LocalBinding(child), visit_depth + 1, owner)?;
                }
                scan.push(Node::StateAction(action), visit_depth + 1, owner)?;
            }
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::CounterAmount(amount), visit_depth + 1, owner)?;
            }
            StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::CounterAmount(amount), visit_depth + 1, owner)?;
            }
            StateAction::CounterReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::SetInsert {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::CellWriteCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(coin), visit_depth + 1, owner)?;
                scan.push(Node::Expr(recipient), visit_depth + 1, owner)?;
            }
            StateAction::SetInsertCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(coin), visit_depth + 1, owner)?;
                scan.push(Node::Expr(recipient), visit_depth + 1, owner)?;
            }
            StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::SetReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::ListPushFront {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::ListPopFront { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::ListReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::MapInsert {
                field,
                index,
                key,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::MapInsertDefault { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            StateAction::MapRemove { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            StateAction::MapReset { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::HistoricMerkleInsertIndexDefault {
                field,
                index,
                position,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(position), visit_depth + 1, owner)?;
            }
            StateAction::HistoricMerkleInsert {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::HistoricMerkleInsertIndex {
                field,
                index,
                value,
                position,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                scan.push(Node::Expr(position), visit_depth + 1, owner)?;
            }
            StateAction::HistoricMerkleInsertHash { field, index, hash } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(hash), visit_depth + 1, owner)?;
            }
            StateAction::HistoricMerkleInsertHashIndex {
                field,
                index,
                hash,
                position,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(hash), visit_depth + 1, owner)?;
                scan.push(Node::Expr(position), visit_depth + 1, owner)?;
            }
            StateAction::HistoricMerkleResetHistory { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::HistoricMerkleResetToDefault { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            StateAction::MerkleInsertIndexDefault {
                field,
                index,
                position,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(position), visit_depth + 1, owner)?;
            }
            StateAction::MerkleInsert {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            StateAction::MerkleInsertIndex {
                field,
                index,
                value,
                position,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                scan.push(Node::Expr(position), visit_depth + 1, owner)?;
            }
            StateAction::MerkleInsertHash { field, index, hash } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(hash), visit_depth + 1, owner)?;
            }
            StateAction::MerkleInsertHashIndex {
                field,
                index,
                hash,
                position,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(hash), visit_depth + 1, owner)?;
                scan.push(Node::Expr(position), visit_depth + 1, owner)?;
            }
            StateAction::MerkleResetToDefault { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
        },
        Node::NativeWitnessBuiltin(value) => match value {
            NativeWitnessBuiltin::OwnPublicKey => {}
        },
        Node::CounterAmount(value) => match value {
            CounterAmount::Literal { value } => {
                let _ = value;
            }
            CounterAmount::Parameter { name } => {
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
            }
        },
        Node::PureCircuit(value) => {
            let PureCircuit {
                source,
                name,
                internal,
                parameters,
                result,
                body,
            } = value;
            if let Some(child) = source {
                scan.push(Node::SourceLocation(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            let _ = internal;
            scan.reserve(parameters.len())?;
            for child in parameters {
                scan.push(Node::Parameter(child), visit_depth + 1, owner)?;
            }
            scan.push(Node::Type(result), visit_depth + 1, owner)?;
            scan.push(Node::Expr(body), visit_depth + 1, owner)?;
        }
        Node::Parameter(value) => {
            let Parameter { name, ty } = value;
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            scan.push(Node::Type(ty), visit_depth + 1, owner)?;
        }
        Node::LocalBinding(value) => {
            let LocalBinding { name, ty, value } = value;
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            scan.push(Node::Expr(value), visit_depth + 1, owner)?;
        }
        Node::Type(value) => match value {
            Type::Unit => {}
            Type::Boolean => {}
            Type::Field => {}
            Type::JubjubPoint => {}
            Type::OpaqueString => {}
            Type::OpaqueBytes => {}
            Type::Bytes { length } => {
                let _ = length;
            }
            Type::Struct { name, fields } => {
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.reserve(fields.len())?;
                for child in fields {
                    scan.push(Node::StructField(child), visit_depth + 1, owner)?;
                }
            }
            Type::Enum { name, variants } => {
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.reserve(variants.len())?;
                for child in variants {
                    scan.push(Node::Text(child), visit_depth + 1, owner)?;
                }
            }
            Type::Unsigned { max } => {
                scan.push(Node::Text(max), visit_depth + 1, owner)?;
            }
            Type::Tuple { elements } => {
                scan.reserve(elements.len())?;
                for child in elements {
                    scan.push(Node::Type(child), visit_depth + 1, owner)?;
                }
            }
            Type::Vector { element, length } => {
                scan.push(Node::Type(element), visit_depth + 1, owner)?;
                let _ = length;
            }
            Type::LedgerMap { key, value } => {
                scan.push(Node::Type(key), visit_depth + 1, owner)?;
                scan.push(Node::Type(value), visit_depth + 1, owner)?;
            }
        },
        Node::StructField(value) => {
            let StructField { name, ty } = value;
            scan.push(Node::Text(name), visit_depth + 1, owner)?;
            scan.push(Node::Type(ty), visit_depth + 1, owner)?;
        }
        Node::KernelClaimKind(value) => match value {
            KernelClaimKind::Nullifier => {}
            KernelClaimKind::CoinSpend => {}
            KernelClaimKind::CoinReceive => {}
        },
        Node::Expr(value) => match value {
            Expr::Unit => {}
            Expr::Default { ty } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            Expr::Boolean { value } => {
                let _ = value;
            }
            Expr::FieldLiteral { value } => {
                scan.push(Node::Text(value), visit_depth + 1, owner)?;
            }
            Expr::BytesLiteral { bytes } => {
                scan.literal_bytes(bytes.len())?;
            }
            Expr::UnsignedLiteral { value, max } => {
                scan.push(Node::Text(value), visit_depth + 1, owner)?;
                scan.push(Node::Text(max), visit_depth + 1, owner)?;
            }
            Expr::UnsignedCast { max, value } => {
                scan.push(Node::Text(max), visit_depth + 1, owner)?;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::FieldCast { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::FieldToBytes32 { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::Coerce { value, ty } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            Expr::Parameter { name } => {
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
            }
            Expr::EnumVariant { ty, variant } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
                scan.push(Node::Text(variant), visit_depth + 1, owner)?;
            }
            Expr::StructField {
                value,
                field,
                index,
            } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::TupleIndex { value, index } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::StructLiteral { ty, fields } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
                scan.reserve(fields.len())?;
                for child in fields {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            Expr::SetMember {
                field,
                index,
                value,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::MapMember { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            Expr::MapLookup { field, index, key } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(key), visit_depth + 1, owner)?;
            }
            Expr::ListLength { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::ListIsEmpty { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::ListHead { field, index, ty } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            Expr::MerkleCheckRoot { field, index, root } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(root), visit_depth + 1, owner)?;
            }
            Expr::HistoricMerkleCheckRoot { field, index, root } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(root), visit_depth + 1, owner)?;
            }
            Expr::CellRead { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::CounterLessThan {
                field,
                index,
                threshold,
            } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
                scan.push(Node::Expr(threshold), visit_depth + 1, owner)?;
            }
            Expr::CounterRead { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::KernelSelf { ty } => {
                scan.push(Node::Type(ty), visit_depth + 1, owner)?;
            }
            Expr::SetSize { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::SetIsEmpty { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::MapIsEmpty { field, index } => {
                scan.push(Node::Text(field), visit_depth + 1, owner)?;
                let _ = index;
            }
            Expr::Tuple { elements } => {
                scan.reserve(elements.len())?;
                for child in elements {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            Expr::Vector { element, elements } => {
                scan.push(Node::Type(element), visit_depth + 1, owner)?;
                scan.reserve(elements.len())?;
                for child in elements {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            Expr::VectorMap {
                parameter,
                source,
                body,
                result,
                length,
            } => {
                scan.push(Node::Parameter(parameter), visit_depth + 1, owner)?;
                scan.push(Node::Expr(source), visit_depth + 1, owner)?;
                scan.push(Node::Expr(body), visit_depth + 1, owner)?;
                scan.push(Node::Type(result), visit_depth + 1, owner)?;
                let _ = length;
            }
            Expr::VectorFoldCall {
                name,
                initial,
                source,
                accumulator,
                element,
                length,
            } => {
                scan.call(owner, name, visit_depth);
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.push(Node::Expr(initial), visit_depth + 1, owner)?;
                scan.push(Node::Expr(source), visit_depth + 1, owner)?;
                scan.push(Node::Type(accumulator), visit_depth + 1, owner)?;
                scan.push(Node::Type(element), visit_depth + 1, owner)?;
                let _ = length;
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.push(Node::Expr(then), visit_depth + 1, owner)?;
                scan.push(Node::Expr(otherwise), visit_depth + 1, owner)?;
            }
            Expr::Let { bindings, body } => {
                scan.reserve(bindings.len())?;
                for child in bindings {
                    scan.push(Node::LocalBinding(child), visit_depth + 1, owner)?;
                }
                scan.push(Node::Expr(body), visit_depth + 1, owner)?;
            }
            Expr::Sequence { steps, value } => {
                scan.reserve(steps.len())?;
                for child in steps {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::Assert { condition, message } => {
                scan.push(Node::Expr(condition), visit_depth + 1, owner)?;
                scan.push(Node::Text(message), visit_depth + 1, owner)?;
            }
            Expr::Equal { left, right } => {
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::NotEqual { left, right } => {
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::Compare {
                operator,
                left,
                right,
            } => {
                scan.push(Node::ComparisonOperator(operator), visit_depth + 1, owner)?;
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::Call { name, arguments } => {
                scan.call(owner, name, visit_depth);
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.reserve(arguments.len())?;
                for child in arguments {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            Expr::TransientHash { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::TransientCommit { value, opening } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                scan.push(Node::Expr(opening), visit_depth + 1, owner)?;
            }
            Expr::PersistentHash { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::Keccak256 { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::PersistentCommit { value, opening } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
                scan.push(Node::Expr(opening), visit_depth + 1, owner)?;
            }
            Expr::DegradeToTransient { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::UpgradeFromTransient { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::HashToCurve { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::JubjubPointX { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::JubjubPointY { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::EcAdd { left, right } => {
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::ConstructJubjubPoint { x, y } => {
                scan.push(Node::Expr(x), visit_depth + 1, owner)?;
                scan.push(Node::Expr(y), visit_depth + 1, owner)?;
            }
            Expr::EcNeg { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::EcMul { point, scalar } => {
                scan.push(Node::Expr(point), visit_depth + 1, owner)?;
                scan.push(Node::Expr(scalar), visit_depth + 1, owner)?;
            }
            Expr::EcMulGenerator { scalar } => {
                scan.push(Node::Expr(scalar), visit_depth + 1, owner)?;
            }
            Expr::JubjubScalarFromNative { value } => {
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::WitnessCall { name, arguments } => {
                scan.push(Node::Text(name), visit_depth + 1, owner)?;
                scan.reserve(arguments.len())?;
                for child in arguments {
                    scan.push(Node::Expr(child), visit_depth + 1, owner)?;
                }
            }
            Expr::KernelClaim { claim, value } => {
                scan.push(Node::KernelClaimKind(claim), visit_depth + 1, owner)?;
                scan.push(Node::Expr(value), visit_depth + 1, owner)?;
            }
            Expr::KernelMintShielded { domain, amount } => {
                scan.push(Node::Expr(domain), visit_depth + 1, owner)?;
                scan.push(Node::Expr(amount), visit_depth + 1, owner)?;
            }
            Expr::CreateZswapInput { coin } => {
                scan.push(Node::Expr(coin), visit_depth + 1, owner)?;
            }
            Expr::CreateZswapOutput { coin, recipient } => {
                scan.push(Node::Expr(coin), visit_depth + 1, owner)?;
                scan.push(Node::Expr(recipient), visit_depth + 1, owner)?;
            }
            Expr::NativeWitnessCall { builtin } => {
                scan.push(Node::NativeWitnessBuiltin(builtin), visit_depth + 1, owner)?;
            }
            Expr::Add { left, right } => {
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::Subtract { left, right } => {
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::Multiply { left, right } => {
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::UnsignedAdd { max, left, right } => {
                scan.push(Node::Text(max), visit_depth + 1, owner)?;
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::UnsignedSubtract { max, left, right } => {
                scan.push(Node::Text(max), visit_depth + 1, owner)?;
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
            Expr::UnsignedMultiply { max, left, right } => {
                scan.push(Node::Text(max), visit_depth + 1, owner)?;
                scan.push(Node::Expr(left), visit_depth + 1, owner)?;
                scan.push(Node::Expr(right), visit_depth + 1, owner)?;
            }
        },
        Node::ComparisonOperator(value) => match value {
            ComparisonOperator::Less => {}
            ComparisonOperator::LessEqual => {}
            ComparisonOperator::Greater => {}
            ComparisonOperator::GreaterEqual => {}
        },
    }
    Ok(())
}
