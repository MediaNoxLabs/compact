// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Ordinary expression returns retain only their terminal lexical continuation.
use super::*;

impl Lowerer<'_> {
    pub(super) fn ordinary(
        &mut self,
        actions: &[StateAction],
        returned: &StateReturn,
        declared_result: &Type,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        base: &str,
    ) -> Result<Value, RecordingGap> {
        let return_path = path_at(base, "return_value");
        let return_scope = self.terminal_actions(
            actions,
            scope,
            bindings,
            &path_at(base, "actions"),
            self.call_stack.len() == 1,
        )?;
        match returned {
            StateReturn::Expression { value } => self.expression(
                value,
                &return_scope,
                bindings,
                &format!("{return_path}.value"),
            ),
            StateReturn::Unit => Ok(self.emit(
                Operation::Literal(Literal::Unit),
                Type::Unit,
                bindings,
                &return_path,
            )),
            // Specialized ledger returns use the original parameter scope, as native does.
            StateReturn::CellRead { field, index } => self.borrowed_leaf_return(
                returned,
                borrowed_operations::LeafQuery::CellRead {
                    field,
                    index: *index,
                },
                bindings,
                &return_path,
            ),
            StateReturn::CounterRead { field, index } => self.borrowed_leaf_return(
                returned,
                borrowed_operations::LeafQuery::CounterRead {
                    field,
                    index: *index,
                },
                bindings,
                &return_path,
            ),
            StateReturn::ListLength { field, index }
            | StateReturn::ListIsEmpty { field, index }
            | StateReturn::ListHead { field, index } => {
                self.spend(&return_path)?;
                let query = match returned {
                    StateReturn::ListLength { .. } => lists::Query::Length,
                    StateReturn::ListIsEmpty { .. } => lists::Query::IsEmpty,
                    _ => lists::Query::Head(declared_result),
                };
                let (operation, ty) = self.list_query(field, *index, query, &return_path)?;
                Ok(self.emit(operation, ty, bindings, &return_path))
            }
            StateReturn::MapMember { .. }
            | StateReturn::MapLookup { .. }
            | StateReturn::MapSize { .. }
            | StateReturn::MapIsEmpty { .. } => {
                self.spend(&return_path)?;
                let query = match returned {
                    StateReturn::MapMember { field, index, key } => {
                        maps::MapQuery::Member(field, *index, key)
                    }
                    StateReturn::MapLookup { field, index, key } => {
                        maps::MapQuery::Lookup(field, *index, key)
                    }
                    StateReturn::MapSize { field, index } => maps::MapQuery::Size(field, *index),
                    StateReturn::MapIsEmpty { field, index } => {
                        maps::MapQuery::IsEmpty(field, *index)
                    }
                    _ => unreachable!("Map return dispatch"),
                };
                let (operation, ty) = self.map_query(query, scope, bindings, &return_path)?;
                Ok(self.emit(operation, ty, bindings, &return_path))
            }
            StateReturn::SetMember {
                field,
                index,
                value,
            } => {
                // Borrow the operand: cloning a recursive return expression is
                // unnecessary. This is the same operation lowering as Expr::SetMember.
                self.spend(&return_path)?;
                let (operation, ty) =
                    self.set_member(field, *index, value, scope, bindings, &return_path)?;
                Ok(self.emit(operation, ty, bindings, &return_path))
            }
            StateReturn::SetSize { field, index } => self.borrowed_leaf_return(
                returned,
                borrowed_operations::LeafQuery::SetSize {
                    field,
                    index: *index,
                },
                bindings,
                &return_path,
            ),
            StateReturn::SetIsEmpty { field, index } => self.borrowed_leaf_return(
                returned,
                borrowed_operations::LeafQuery::SetIsEmpty {
                    field,
                    index: *index,
                },
                bindings,
                &return_path,
            ),
            StateReturn::MerkleCheckRoot { field, index, root }
            | StateReturn::HistoricMerkleCheckRoot { field, index, root } => {
                self.spend(&return_path)?;
                let (operation, ty) = self.merkle_query(
                    field,
                    *index,
                    matches!(returned, StateReturn::HistoricMerkleCheckRoot { .. }),
                    Some(root),
                    scope,
                    bindings,
                    &return_path,
                )?;
                Ok(self.emit(operation, ty, bindings, &return_path))
            }
            StateReturn::MerkleIsFull { field, index }
            | StateReturn::HistoricMerkleIsFull { field, index } => {
                self.spend(&return_path)?;
                let (operation, ty) = self.merkle_query(
                    field,
                    *index,
                    matches!(returned, StateReturn::HistoricMerkleIsFull { .. }),
                    None,
                    scope,
                    bindings,
                    &return_path,
                )?;
                Ok(self.emit(operation, ty, bindings, &return_path))
            }
            _ => {
                let mut gap = RecordingGap::returned(returned);
                gap.path = return_path;
                Err(gap)
            }
        }
    }

    /// Earlier siblings restore their lexical scopes before the continuation.
    fn terminal_actions(
        &mut self,
        actions: &[StateAction],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
        root_boundary: bool,
    ) -> Result<Scope, RecordingGap> {
        let Some((last, prefix)) = actions.split_last() else {
            return Ok(scope.clone());
        };
        #[cfg(not(test))]
        let _ = root_boundary;
        #[cfg(test)]
        let outer = self.root_action;
        for (i, action) in prefix.iter().enumerate() {
            #[cfg(test)]
            if root_boundary {
                self.root_action = Some(i);
            }
            self.action(action, scope, bindings, &format!("{path}[{i}]"))?;
            #[cfg(test)]
            {
                self.root_action = outer;
            }
        }
        #[cfg(test)]
        if root_boundary {
            self.root_action = Some(prefix.len());
        }
        let result =
            self.terminal_action(last, scope, bindings, &format!("{path}[{}]", prefix.len()))?;
        #[cfg(test)]
        {
            self.root_action = outer;
        }
        Ok(result)
    }

    /// Follow only final Sequence children and Let bodies; If branches stay local.
    fn terminal_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Scope, RecordingGap> {
        self.spend(path)?;
        match action {
            StateAction::Sequence { actions } => {
                self.terminal_actions(actions, scope, bindings, &format!("{path}.actions"), false)
            }
            StateAction::Let {
                bindings: locals,
                action,
            } => {
                let scope = self.locals(locals, scope, bindings, path)?;
                self.terminal_action(action, &scope, bindings, &format!("{path}.action"))
            }
            _ => {
                self.action(action, scope, bindings, path)?;
                Ok(scope.clone())
            }
        }
    }
}
