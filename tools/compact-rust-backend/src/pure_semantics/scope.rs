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
