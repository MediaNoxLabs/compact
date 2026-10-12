// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Internal lowering provenance; display strings never decide diagnostic priority.
use super::{PlanError, RecordingGap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum Precision {
    ConcreteNode,
    #[default]
    Unclassified,
}

#[derive(Debug)]
pub(in crate::recorded) struct LoweringRejection {
    pub(super) gap: RecordingGap,
    pub(super) root_action: Option<usize>,
    pub(super) precision: Precision,
    pub(super) concrete_node: Option<Box<str>>,
}

impl From<RecordingGap> for LoweringRejection {
    fn from(gap: RecordingGap) -> Self {
        Self {
            gap,
            root_action: None,
            precision: Precision::Unclassified,
            concrete_node: None,
        }
    }
}

impl LoweringRejection {
    /// Existing callers intentionally discard provenance without running lowering again.
    pub(super) fn into_parts(self) -> (RecordingGap, Option<usize>, Precision, Option<Box<str>>) {
        (
            self.gap,
            self.root_action,
            self.precision,
            self.concrete_node,
        )
    }

    // Production Unit admission refines diagnostics using typed lowering provenance.
    pub(in crate::recorded) fn refine_action(
        &self,
        ordinal: usize,
        legacy_precision: crate::recorded::LegacyActionPrecision,
        gap: RecordingGap,
    ) -> RecordingGap {
        if self.root_action == Some(ordinal)
            && self.precision == Precision::ConcreteNode
            && legacy_precision == crate::recorded::LegacyActionPrecision::Coarse
            && gap.code == super::RecordingGapCode::UnsupportedAction
        {
            let mut refined = self.gap.clone();
            if let Some(node) = &self.concrete_node {
                refined.ir_node = node.to_string();
            }
            refined
        } else {
            gap
        }
    }
}

#[derive(Debug)]
pub(in crate::recorded) enum DiagnosticPlanError {
    Unsupported(LoweringRejection),
    Invalid(RecordingGap),
}

impl From<DiagnosticPlanError> for PlanError {
    fn from(error: DiagnosticPlanError) -> Self {
        match error {
            DiagnosticPlanError::Unsupported(rejection) => {
                Self::Unsupported(rejection.into_parts().0)
            }
            DiagnosticPlanError::Invalid(gap) => Self::Invalid(gap),
        }
    }
}

#[cfg(test)]
#[path = "rejection/tests.rs"]
mod tests;
