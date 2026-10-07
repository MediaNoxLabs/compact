// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_jubjub_scalar_cell_fixture::ledger_contract as acc;
use r::context::ConstructorContext;

fn initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(acc::initial_state(ConstructorContext::new(17))?)
}
fn call(
    case: &Case,
    observed: &ObservedContractState,
) -> Result<(Recorded, AlignedValue), Failure> {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests-rust-backend/jubjub-scalar-cell/oracle/cases.json"
    ))?;
    let row = capture["cases"]
        .as_array()
        .ok_or("missing captured cases")?
        .iter()
        .find(|row| row["id"].as_str() == Some(case.id))
        .ok_or("unknown ACC scalar boundary")?;
    let scalar = r::Field::from_le_bytes(&hex::decode(
        row["scalar"].as_str().ok_or("missing scalar")?,
    )?)
    .ok_or("noncanonical native Field")?;
    let supplied = point(7)?;
    let native = acc::apply(observed.circuit_context(17), supplied, scalar)?;
    let value = r::fab::Value::from(native.result);
    let result = serde_json::json!({
        "x":hex::encode(r::jubjub_point_x(native.result).as_le_bytes()),
        "y":hex::encode(r::jubjub_point_y(native.result).as_le_bytes()),
        "fab":value.0.iter().map(|a|hex::encode(&a.0)).collect::<Vec<_>>()
    });
    if result != row["expected"] {
        return Err("ACC native value differs from independent original pure oracle".into());
    }
    let paired = pair(
        native,
        acc::recorded::apply(observed.circuit_context(17), supplied, scalar)?,
        AlignedValue::from((supplied, scalar)),
    )?;
    // Exercise the generated observed facade against the same installed verifier.
    let verifier = observed
        .contract()
        .operations
        .get(&midnight_onchain_state::state::EntryPointBuf(
            b"apply".to_vec(),
        ))
        .and_then(|operation| operation.latest().cloned())
        .ok_or("missing apply verifier")?;
    let typed = acc::recorded::Contract
        .apply_call(observed, 17_u64, supplied, scalar)?
        .prepare(
            verifier.clone(),
            midnight_transient_crypto::curve::Fr::from(0_u64),
        )?;
    let manual = r::transaction::prepare_call(
        acc::recorded::apply(observed.circuit_context(17_u64), supplied, scalar)?,
        r::transaction::CallSpec::new(
            "apply",
            verifier,
            (supplied, scalar),
            midnight_transient_crypto::curve::Fr::from(0_u64),
        ),
    )?;
    if format!("{typed:?}") != format!("{manual:?}") {
        return Err("ACC observed facade differs from manual preparation".into());
    }
    Ok(paired)
}
pub(super) fn fixture() -> Fixture {
    Fixture {
        operations: &["apply"],
        cases: &[
            Case {
                id: "zero",
                operation: "apply",
            },
            Case {
                id: "one",
                operation: "apply",
            },
            Case {
                id: "eight",
                operation: "apply",
            },
            Case {
                id: "q_minus_one",
                operation: "apply",
            },
            Case {
                id: "q",
                operation: "apply",
            },
            Case {
                id: "q_plus_one",
                operation: "apply",
            },
            Case {
                id: "native_max",
                operation: "apply",
            },
        ],
        initial,
        call,
    }
}
