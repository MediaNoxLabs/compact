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

//! Private, conjunctive resource policy. These are bounded support limits, not
//! a universal denial-of-service or stack-safety guarantee. Render path units
//! are private, calibrated weights rounded from debug frame families. They are
//! neither measured live stack bytes nor a cross-platform stack guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResourceKind {
    Nodes,
    PendingNodes,
    StringBytes,
    LiteralBytes,
    SyntaxDepth,
    Declarations,
    CallDepth,
    ExpandedDepth,
    ExpandedWork,
    PurePath,
    NativePath,
    LegacyRecordedPath,
    TypedRecordedPath,
}
impl ResourceKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Nodes => "nodes",
            Self::PendingNodes => "pending_nodes",
            Self::StringBytes => "string_bytes",
            Self::LiteralBytes => "literal_bytes",
            Self::SyntaxDepth => "syntax_depth",
            Self::Declarations => "declarations",
            Self::CallDepth => "call_depth",
            Self::ExpandedDepth => "expanded_depth",
            Self::ExpandedWork => "expanded_work",
            Self::PurePath => "pure_render_path_units",
            Self::NativePath => "native_render_path_units",
            Self::LegacyRecordedPath => "legacy_recorded_render_path_units",
            Self::TypedRecordedPath => "typed_recorded_render_path_units",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    pub(super) nodes: usize,
    pub(super) pending: usize,
    pub(super) strings: usize,
    pub(super) literal_bytes: usize,
    pub(super) syntax_depth: usize,
    pub(super) declarations: usize,
    pub(super) call_depth: usize,
    pub(super) expanded_depth: usize,
    pub(super) expanded_work: usize,
    pub(super) path_units: usize,
}
impl Limits {
    /// Census envelope only: not a published support policy or unlimited mode.
    #[cfg(test)]
    pub(super) const CENSUS: Self = Self {
        nodes: 1_000_000,
        pending: 100_000,
        strings: 16_777_216,
        literal_bytes: 16_777_216,
        syntax_depth: 256,
        declarations: 8192,
        call_depth: 256,
        expanded_depth: 4096,
        expanded_work: 100_000_000,
        path_units: 100_000,
    };
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LimitError {
    pub(super) resource: ResourceKind,
    pub(super) observed: usize,
    pub(super) limit: usize,
}
impl std::fmt::Display for LimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "compiler resource {} exceeds {} (observed {})",
            self.resource.label(),
            self.limit,
            self.observed
        )
    }
}
impl std::error::Error for LimitError {}
pub(super) fn check(resource: ResourceKind, value: usize, limit: usize) -> Result<(), LimitError> {
    if value > limit {
        Err(LimitError {
            resource,
            observed: value,
            limit,
        })
    } else {
        Ok(())
    }
}

impl Limits {
    pub(super) const DEFAULT: Self = Self {
        nodes: 32768,
        pending: 4096,
        strings: 262144,
        literal_bytes: 65536,
        syntax_depth: 48,
        declarations: 256,
        call_depth: 16,
        expanded_depth: 64,
        expanded_work: 131072,
        path_units: 1664,
    };
}
