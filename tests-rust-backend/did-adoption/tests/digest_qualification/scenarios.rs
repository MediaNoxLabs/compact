// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
const NAME: &str = "verifySchnorrJubjubDigestSignature";
fn rows() -> Vec<Value> {
    let capture: Value = serde_json::from_str(include_str!("../../oracle/lifecycle.json")).unwrap();
    capture["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["steps"].as_array().unwrap())
        .filter(|r| r["name"] == NAME)
        .cloned()
        .collect()
}
fn check(row: &Value, fail: bool) {
    let label = row["id"].as_str().unwrap();
    let before = state(row);
    let private = row["privateBefore"].as_u64().unwrap();
    let id = codec::string(&row["args"]["id"]);
    let digest = r::FixedVector::new([1u64, 2, 3, 4].map(r::Field::from));
    let signature = types::SchnorrSignature {
        announcement: codec::point("2"),
        response: codec::field_hex(row["responseHex"].as_str().unwrap()),
    };
    let input = AlignedValue::from((id.clone(), digest.clone(), signature.clone()));
    let mut observed = observation(before.clone());
    let [nw, rw, ow, mw] = std::array::from_fn(|_| Provider::new(fail));
    let manual = manual::execute(
        &observed,
        private,
        &mw,
        id.clone(),
        digest.clone(),
        signature.clone(),
    );
    let key = verifier();
    let prepared_manual = manual.as_ref().ok().map(|_| {
        let fresh = manual::execute(
            &observed,
            private,
            &Provider::new(fail),
            id.clone(),
            digest.clone(),
            signature.clone(),
        )
        .unwrap();
        prepare_call(
            fresh,
            CallSpec::new(NAME, key.clone(), input.clone(), r::Field::from(0u64)),
        )
        .unwrap()
    });
    if let Some(prepared) = &prepared_manual {
        let mut contract = before.clone();
        contract.operations = contract
            .operations
            .insert(prepared.entry_point.clone(), prepared.op.clone());
        observed = observation(contract);
    }
    let native = c::verifySchnorrJubjubDigestSignature(
        observed.circuit_context(private),
        &nw,
        id.clone(),
        digest.clone(),
        signature.clone(),
    );
    let recorded = c::recorded::verifySchnorrJubjubDigestSignature(
        observed.circuit_context(private),
        &rw,
        id.clone(),
        digest.clone(),
        signature.clone(),
    );
    let contract = c::Contract::from(ow.clone());
    let call = contract
        .recording()
        .verifySchnorrJubjubDigestSignature_call(&observed, private, id, digest, signature);
    for provider in [&nw, &rw, &ow, &mw] {
        assert_eq!(
            json!(*provider.calls.borrow()),
            row["witnessCalls"],
            "{label}"
        );
        assert_eq!(
            *provider.private.borrow(),
            vec![private; row["witnessCalls"].as_array().unwrap().len()],
            "{label}"
        );
    }
    let error = if fail {
        Some("reduction failure")
    } else {
        row["error"]
            .as_str()
            .map(|e| e.strip_prefix("failed assert: ").unwrap_or(e))
    };
    if let Some(message) = error {
        let expected = r::CompactError::AssertionFailed(message.into());
        assert_eq!(native.err().unwrap(), expected, "native {label}");
        assert_eq!(recorded.err().unwrap(), expected, "recorded {label}");
        assert_eq!(call.err().unwrap(), expected, "observed {label}");
        assert_eq!(manual.err().unwrap(), expected, "manual {label}");
        assert_eq!(observed.contract().data, before.data);
        return;
    }
    let native = native.unwrap();
    let recorded = recorded.unwrap();
    let call = call.unwrap();
    let manual = manual.unwrap();
    assert_eq!(native.context.query.state, before.data);
    for result in [&recorded, call.recorded(), &manual] {
        parity(result, &manual, &native, row);
    }
    let prepared = call.prepare(key, r::Field::from(0u64)).unwrap();
    assert_eq!(prepared.input, input);
    assert_eq!(prepared.output, AlignedValue::from(()));
    assert_eq!(
        format!("{prepared:?}"),
        format!("{:?}", prepared_manual.unwrap())
    );
}
#[test]
fn original_five_digest_rows_add_observed_manual_preparation_and_exact_callback_order() {
    let rows = rows();
    assert_eq!(rows.len(), 5);
    for row in rows {
        check(&row, false);
    }
}
#[test]
fn provider_refusal_precedes_signature_verification_and_preserves_callback_prefix() {
    // The original wrong-key case reaches the witness then fails signature verification.
    // Override only the provider to prove its refusal is propagated before that later failure.
    let row = rows()
        .into_iter()
        .find(|r| r["id"] == "read-wrong-key")
        .unwrap();
    check(&row, true);
}
