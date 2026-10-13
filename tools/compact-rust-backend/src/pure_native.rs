// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native output views over children checked once by the pure semantic owner.
//! This checkpoint still constructs syntax in body certification. It does not
//! implement the complete syntax-free Expr fold or its CheckSink.

pub(crate) mod control;
pub(crate) mod dispatch;
pub(crate) mod fragment;
pub(crate) mod leaf;

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod visitation;

pub(crate) mod children;
pub(crate) mod conversion;

pub(crate) mod arithmetic;

pub(crate) mod crypto;

pub(crate) mod aggregate;

pub(crate) mod item;

pub(crate) mod local;
