// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn rows() -> Vec<Value> {
    let capture: Value = serde_json::from_str(include_str!("../../oracle/cases.json")).unwrap();
    capture["cases"].as_array().unwrap().clone()
}
fn check(row: &Value, fail: bool) {
    let label = row["id"].as_str().unwrap();
    let before = state(row);
    let private = row["privateBefore"].as_u64().unwrap();
    let id = r::OpaqueString(row["methodId"].as_str().unwrap().into());
    let point = r::ec_mul_generator(r::Field::from(
        row["expectedKey"].as_str().unwrap().parse::<u64>().unwrap(),
    ))
    .unwrap();
    let input = AlignedValue::from((id.clone(), point));
    let mut observed = observation(before.clone());
    let [nw, rw, ow, mw] = std::array::from_fn(|_| Provider::new(row, fail));
    let manual = manual::execute(&observed, private, &mw, id.clone(), point);
    let key = verifier();
    // Derive operation metadata from the public prepared-call API: no extra ledger dependency.
    let prepared_manual = manual.as_ref().ok().map(|_| {
        let fresh = manual::execute(
            &observed,
            private,
            &Provider::new(row, fail),
            id.clone(),
            point,
        )
        .unwrap();
        prepare_call(
            fresh,
            CallSpec::new("verify", key.clone(), input.clone(), r::Field::from(0u64)),
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
    let native = c::verify(observed.circuit_context(private), &nw, id.clone(), point);
    let recorded = c::recorded::verify(observed.circuit_context(private), &rw, id.clone(), point);
    let contract = c::Contract::from(ow.clone());
    let call = contract
        .recording()
        .verify_call(&observed, private, id, point);
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
        Some("admitted provider refused")
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
fn original_four_rows_add_observed_manual_preparation_and_exact_callback_order() {
    let rows = rows();
    assert_eq!(rows.len(), 4);
    for row in rows {
        check(&row, false);
    }
}
#[test]
fn provider_refusal_occurs_once_after_map_checks_and_before_point_checks() {
    let mut row = rows().into_iter().find(|r| r["id"] == "matching").unwrap();
    // Wrong point would fail later; provider refusal must remain the first error.
    row["expectedKey"] = json!("6");
    check(&row, true);
}
