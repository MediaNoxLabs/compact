// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The new Map recording methods reuse the established native/TS oracle checks.
use super::*;
use runtime::context::CircuitContext;
use runtime::recording::RecordedCircuitResult;
use runtime::transaction::{Observation, ObservedContractState, RecordedCall};

type Native<T> =
    fn(CircuitContext<u64>, &CountingWitness) -> Result<CircuitResult<u64, T>, CompactError>;
type Recorded<T> = fn(
    CircuitContext<u64>,
    &CountingWitness,
) -> Result<RecordedCircuitResult<u64, T>, CompactError>;
type Observed<T> = for<'a> fn(
    &'a ObservedContractState,
    &CountingWitness,
) -> Result<RecordedCall<'a, u64, T>, CompactError>;

fn observed_member<'a>(
    state: &'a ObservedContractState,
    witness: &CountingWitness,
) -> Result<RecordedCall<'a, u64, bool>, CompactError> {
    contract::Contract::from(witness.clone())
        .recording()
        .map_member_call(state, 7)
}
fn observed_lookup<'a>(
    state: &'a ObservedContractState,
    witness: &CountingWitness,
) -> Result<RecordedCall<'a, u64, Field>, CompactError> {
    contract::Contract::from(witness.clone())
        .recording()
        .map_lookup_call(state, 7)
}
fn state() -> ObservedContractState {
    let initial = contract::initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    ObservedContractState::new(
        Default::default(),
        ContractState::new(
            initial.query.state.get_ref().clone(),
            Default::default(),
            Default::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}
fn qualify<T: Clone + Into<AlignedValue> + std::fmt::Debug + PartialEq>(
    name: &str,
    native: Native<T>,
    recorded: Recorded<T>,
    observed: Observed<T>,
    expected: fn(&Value) -> AlignedValue,
) {
    let rows: Vec<_> = oracle_rows()
        .into_iter()
        .filter(|r| r["name"] == name)
        .collect();
    assert_eq!(rows.len(), 5, "same-source captured Map rows");
    for row in rows {
        let state = state();
        let context = || {
            let mut context = state.circuit_context(7);
            if row["zeroGas"] == true {
                context.gas_limit = Some(Default::default());
            }
            context
        };
        let (a, b, w) = (
            CountingWitness::new(&row["choice"]),
            CountingWitness::new(&row["choice"]),
            CountingWitness::new(&row["choice"]),
        );
        let native = native(context(), &a);
        let recorded = recorded(context(), &b);
        // Generated observed calls have no gas-limit argument. The two explicit
        // zero-budget rows therefore qualify native/recorded calls only.
        let call = (row["zeroGas"] == false).then(|| observed(&state, &w));
        if row.get("error").is_some() {
            assert_eq!(
                native.as_ref().err().unwrap(),
                recorded.as_ref().err().unwrap()
            );
            if let Some(call) = &call {
                assert_eq!(native.as_ref().err().unwrap(), call.as_ref().err().unwrap());
            }
            // The shared checker pins refusal-before-query, zero-gas and
            // missing-cell errors independently to the captured official TS rows.
            check_result(native, &row, expected(&row));
        } else {
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            let call = call.unwrap().unwrap();
            for actual in [&recorded.execution, &call.recorded().execution] {
                assert_eq!(actual.result, native.result);
                assert_eq!(actual.context.query.state, native.context.query.state);
                assert_eq!(actual.context.query.effects, native.context.query.effects);
                assert_eq!(actual.context.private_state, native.context.private_state);
                assert_eq!(
                    actual.private_transcript_outputs,
                    native.private_transcript_outputs
                );
                assert_eq!(actual.gas_cost, native.gas_cost);
                let output: AlignedValue = actual.result.clone().into();
                assert_eq!(json!(output), row["output"]);
            }
            assert_eq!(
                json!(recorded.public.verify_ops()),
                json!(call.recorded().public.verify_ops())
            );
            assert_eq!(
                recorded.public.verify_ops().len(),
                row["publicTranscript"].as_array().unwrap().len()
            );
            for trace in [&recorded.public, &call.recorded().public] {
                assert!(!trace.verify_ops().is_empty());
                let replay = trace
                    .initial()
                    .query(trace.verify_ops(), None, &native.context.cost_model)
                    .unwrap();
                assert_eq!(replay.context.state, native.context.query.state);
                assert_eq!(replay.context.effects, native.context.query.effects);
            }
            check_result(Ok(native), &row, expected(&row));
        }
        for witness in [&a, &b] {
            assert_eq!(
                witness.key_calls.get(),
                1,
                "one callback, even for refusal/query errors"
            );
            assert_eq!(witness.gate_calls.get(), 0);
        }
        assert_eq!(w.key_calls.get(), usize::from(row["zeroGas"] == false));
        assert_eq!(w.gate_calls.get(), 0);
        assert_eq!(row["calls"], json!({"key":1,"gate":0}));
        let queried = row["choice"] != "error";
        assert_eq!(
            row["queries"].as_array().unwrap().len(),
            usize::from(queried)
        );
        assert_eq!(
            row["events"],
            if queried {
                json!(["key", "query"])
            } else {
                json!(["key"])
            }
        );
    }
}
#[test]
fn recorded_map_membership_preserves_ts_values_callback_order_and_query_errors() {
    qualify(
        "map_member",
        contract::map_member,
        contract::recorded::map_member,
        observed_member,
        |row| AlignedValue::from(row["choice"] == 7),
    );
}
#[test]
fn recorded_map_lookup_preserves_ts_missing_keys_callback_order_and_replay() {
    qualify(
        "map_lookup",
        contract::map_lookup,
        contract::recorded::map_lookup,
        observed_lookup,
        |_| AlignedValue::from(Field::from(42_u64)),
    );
}
