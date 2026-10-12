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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum FailurePhase {
    #[default]
    OutsideBody,
    CircuitBody,
}

/// Sized diagnostic label keeps a single pointer in the recursive error carrier.
#[derive(Debug)]
pub(super) struct ConcreteNode {
    pub(super) name: String,
}

#[derive(Debug)]
pub(in crate::recorded) struct LoweringRejection {
    pub(super) gap: RecordingGap,
    pub(super) root_action: Option<usize>,
    pub(super) precision: Precision,
    pub(super) phase: FailurePhase,
    pub(super) concrete_node: Option<Box<ConcreteNode>>,
}

impl From<RecordingGap> for LoweringRejection {
    fn from(gap: RecordingGap) -> Self {
        Self {
            gap,
            root_action: None,
            precision: Precision::Unclassified,
            phase: FailurePhase::OutsideBody,
            concrete_node: None,
        }
    }
}

impl LoweringRejection {
    /// Consume the public gap for a refusal-only diagnostic projection.
    pub(in crate::recorded) fn into_gap(self) -> RecordingGap {
        self.gap
    }

    /// Existing callers intentionally discard provenance without running lowering again.
    pub(super) fn into_parts(
        self,
    ) -> (
        RecordingGap,
        Option<usize>,
        Precision,
        Option<Box<ConcreteNode>>,
    ) {
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
        if self.root_action != Some(ordinal)
            || gap.code != super::RecordingGapCode::UnsupportedAction
        {
            return gap;
        }
        let concrete = self.precision == Precision::ConcreteNode
            && legacy_precision == crate::recorded::LegacyActionPrecision::Coarse;
        // Only an authenticated failed binding can identify the actual legacy
        // refusal within an enclosing Let. A coarse call alone cannot establish
        // which argument or nested action failed first.
        let resource = self.phase == FailurePhase::CircuitBody
            && self.gap.resource.as_ref().is_some_and(|resource| {
                resource.phase == crate::recorded::RecordingResourcePhase::FixedLayoutValidation
            })
            && match legacy_precision {
                crate::recorded::LegacyActionPrecision::BindingValue(index) => {
                    let binding = format!("{}.bindings[{index}].value", gap.path);
                    encloses(&binding, &self.gap.path)
                }
                _ => false,
            };
        if concrete || resource {
            let mut refined = self.gap.clone();
            if concrete && let Some(node) = &self.concrete_node {
                refined.ir_node = node.name.clone();
            }
            refined
        } else {
            gap
        }
    }
}

fn encloses(parent: &str, child: &str) -> bool {
    child.strip_prefix(parent).is_some_and(|suffix| {
        suffix.is_empty() || suffix.starts_with('.') || suffix.starts_with('[')
    })
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
