// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared semantic decisions for native carriers and pure certification.
//!
//! Policies in this module construct no Rust syntax. Native interpreters consume
//! checked facts. Production pure body certification uses the complete shared
//! expression fold with unit output; remaining qualification is tracked in #622.

pub(crate) mod carrier;
pub(crate) mod coercion;
pub(crate) mod identifier;
pub(crate) mod numeric;

pub(crate) mod leaf;
pub(crate) mod ownership;
pub(crate) mod scope;
pub(crate) mod sink;

pub(crate) mod children;
pub(crate) mod conversion;

pub(crate) mod arithmetic;

pub(crate) mod crypto;
pub(crate) mod hash_encoding;
pub(crate) mod jubjub;

pub(crate) mod aggregate;
pub(crate) mod declarations;

pub(crate) mod expression;

pub(crate) mod visitor;

pub(crate) mod control;

pub(crate) mod body_check;
