// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Child traversal contract for shared semantic families.
//! The core has no native dispatcher or unmigrated syntax fallback. A complete
//! generic body visitor now supplies complete traversal for native output.
use super::sink::{LeafSink, Typed};
use crate::{RenderError, ir::Expr};

pub(crate) trait Children<S: LeafSink> {
    /// Return the checked child and its bounded inferred type. The caller's
    /// existing borrowed source/declaration gates retain their error order.
    fn visit(&mut self, source: &Expr, sink: &mut S) -> Result<Typed<S::Value>, RenderError>;
}

/// One lexical body visit with a borrowed annotation and consumer-owned symbol.
/// Implementations overlay the original source key and restore outer scope;
/// they do not validate the name/type or mutate the caller's bindings.
pub(crate) trait ScopedChildren<S: super::sink::AggregateSink>: Children<S> {
    fn visit_bound(
        &mut self,
        source: &Expr,
        source_key: &str,
        ty: &crate::ir::Type,
        binding: &S::Binding,
        sink: &mut S,
    ) -> Result<Typed<S::Value>, RenderError>;
}
