// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native output views over children checked once by the pure semantic owner.
//! This checkpoint still constructs syntax in body certification. It does not
//! implement the complete syntax-free Expr fold or its CheckSink.

pub(crate) mod control;
pub(crate) mod dispatch;
pub(crate) mod fragment;

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod visitation;
