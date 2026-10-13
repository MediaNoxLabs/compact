// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Child traversal contract for shared semantic families.
//! The core has no native dispatcher or unmigrated syntax fallback. A complete
//! generic body visitor replaces the transitional native environment later.
use super::sink::{LeafSink, Typed};
use crate::{RenderError, ir::Expr};

pub(crate) trait Children<S: LeafSink> {
    /// Return the checked child and its bounded inferred type. The caller's
    /// existing borrowed source/declaration gates retain their error order.
    fn visit(&mut self, source: &Expr, sink: &mut S) -> Result<Typed<S::Value>, RenderError>;
}
