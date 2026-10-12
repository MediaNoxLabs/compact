// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Measured next gap shared by retained full DID/crypto source qualifications.
use super::*;

pub(super) fn assert_wide_field_cast_gap(label: &str, gap: &RecordingGap) {
    let prefix = match label {
        "crypto-close" | "crypto-verifyOnly" => "actions[0].callee[authorize].actions[1]",
        "did-deactivate" => "actions[0].callee[assertController].actions[1]",
        "did-recoverControllerKey" => {
            "actions[0].action.actions[0].callee[assertRecoveryCanUpdate].actions[1]"
        }
        "did-rotateControllerKey"
        | "did-removeVerificationMethod"
        | "did-removeSchnorrJubjubVerificationMethod"
        | "did-removeService" => {
            "actions[0].action.actions[0].callee[assertControllerCanUpdate].actions[0].callee[assertController].actions[1]"
        }
        "did-setAlsoKnownAs"
        | "did-setVerificationMethod"
        | "did-setSchnorrJubjubVerificationMethod"
        | "did-setService" => {
            "actions[0].action.action.actions[0].callee[assertControllerCanUpdate].actions[0].callee[assertController].actions[1]"
        }
        "did-setVerificationMethodRelation" => {
            "actions[0].action.action.action.actions[0].callee[assertControllerCanUpdate].actions[0].callee[assertController].actions[1]"
        }
        _ => panic!("unmeasured retained Schnorr root: {label}"),
    };
    let suffix = "callee[schnorrVerifyDigest].actions[0].callee[schnorrVerify].actions[0].action.action.action.action.action.action.action.action.actions[1].condition.left.right";
    assert_eq!(gap.ir_node, "RecordedPlan", "{label}: {gap:?}");
    assert_eq!(
        gap.detail, "Field cast requires unsigned input",
        "{label}: {gap:?}"
    );
    assert_eq!(gap.path, format!("{prefix}.{suffix}"), "{label}: {gap:?}");
}
