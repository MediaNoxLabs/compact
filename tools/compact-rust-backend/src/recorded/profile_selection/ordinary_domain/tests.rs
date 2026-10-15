// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::recorded::RecordingGapCode;
#[test]
fn ordinary_unsupported_falls_back_but_invalid_never_does() {
    let gap = RecordingGap {
        code: RecordingGapCode::UnsupportedType,
        ir_node: "RecordedPlan".into(),
        path: "return_value.value".into(),
        detail: "independent result mismatch".into(),
        resource: None,
    };
    assert!(
        failure(effect_plan::DiagnosticPlanError::Unsupported(
            gap.clone().into()
        ))
        .unwrap()
        .is_none()
    );
    match failure(effect_plan::DiagnosticPlanError::Invalid(gap.clone())) {
        Err(RenderError::RecordingPlanInvariant { path, detail }) => {
            assert_eq!(path, gap.path);
            assert_eq!(detail, gap.detail);
        }
        _ => panic!("invalid checked plans must abort before legacy fallback"),
    }
}
