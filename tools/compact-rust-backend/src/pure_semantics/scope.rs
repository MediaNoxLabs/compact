// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Borrow declared types and generic binding payloads without rebuilding scopes.
//! Signature/lexical owners retain their own insertion, name and duplicate order.
use crate::ir::Type;
use std::collections::HashMap;

pub(crate) trait Bindings<B> {
    fn get(&self, source_name: &str) -> Option<(&Type, &B)>;
}

impl<B> Bindings<B> for HashMap<&str, (&Type, B)> {
    fn get(&self, source_name: &str) -> Option<(&Type, &B)> {
        HashMap::get(self, source_name).map(|(ty, binding)| (*ty, binding))
    }
}

/// Original raw keys and borrowed annotations; consumers own only binding metadata.
pub(crate) type Scope<'a, B> = HashMap<&'a str, (&'a Type, B)>;

/// A lexical session owns one map clone and never mutates its parent.
/// Insertion performs no validation: the control/aggregate owner checks first.
pub(crate) struct LocalScope<'a, B> {
    bindings: Scope<'a, B>,
}
impl<'a, B: Clone> LocalScope<'a, B> {
    pub(crate) fn new(bindings: &Scope<'a, B>) -> Self {
        Self {
            bindings: bindings.clone(),
        }
    }
    pub(crate) fn bindings(&self) -> &Scope<'a, B> {
        &self.bindings
    }
    pub(crate) fn insert(&mut self, key: &'a str, annotation: &'a Type, binding: B) {
        self.bindings.insert(key, (annotation, binding));
    }
}
