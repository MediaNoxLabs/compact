// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Typed resource identity is produced at the policy check, never from text.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingResourceKind {
    FixedBytePayload,
    FixedLayoutSlots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingResourcePhase {
    FixedLayoutValidation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordingResource {
    pub kind: RecordingResourceKind,
    pub phase: RecordingResourcePhase,
    pub limit: usize,
}

impl super::RecordingGap {
    pub(in crate::recorded) fn with_fixed_layout_resource(
        mut self,
        kind: RecordingResourceKind,
        limit: usize,
    ) -> Self {
        self.resource = Some(Box::new(RecordingResource {
            kind,
            phase: RecordingResourcePhase::FixedLayoutValidation,
            limit,
        }));
        self
    }
}
