// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn probe(source: &'static str, entry: &'static str, label: String, require_old: bool) {
    // Complete DID closures use the existing whole-root qualification envelope.
    // Ordinary-worker limits have separate unchanged 2 MiB boundary tests.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || inspect(source, entry, &label, require_old))
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn complete_original_did_roots_measure_bounded_ordering_composition() {
    let source =
        include_str!("../../../../../tests/relation-composition/original-did-schema20-ir.json");
    for entry in [
        "rotateControllerKey",
        "recoverControllerKey",
        "setAlsoKnownAs",
        "setVerificationMethod",
        "removeVerificationMethod",
        "setSchnorrJubjubVerificationMethod",
        "removeSchnorrJubjubVerificationMethod",
        "setVerificationMethodRelation",
        "setService",
        "removeService",
        "deactivate",
    ] {
        probe(source, entry, format!("did-{entry}"), true);
    }
}
#[test]
fn complete_crypto_roots_measure_bounded_ordering_composition() {
    let source = include_str!("../../../../../tests/unit-composition/crypto.json");
    for entry in ["close", "verifyOnly"] {
        probe(source, entry, format!("crypto-{entry}"), true);
    }
}

#[test]
fn genuine_mixed_inputs_maximum_carrier_and_lazy_witnesses_compose_ordering() {
    let source =
        include_str!("../../../../../tests/effect-plan-ordered-comparisons-schema20-ir.json");
    for entry in ["compare", "compare128", "witnessed"] {
        probe(source, entry, format!("source-{entry}"), false);
    }
}
