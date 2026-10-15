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

//! Effects observed while lowering a candidate recording plan.
//!
//! Counts include observations across syntactic branches and may include work
//! from a partially lowered attempt that is later rejected. They are admission
//! evidence, not runtime event counts, cost estimates, or proof guarantees.

#[derive(Default)]
pub(super) struct EffectEvidence {
    pub(super) witness_calls: usize,
    pub(super) kernel_self_reads: usize,
    pub(super) root_observations: usize,
    pub(super) tree_writes: usize,
    pub(super) set_writes: usize,
    pub(super) counter_writes: usize,
    pub(super) counter_reads: usize,
    pub(super) counter_comparisons: usize,
    pub(super) cell_reads: usize,
    pub(super) cell_writes: usize,
    pub(super) field_cell_writes: usize,
    pub(super) intent_effects: usize,
    pub(super) intent_queries: usize,
    pub(super) zswap_inputs: usize,
    pub(super) zswap_outputs: usize,
    pub(super) scalar_helper_calls: usize,
    pub(super) scalar_counter_reads: usize,
    pub(super) optional_cells: usize,
    pub(super) opaque_cells: usize,
    pub(super) historic_roots: usize,
    pub(super) historic_writes: usize,
    pub(super) qualified_set_reads: usize,
    pub(super) qualified_set_writes: usize,
    pub(super) qualified_cell_writes: usize,
}
