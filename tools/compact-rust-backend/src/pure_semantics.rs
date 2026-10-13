// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared semantic decisions for native carriers and pure certification.
//!
//! Policies in this module construct no Rust syntax. Native interpreters consume
//! checked facts. The complete expression-body certification fold is still being
//! migrated; carrier checks alone do not complete ADR-0473 or issue #622.

pub(crate) mod carrier;
pub(crate) mod coercion;
pub(crate) mod identifier;
pub(crate) mod numeric;
