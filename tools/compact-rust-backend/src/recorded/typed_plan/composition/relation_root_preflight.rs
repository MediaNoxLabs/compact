// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Root relation owner. Borrowed typed-IR traversal indexes only reachable
//! helper roles; scoped argument origins and finite event states then require
//! one selected read before at most one matching write on each path. The
//! composition Audit still rejects hidden or unsupported effects everywhere.
use super::nested_map_preflight::NestedMapRead;
use super::selected_set_preflight::{
    SelectedSetFamily, SelectedSetRole, checked_insert, checked_read, checked_remove,
};
use super::*;
use std::collections::BTreeSet;

#[derive(Default)]
struct CalledHelpers {
    expression: BTreeSet<String>,
    action: BTreeSet<String>,
}

struct ClassifiedRoles<'a> {
    reads: Vec<SelectedSetRole<'a>>,
    inserts: Vec<SelectedSetRole<'a>>,
    removes: Vec<SelectedSetRole<'a>>,
    compatible: Vec<NestedMapRead<'a>>,
    #[cfg(test)]
    inspected: usize,
}

impl<'a> ClassifiedRoles<'a> {
    fn from_called(
        called: &CalledHelpers,
        fields: &HashMap<&str, &'a LedgerField>,
        circuits: &HashMap<&str, &'a StatefulCircuit>,
    ) -> Self {
        let mut roles = Self {
            reads: Vec::new(),
            inserts: Vec::new(),
            removes: Vec::new(),
            compatible: Vec::new(),
            #[cfg(test)]
            inspected: 0,
        };
        for name in &called.expression {
            if let Some(circuit) = circuits.get(name.as_str()).copied() {
                #[cfg(test)]
                {
                    roles.inspected += 1;
                }
                if let Some(read) = checked_read(circuit, fields) {
                    roles.reads.push(read);
                }
            }
        }
        for name in &called.action {
            if let Some(circuit) = circuits.get(name.as_str()).copied() {
                #[cfg(test)]
                {
                    roles.inspected += 3;
                }
                if let Some(insert) = checked_insert(circuit, fields) {
                    roles.inserts.push(insert);
                }
                if let Some(remove) = checked_remove(circuit, fields) {
                    roles.removes.push(remove);
                }
                if let Some(nested) = NestedMapRead::checked(circuit, fields) {
                    roles.compatible.push(nested);
                }
            }
        }
        roles
    }
}

// Candidate discovery traverses borrowed, typed IR. This is a search index;
// checked family and scoped origins still decide admission below.
fn called_helpers(actions: &[StateAction], result: &StateReturn) -> CalledHelpers {
    fn expression(value: &Expr, called: &mut CalledHelpers) {
        let names = std::cell::RefCell::new(&mut called.expression);
        crate::circuit_analysis::expression_contains(value, &|node| {
            if let Expr::Call { name, .. } = node {
                names.borrow_mut().insert(name.clone());
            }
            false
        });
    }
    fn action(value: &StateAction, called: &mut CalledHelpers) {
        match value {
            StateAction::Sequence { actions } => {
                for item in actions {
                    action(item, called);
                }
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                expression(condition, called);
                action(then, called);
                action(otherwise, called);
            }
            StateAction::Let {
                bindings,
                action: body,
            } => {
                for binding in bindings {
                    expression(&binding.value, called);
                }
                action(body, called);
            }
            StateAction::CircuitCall { name, arguments } => {
                called.action.insert(name.clone());
                for argument in arguments {
                    expression(argument, called);
                }
            }
            StateAction::PureCall { arguments, .. } => {
                for argument in arguments {
                    expression(argument, called);
                }
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
            | StateAction::MapRemove { key: value, .. } => expression(value, called),
            StateAction::Assert { condition, .. } => expression(condition, called),
            StateAction::MapInsert { key, value, .. } => {
                expression(key, called);
                expression(value, called);
            }
            StateAction::CellWriteCoin {
                coin, recipient, ..
            }
            | StateAction::SetInsertCoin {
                coin, recipient, ..
            } => {
                expression(coin, called);
                expression(recipient, called);
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
                expression(value, called);
                expression(position, called);
            }
            StateAction::CounterIncrement { .. }
            | StateAction::CounterDecrement { .. }
            | StateAction::CounterReset { .. }
            | StateAction::NativeWitnessCall { .. }
            | StateAction::SetReset { .. }
            | StateAction::ListPopFront { .. }
            | StateAction::ListReset { .. }
            | StateAction::MapReset { .. }
            | StateAction::HistoricMerkleResetHistory { .. }
            | StateAction::HistoricMerkleResetToDefault { .. }
            | StateAction::MerkleResetToDefault { .. } => {}
        }
    }
    fn plan(value: &ReturnPlan, called: &mut CalledHelpers) {
        match value {
            ReturnPlan::Value { value } => expression(value, called),
            ReturnPlan::Sequence { actions, result } => {
                for item in actions {
                    action(item, called);
                }
                plan(result, called);
            }
            ReturnPlan::Let { bindings, result } => {
                for binding in bindings {
                    expression(&binding.value, called);
                }
                plan(result, called);
            }
            ReturnPlan::Conditional {
                condition,
                then,
                otherwise,
            } => {
                expression(condition, called);
                plan(then, called);
                plan(otherwise, called);
            }
        }
    }
    let mut called = CalledHelpers::default();
    for item in actions {
        action(item, &mut called);
    }
    match result {
        StateReturn::Expression { value }
        | StateReturn::SetMember { value, .. }
        | StateReturn::HistoricMerkleCheckRoot { root: value, .. }
        | StateReturn::MerkleCheckRoot { root: value, .. } => expression(value, &mut called),
        StateReturn::MapMember { key, .. } | StateReturn::MapLookup { key, .. } => {
            expression(key, &mut called)
        }
        StateReturn::Effectful { body } => plan(body, &mut called),
        StateReturn::Unit
        | StateReturn::CellRead { .. }
        | StateReturn::CounterRead { .. }
        | StateReturn::SetSize { .. }
        | StateReturn::SetIsEmpty { .. }
        | StateReturn::MapSize { .. }
        | StateReturn::MapIsEmpty { .. }
        | StateReturn::ListLength { .. }
        | StateReturn::ListIsEmpty { .. }
        | StateReturn::ListHead { .. }
        | StateReturn::HistoricMerkleIsFull { .. }
        | StateReturn::MerkleIsFull { .. } => {}
    }
    called
}

fn indirect_selected_call(
    root: &CalledHelpers,
    family: &SelectedSetFamily<'_>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Option<bool> {
    let selected = [
        family.read.name.as_str(),
        family.insert.name.as_str(),
        family.remove.name.as_str(),
    ];
    fn reaches(
        name: &str,
        selected: &[&str; 3],
        circuits: &HashMap<&str, &StatefulCircuit>,
        visited: &mut HashSet<String>,
    ) -> Option<bool> {
        if selected.contains(&name) {
            return Some(true);
        }
        if !visited.insert(name.to_owned()) {
            return Some(false);
        }
        let Some(circuit) = circuits.get(name) else {
            // Pure callees have their own existing complete body audit.
            return Some(false);
        };
        let called = called_helpers(&circuit.actions, &circuit.return_value);
        for child in called.expression.iter().chain(called.action.iter()) {
            if reaches(child, selected, circuits, visited)? {
                return Some(true);
            }
        }
        Some(false)
    }
    for name in root.expression.iter().chain(root.action.iter()) {
        if selected.contains(&name.as_str()) {
            continue;
        }
        if reaches(name, &selected, circuits, &mut HashSet::new())? {
            return Some(true);
        }
    }
    Some(false)
}

pub(super) enum RelationPermit<'a> {
    Selected {
        family: SelectedSetFamily<'a>,
        compatible: Option<NestedMapRead<'a>>,
    },
    NestedStandalone(NestedMapRead<'a>),
}

impl<'a> RelationPermit<'a> {
    pub(super) fn discover(
        root: &'a StatefulCircuit,
        fields: &HashMap<&str, &'a LedgerField>,
        circuits: &HashMap<&str, &'a StatefulCircuit>,
    ) -> Option<Self> {
        let called = called_helpers(&root.actions, &root.return_value);
        let roles = ClassifiedRoles::from_called(&called, fields, circuits);
        if let [read] = roles.reads.as_slice() {
            let inserts = roles
                .inserts
                .iter()
                .filter(|candidate| read.same_family(candidate))
                .collect::<Vec<_>>();
            let removes = roles
                .removes
                .iter()
                .filter(|candidate| read.same_family(candidate))
                .collect::<Vec<_>>();
            if let ([insert], [remove]) = (inserts.as_slice(), removes.as_slice()) {
                let family = SelectedSetFamily::join(read.clone(), insert, remove)?;
                if !indirect_selected_call(&called, &family, circuits)? {
                    for nested in &roles.compatible {
                        let mut owner = RootOwner {
                            selected: &family,
                            compatible: Some(nested),
                            origin: None,
                        };
                        if owner.checked(root).is_some() {
                            return Some(Self::Selected {
                                family,
                                compatible: Some(nested.clone()),
                            });
                        }
                    }
                    let mut owner = RootOwner {
                        selected: &family,
                        compatible: None,
                        origin: None,
                    };
                    if owner.checked(root).is_some() {
                        return Some(Self::Selected {
                            family,
                            compatible: None,
                        });
                    }
                }
            }
        }
        NestedMapRead::checked(root, fields).map(Self::NestedStandalone)
    }

    pub(super) fn selected_member(
        &self,
        active: &HashSet<String>,
        declaration: &LedgerField,
    ) -> bool {
        match self {
            Self::Selected { family, .. } => {
                active.contains(&family.read.name)
                    && family
                        .branches
                        .values()
                        .any(|slot| std::ptr::eq(*slot, declaration))
            }
            Self::NestedStandalone(_) => false,
        }
    }
    pub(super) fn nested_map(&self, active: &HashSet<String>, declaration: &LedgerField) -> bool {
        let nested = match self {
            Self::Selected {
                compatible: Some(nested),
                ..
            }
            | Self::NestedStandalone(nested) => nested,
            Self::Selected {
                compatible: None, ..
            } => return false,
        };
        active.contains(&nested.circuit.name)
            && std::ptr::eq(nested.declaration, declaration)
            && nested.retains_checked_projection()
    }
    pub(super) fn has_selected(&self) -> bool {
        matches!(self, Self::Selected { .. })
    }
    pub(super) fn standalone(&self) -> bool {
        matches!(self, Self::NestedStandalone(_))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Event {
    Read,
    Compatible,
    Insert,
    Remove,
}
#[derive(Clone)]
struct ScopeOrigins {
    values: HashMap<String, usize>,
    occupied: HashSet<String>,
}
impl ScopeOrigins {
    fn new(circuit: &StatefulCircuit) -> Option<Self> {
        let values = circuit
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.clone(), i))
            .collect::<HashMap<_, _>>();
        (values.len() == circuit.parameters.len()).then(|| Self {
            occupied: values.keys().cloned().collect(),
            values,
        })
    }
    fn origin(&self, value: &Expr) -> Option<usize> {
        match value {
            Expr::Parameter { name } => self.values.get(name).copied(),
            Expr::Coerce { value, .. } => self.origin(value),
            _ => None,
        }
    }
    fn bind(&mut self, binding: &LocalBinding) -> Option<()> {
        let source = self.origin(&binding.value);
        if !self.occupied.insert(binding.name.clone()) {
            return None;
        }
        if let Some(source) = source {
            self.values.insert(binding.name.clone(), source);
        }
        Some(())
    }
}
struct RootOwner<'a> {
    selected: &'a SelectedSetFamily<'a>,
    compatible: Option<&'a NestedMapRead<'a>>,
    origin: Option<(usize, usize)>,
}
impl RootOwner<'_> {
    fn event(&self, path: &mut Vec<Event>, next: Event) -> Option<()> {
        let valid = match (path.as_slice(), next) {
            ([], Event::Read) => true,
            ([Event::Read], Event::Compatible) => self.compatible.is_some(),
            ([Event::Read, Event::Compatible], Event::Insert) => self.compatible.is_some(),
            ([Event::Read], Event::Insert) => self.compatible.is_none(),
            ([Event::Read], Event::Remove) => true,
            _ => false,
        };
        if !valid {
            return None;
        }
        path.push(next);
        Some(())
    }
    fn direct_origin(arguments: &[Expr], scope: &ScopeOrigins) -> Option<(usize, usize)> {
        let [relation, key] = arguments else {
            return None;
        };
        Some((scope.origin(relation)?, scope.origin(key)?))
    }
    fn call_arguments(&self, arguments: &[Expr], scope: &ScopeOrigins) -> bool {
        Self::direct_origin(arguments, scope).is_some_and(|origin| Some(origin) == self.origin)
    }
    fn expression(
        &mut self,
        value: &Expr,
        scope: &ScopeOrigins,
        events: &mut Vec<Event>,
    ) -> Option<()> {
        match value {
            Expr::Call { name, arguments } => {
                for argument in arguments {
                    self.expression(argument, scope, events)?;
                }
                if name == &self.selected.read.name {
                    let candidate = Self::direct_origin(arguments, scope)?;
                    if self.origin.is_some_and(|previous| previous != candidate) {
                        return None;
                    }
                    self.origin = Some(candidate);
                    self.event(events, Event::Read)?;
                }
                Some(())
            }
            Expr::Coerce { value, .. }
            | Expr::StructField { value, .. }
            | Expr::UnsignedCast { value, .. }
            | Expr::FieldCast { value }
            | Expr::JubjubPointX { value }
            | Expr::JubjubPointY { value } => self.expression(value, scope, events),
            Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
                self.expression(left, scope, events)?;
                self.expression(right, scope, events)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                self.expression(condition, scope, events)?;
                // A selected reader in either arm is not a direct scoped read.
                // The admitted original and reducers have no selected calls in
                // expression arms; the normal Audit still checks all arms.
                let before = events.len();
                self.expression(then, scope, events)?;
                self.expression(otherwise, scope, events)?;
                (events.len() == before).then_some(())
            }
            Expr::Parameter { .. }
            | Expr::EnumVariant { .. }
            | Expr::Boolean { .. }
            | Expr::FieldLiteral { .. }
            | Expr::UnsignedLiteral { .. }
            | Expr::BytesLiteral { .. }
            | Expr::CellRead { .. } => Some(()),
            _ => None,
        }
    }
    fn action(
        &mut self,
        action: &StateAction,
        scope: &ScopeOrigins,
        paths: Vec<Vec<Event>>,
    ) -> Option<Vec<Vec<Event>>> {
        match action {
            StateAction::Sequence { actions } => {
                let mut paths = paths;
                for action in actions {
                    paths = self.action(action, scope, paths)?;
                }
                Some(paths)
            }
            StateAction::Let { bindings, action } => {
                let mut local = scope.clone();
                let mut paths = paths;
                for binding in bindings {
                    for path in &mut paths {
                        self.expression(&binding.value, &local, path)?;
                    }
                    local.bind(binding)?;
                }
                self.action(action, &local, paths)
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let mut selected = paths.clone();
                let mut alternate = paths;
                for path in &mut selected {
                    self.expression(condition, scope, path)?;
                }
                for path in &mut alternate {
                    self.expression(condition, scope, path)?;
                }
                selected = self.action(then, scope, selected)?;
                selected.extend(self.action(otherwise, scope, alternate)?);
                selected.sort();
                selected.dedup();
                Some(selected)
            }
            StateAction::Assert { condition, .. } => {
                let mut paths = paths;
                for path in &mut paths {
                    self.expression(condition, scope, path)?;
                }
                Some(paths)
            }
            StateAction::PureCall { arguments, .. }
            | StateAction::CircuitCall { arguments, .. } => {
                let mut paths = paths;
                for path in &mut paths {
                    for argument in arguments {
                        self.expression(argument, scope, path)?;
                    }
                }
                if let StateAction::CircuitCall { name, .. } = action {
                    let event = if name == &self.selected.insert.name {
                        Some(Event::Insert)
                    } else if name == &self.selected.remove.name {
                        Some(Event::Remove)
                    } else if self.compatible.is_some_and(|c| name == &c.circuit.name) {
                        Some(Event::Compatible)
                    } else {
                        None
                    };
                    if let Some(event) = event {
                        if !self.call_arguments(arguments, scope) {
                            return None;
                        }
                        for path in &mut paths {
                            self.event(path, event)?;
                        }
                    }
                }
                Some(paths)
            }
            _ => None,
        }
    }
    fn checked(&mut self, root: &StatefulCircuit) -> Option<Vec<Vec<Event>>> {
        if root.result != Type::Unit || root.return_value != StateReturn::Unit {
            return None;
        }
        let scope = ScopeOrigins::new(root)?;
        let mut paths = vec![Vec::new()];
        for action in &root.actions {
            paths = self.action(action, &scope, paths)?;
        }
        let (relation, key) = self.origin?;
        if root.parameters.get(relation)?.ty != self.selected.read.parameters[0].ty
            || root.parameters.get(key)?.ty != Type::OpaqueString
        {
            return None;
        }
        let mut saw_insert = false;
        let mut saw_remove = false;
        for path in &paths {
            if path.first() != Some(&Event::Read)
                || path.iter().filter(|e| **e == Event::Read).count() != 1
            {
                return None;
            }
            let writes = path
                .iter()
                .filter(|e| matches!(e, Event::Insert | Event::Remove))
                .count();
            if writes > 1 {
                return None;
            }
            saw_insert |= path.contains(&Event::Insert);
            saw_remove |= path.contains(&Event::Remove);
            let checks = path.iter().filter(|e| **e == Event::Compatible).count();
            if checks > 1 {
                return None;
            }
            if self.compatible.is_some() {
                if path.contains(&Event::Insert) {
                    if path.as_slice() != [Event::Read, Event::Compatible, Event::Insert] {
                        return None;
                    }
                } else if checks != 0 {
                    return None;
                }
            } else if checks != 0 {
                return None;
            }
        }
        (saw_insert && saw_remove).then_some(paths)
    }
}

#[cfg(test)]
#[path = "relation_root_preflight_tests.rs"]
mod tests;
