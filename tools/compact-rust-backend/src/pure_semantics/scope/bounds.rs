// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Successful resource bounds belong to one immutable binding session.
//! These facts certify neither a carrier nor a semantic Type operation.
use super::{Bindings, Scope};
use crate::{RenderError, ir::Type};
use std::{borrow::Cow, cell::RefCell, collections::HashSet};

pub(crate) struct TypeBoundsSession<'a, B: Clone> {
    bindings: Cow<'a, Scope<'a, B>>,
    successful: RefCell<HashSet<&'a str>>,
    #[cfg(test)]
    checks: std::cell::Cell<usize>,
}
impl<'a, B: Clone> TypeBoundsSession<'a, B> {
    /// Borrow the root map; do not validate unused annotations or clone metadata.
    pub(crate) fn new(bindings: &'a Scope<'a, B>) -> Self {
        Self {
            bindings: Cow::Borrowed(bindings),
            successful: RefCell::new(HashSet::new()),
            #[cfg(test)]
            checks: std::cell::Cell::new(0),
        }
    }
    /// Reborrow inherited annotations into the child's shorter lifetime.
    pub(crate) fn child<'child>(&'child self) -> TypeBoundsSession<'child, B>
    where
        'a: 'child,
    {
        TypeBoundsSession {
            // This is the same one metadata-map clone as the old LocalScope.
            bindings: Cow::Owned(self.bindings.as_ref().clone()),
            successful: RefCell::new(self.successful.borrow().clone()),
            #[cfg(test)]
            checks: std::cell::Cell::new(0),
        }
    }
    pub(crate) fn insert(&mut self, key: &'a str, annotation: &'a Type, binding: B) {
        // Same raw key may shadow a different annotation: never retain its fact.
        self.successful.get_mut().remove(key);
        self.bindings.to_mut().insert(key, (annotation, binding));
    }
    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, &'a str, (&'a Type, B)> {
        self.bindings.iter()
    }
}
impl<B: Clone> Bindings<B> for TypeBoundsSession<'_, B> {
    fn get(&self, source_name: &str) -> Option<(&Type, &B)> {
        self.bindings
            .get(source_name)
            .map(|(ty, binding)| (*ty, binding))
    }
    fn get_bounded(&self, source_name: &str) -> Result<Option<(&Type, &B)>, RenderError> {
        let Some((raw_key, (ty, binding))) = self.bindings.get_key_value(source_name) else {
            return Ok(None);
        };
        if !self.successful.borrow().contains(raw_key) {
            drop(crate::resource_limits::recording_type_nodes(ty)?);
            self.successful.borrow_mut().insert(*raw_key);
            #[cfg(test)]
            self.checks.set(self.checks.get() + 1);
        }
        Ok(Some((*ty, binding)))
    }
}

#[cfg(test)]
mod tests;
