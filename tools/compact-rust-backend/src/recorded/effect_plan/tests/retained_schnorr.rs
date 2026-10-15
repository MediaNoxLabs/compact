// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Measured full-root outcomes after Schnorr arithmetic composes; ownership is separate.

pub(super) fn assert_retained_root_label(label: &str) {
    assert!(
        matches!(
            label,
            "crypto-close"
                | "crypto-verifyOnly"
                | "did-deactivate"
                | "did-recoverControllerKey"
                | "did-rotateControllerKey"
                | "did-setAlsoKnownAs"
                | "did-removeVerificationMethod"
                | "did-removeSchnorrJubjubVerificationMethod"
                | "did-removeService"
                | "did-setSchnorrJubjubVerificationMethod"
                | "did-setService"
                | "did-setVerificationMethod"
                | "did-setVerificationMethodRelation"
        ),
        "unmeasured retained Schnorr root: {label}"
    );
}
