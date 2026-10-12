// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Measured full-root outcomes after Schnorr arithmetic composes; ownership is separate.
use super::*;

pub(super) fn remaining_map_member_path(label: &str) -> Option<&'static str> {
    match label {
        "crypto-close"
        | "crypto-verifyOnly"
        | "did-deactivate"
        | "did-recoverControllerKey"
        | "did-rotateControllerKey"
        | "did-setAlsoKnownAs" => None,
        "did-removeVerificationMethod"
        | "did-removeSchnorrJubjubVerificationMethod"
        | "did-removeService" => Some("actions[0].action.actions[1].condition"),
        "did-setSchnorrJubjubVerificationMethod" | "did-setService" => {
            Some("actions[0].action.action.actions[2].then.actions[0].condition.body")
        }
        "did-setVerificationMethod" => {
            Some("actions[0].action.action.actions[3].then.actions[0].condition.body")
        }
        "did-setVerificationMethodRelation" => Some(
            "actions[0].action.action.action.actions[2].condition.callee[verificationMethodExists].return_value.value.condition",
        ),
        _ => panic!("unmeasured retained Schnorr root: {label}"),
    }
}

pub(super) fn assert_map_member_gap(label: &str, gap: &RecordingGap) {
    let path = remaining_map_member_path(label)
        .unwrap_or_else(|| panic!("{label}: complete retained root must compose: {gap:?}"));
    assert_eq!(gap.ir_node, "Expr::MapMember", "{label}: {gap:?}");
    assert_eq!(
        gap.detail, "Expr::MapMember is not supported in this recorded expression",
        "{label}: {gap:?}"
    );
    assert_eq!(gap.path, path, "{label}: {gap:?}");
}
