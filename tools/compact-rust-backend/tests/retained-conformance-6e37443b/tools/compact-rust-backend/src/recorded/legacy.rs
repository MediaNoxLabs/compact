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

//! Existing recording recognizers, separated by admitted domain.
//!
//! These are mechanical extractions, not a new compositional lowering model.
//! Keep profile ordering and rejection refinement in the parent orchestrator.
//! Only explicit arguments cross this boundary; no helper captures outer state.

pub(super) mod guarded_map;
pub(super) mod guarded_pure;
pub(super) mod opaque_map;
pub(super) mod opaque_set;
pub(super) mod value_sources;

// Mutually recursive lowering domains; profile selection remains with the orchestrator.
pub(super) mod actions;
pub(super) mod calls;
pub(super) mod expressions;
pub(super) mod returns;
