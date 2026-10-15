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

//! Mutable state of a recording-plan lowering attempt.
//!
//! Temporary identifiers advance as values are lowered. Helper calls temporarily
//! change query/argument modes and scalar depth, and track active callees for
//! cycle rejection. The caller owns save/restore and cleanup, including failures;
//! these fields are neither fixed admission policy nor observed effect counts.

use std::collections::HashSet;

pub(super) struct EvaluationState {
    pub(super) next: usize,
    pub(super) context_query: bool,
    pub(super) scalar_arguments: bool,
    pub(super) scalar_body_depth: usize,
    pub(super) active_calls: HashSet<String>,
}
