// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Internal lowering provenance; display strings never decide diagnostic priority.
#[cfg(test)]
use super::PlanError;
use super::RecordingGap;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum Precision {
    ConcreteNode,
    #[default]
    Unclassified,
}

#[cfg(test)]
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
    #[cfg(test)]
    pub(super) root_action: Option<usize>,
    #[cfg(test)]
    pub(super) precision: Precision,
    #[cfg(test)]
    pub(super) phase: FailurePhase,
    pub(super) concrete_node: Option<Box<ConcreteNode>>,
}

impl From<RecordingGap> for LoweringRejection {
    fn from(gap: RecordingGap) -> Self {
        Self {
            gap,
            #[cfg(test)]
            root_action: None,
            #[cfg(test)]
            precision: Precision::Unclassified,
            #[cfg(test)]
            phase: FailurePhase::OutsideBody,
            concrete_node: None,
        }
    }
}

impl LoweringRejection {
    /// Publish the checked compiler's actual failed node without parsing paths
    /// or rerunning lowering. Resource and outside-body failures retain their
    /// original typed gap when no concrete expression label was captured.
    pub(in crate::recorded) fn into_checked_gap(mut self) -> RecordingGap {
        if let Some(node) = self.concrete_node {
            self.gap.ir_node = node.name;
        }
        self.gap
    }

    /// Consume the historical gap for a test-only diagnostic projection.
    #[cfg(test)]
    pub(in crate::recorded) fn into_gap(self) -> RecordingGap {
        self.gap
    }

    /// Historical callers discard provenance without lowering again.
    #[cfg(test)]
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

    // Historical Unit projection remains solely for diagnostic oracle tests.
    #[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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
