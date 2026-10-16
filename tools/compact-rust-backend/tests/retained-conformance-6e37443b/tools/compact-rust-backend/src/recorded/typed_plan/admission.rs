// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Configurable admission settings for a candidate recording plan.
//!
//! Profiles select the allowed value and effect domains. Voting setup enables
//! counter-hash helpers only after its recipient purity check; this configuration
//! is not immutable. Composition lookup sites retain the audited AST identities.

use super::{CompositeDomain, composition};
use crate::ir::Expr;
use std::collections::HashSet;

pub(super) struct AdmissionPolicy<'a> {
    pub(super) field_cell_slot: Option<(String, u8)>,
    pub(super) effectful_field_cells: bool,
    pub(super) read_only_assertions: bool,
    pub(super) unit_actions: bool,
    pub(super) phase_reset: bool,
    pub(super) composite_domain: CompositeDomain,
    pub(super) composition_calls: Option<composition::Calls<'a>>,
    pub(super) composition_lookup_sites: HashSet<*const Expr>,
    pub(super) counter_hash_helpers: bool,
}
