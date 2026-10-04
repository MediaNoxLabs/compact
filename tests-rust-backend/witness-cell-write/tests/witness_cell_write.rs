// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_witness_cell_write_fixture::ledger_contract::{
    Contract, LedgerView, TryWitnesses, Witnesses, initial_state, read_cell, write_nested_twice,
    write_secret, write_twice,
};
use compact_rust_witness_cell_write_fixture::ledger_slots;
use midnight_compact_runtime::context::{
    CircuitFrame, CircuitResult, ConstructorContext, RunningCost, WitnessContext, WitnessReadMeter,
};
use midnight_compact_runtime::ledger::StateValue;
use midnight_compact_runtime::ledger::{
    ContractAddress, DefaultDB, query_cell_at_path, read_root_cell,
};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_compact_runtime::{CompactError, Field};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "write_secret",
        "write_twice",
        "write_nested_twice",
        "read_cell",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn normalized_verify_ops(recorded: &RecordedCircuitResult<u64, ()>) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("alignment")
                    && let Some(serde_json::Value::Array(chunks)) = object.get_mut("value")
                {
                    for chunk in chunks {
                        if let serde_json::Value::Array(bytes) = chunk {
                            let bytes = bytes
                                .iter()
                                .map(|byte| byte.as_u64().unwrap() as u8)
                                .collect::<Vec<_>>();
                            *chunk = serde_json::json!({ "bytesHex": hex::encode(bytes) });
                        }
                    }
                }
                for child in object.values_mut() {
                    normalize(child);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    normalize(child);
                }
            }
            _ => {}
        }
    }
    let mut ops = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    normalize(&mut ops);
    ops
}

struct Secret;

struct FallibleSecret;

impl TryWitnesses<u64> for FallibleSecret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> Result<(u64, Field), CompactError> {
        let current = context.ledger.cell()?;
        Ok(secret_logic(*context.private_state, current, seed))
    }
}

impl Witnesses<u64> for Secret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        assert_eq!(seed, Field::from(2_u64));
        secret_logic(*context.private_state, context.ledger.cell().unwrap(), seed)
    }
}

fn secret_logic(private_state: u64, current_cell: Field, seed: Field) -> (u64, Field) {
    let expected_cell = if private_state == 7 { 0 } else { 9 };
    assert_eq!(current_cell, Field::from(expected_cell));
    (private_state + 1, seed + Field::from(private_state))
}

#[test]
fn fallible_witness_succeeds_in_native_recorded_and_facade_calls() {
    let seed = Field::from(2_u64);
    let context = || {
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let legacy = write_secret(context(), &Secret, seed).unwrap();
    let fallible = write_secret(context(), &FallibleSecret, seed).unwrap();
    assert_eq!(fallible.context.private_state, legacy.context.private_state);
    assert_eq!(fallible.gas_cost, legacy.gas_cost);
    assert_eq!(
        fallible.private_transcript_outputs,
        legacy.private_transcript_outputs
    );
    assert_eq!(
        state_hex(fallible.context.query.state.get_ref().clone()),
        state_hex(legacy.context.query.state.get_ref().clone()),
    );
    let facade = Contract::from(FallibleSecret)
        .write_secret(context(), seed)
        .unwrap();
    assert_eq!(facade.gas_cost, legacy.gas_cost);
    let recorded = Contract::from(FallibleSecret)
        .recording()
        .write_secret(context(), seed)
        .unwrap();
    assert_eq!(recorded.execution.gas_cost, legacy.gas_cost);
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        legacy.private_transcript_outputs
    );
}

#[test]
fn fallible_witness_rejection_returns_error_in_native_and_recorded_calls() {
    let context = || {
        let mut context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        context.gas_limit = Some(RunningCost::ZERO);
        context
    };
    let seed = Field::from(2_u64);
    assert!(matches!(
        write_secret(context(), &FallibleSecret, seed),
        Err(CompactError::LedgerQueryRejected(_))
    ));
    assert!(matches!(
        Contract::from(FallibleSecret)
            .recording()
            .write_secret(context(), seed),
        Err(CompactError::LedgerQueryRejected(_))
    ));
}

fn assert_oracle_output(write: CircuitResult<u64, ()>, oracle: &serde_json::Value) {
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterCall"],
    );
    let actual_gas = serde_json::to_value(write.gas_cost).unwrap();
    let queries = oracle["queries"].as_array().unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = queries
            .iter()
            .map(|query| {
                query["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(actual_gas[key].as_u64().unwrap(), total, "{key} total gas");
        assert_eq!(
            oracle["reportedGas"][key],
            queries.last().unwrap()["gasCost"][key],
            "TypeScript reports the final query's {key}",
        );
    }
    assert_eq!(
        write.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    let expected_outputs = oracle["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(
        write.private_transcript_outputs.len(),
        expected_outputs.len()
    );
    for (output, expected) in write
        .private_transcript_outputs
        .iter()
        .zip(expected_outputs)
    {
        let atoms = output
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
        assert_eq!(
            serde_json::to_value(&output.alignment).unwrap(),
            expected["alignment"]
        );
    }
    let read = read_cell(write.context).unwrap();
    let expected_cell: u64 = oracle["cell"].as_str().unwrap().parse().unwrap();
    assert_eq!(read.result, Field::from(expected_cell));
}

#[test]
fn witnessed_cell_writes_keep_ledger_and_private_effects_in_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-cell-write-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["single"]["afterInit"],
    );
    let single = write_secret(context, &Secret, Field::from(2_u64)).unwrap();
    assert_oracle_output(single, &oracle["single"]);

    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let twice = write_twice(context, &Secret, Field::from(2_u64)).unwrap();
    assert_oracle_output(twice, &oracle["twice"]);
}

#[test]
fn witnessed_writes_record_private_values_and_public_operations_in_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-cell-write-ts-output.json"
    ))
    .unwrap();
    let seed = Field::from(2_u64);
    let native = write_twice(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &Secret,
        seed,
    )
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let recorded = Contract::from(Secret)
        .recording()
        .write_twice(context, seed)
        .unwrap();

    assert_eq!(
        recorded.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["twice"]["afterCall"],
    );
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 2);
    assert_eq!(recorded.public.verify_ops().len(), 6);
    assert_eq!(
        normalized_verify_ops(&recorded),
        oracle["twice"]["publicTranscript"],
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        read_root_cell::<Field, _>(native.context.query.state.get_ref(), 0).unwrap(),
        read_root_cell::<Field, _>(replay.context.state.get_ref(), 0).unwrap()
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        replay.context.effects
    );
}

#[test]
fn nested_witnessed_writes_record_the_same_private_and_public_effects() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-cell-write-ts-output.json"
    ))
    .unwrap();
    let seed = Field::from(2_u64);
    let native = write_nested_twice(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &Secret,
        seed,
    )
    .unwrap();
    let native_gas = native.gas_cost;
    assert_oracle_output(native, &oracle["nested"]);

    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let recorded = Contract::from(Secret)
        .recording()
        .write_nested_twice(context, seed)
        .unwrap();
    assert_eq!(recorded.execution.context.private_state, 9);
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 2);
    assert_eq!(recorded.execution.gas_cost, native_gas);
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["nested"]["afterCall"],
    );
    assert_eq!(recorded.public.verify_ops().len(), 6);
    assert_eq!(
        normalized_verify_ops(&recorded),
        oracle["nested"]["publicTranscript"],
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        read_root_cell::<Field, _>(recorded.execution.context.query.state.get_ref(), 0).unwrap(),
        Field::from(10_u64)
    );
}

#[test]
fn contract_facade_exposes_witnessed_circuits_with_typed_arguments() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-cell-write-ts-output.json"
    ))
    .unwrap();
    let contract = Contract::from(Secret);
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let result = contract.write_secret(context, Field::from(2_u64)).unwrap();
    assert_oracle_output(result, &oracle["single"]);
}

#[test]
fn native_frame_preserves_witness_and_nested_call_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-cell-write-ts-output.json"
    ))
    .unwrap();
    let seed = Field::from(2_u64);
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (frame, secret) = CircuitFrame::new(context).witness_metered(|context, meter| {
        let current = meter.read_cell::<Field>(&[0]).unwrap();
        secret_logic(context.private_state, current, seed)
    });
    let (frame, ()) = frame
        .apply(|context| ledger_slots::cell.write(context, secret))
        .unwrap();
    let framed = frame.finish(());
    let native = write_secret(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &Secret,
        seed,
    )
    .unwrap();
    assert_eq!(framed.context.private_state, native.context.private_state);
    assert_eq!(framed.context.query.state, native.context.query.state);
    assert_eq!(framed.context.query.effects, native.context.query.effects);
    assert_eq!(framed.gas_cost, native.gas_cost);
    assert_eq!(
        framed.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_oracle_output(framed, &oracle["single"]);

    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (frame, ()) = CircuitFrame::new(context)
        .apply(|context| write_secret(context, &Secret, seed))
        .unwrap();
    let (frame, ()) = frame
        .apply(|context| write_secret(context, &Secret, seed))
        .unwrap();
    let framed = frame.finish(());
    let native = write_nested_twice(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &Secret,
        seed,
    )
    .unwrap();
    assert_eq!(framed.context.private_state, native.context.private_state);
    assert_eq!(framed.context.query.state, native.context.query.state);
    assert_eq!(framed.context.query.effects, native.context.query.effects);
    assert_eq!(framed.gas_cost, native.gas_cost);
    assert_eq!(
        framed.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_oracle_output(framed, &oracle["nested"]);
}

#[test]
fn native_frame_aborts_before_a_later_step_on_error() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut later_step_ran = false;
    let result = CircuitFrame::new(context)
        .apply(|context| write_secret(context, &Secret, Field::from(2_u64)))
        .and_then(|(frame, ())| {
            frame.apply(
                |_| -> Result<CircuitResult<u64, (), DefaultDB>, CompactError> {
                    Err(CompactError::AssertionFailed("stop".into()))
                },
            )
        })
        .and_then(|(frame, ())| {
            later_step_ran = true;
            frame.apply(|context| ledger_slots::cell.write(context, Field::from(99_u64)))
        });
    assert!(matches!(result, Err(CompactError::AssertionFailed(message)) if message == "stop"));
    assert!(!later_step_ran);
}

#[test]
fn witness_read_meter_charges_each_successful_read() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (query, value) = query_cell_at_path::<Field, _>(
        &context.query,
        &[0],
        context.gas_limit,
        &context.cost_model,
    )
    .unwrap();
    assert_eq!(value, Field::from(0_u64));
    let expected_cost = query.gas_cost;
    let (frame, ()) = CircuitFrame::new(context).witness_metered(|context, meter| {
        assert_eq!(meter.read_cell::<Field>(&[0]).unwrap(), Field::from(0_u64));
        assert_eq!(meter.read_cell::<Field>(&[0]).unwrap(), Field::from(0_u64));
        (context.private_state, ())
    });
    assert_eq!(frame.finish(()).gas_cost, expected_cost + expected_cost);

    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let meter = WitnessReadMeter::new(&context);
    assert!(meter.read_cell::<Field>(&[1]).is_err());
    assert_eq!(
        meter.gas_cost(),
        midnight_compact_runtime::context::RunningCost::ZERO
    );
    let (frame, ()) =
        CircuitFrame::new(context).witness_metered(|context, _meter| (context.private_state, ()));
    assert_eq!(
        frame.finish(()).gas_cost,
        midnight_compact_runtime::context::RunningCost::ZERO,
    );
}

#[test]
fn gas_limit_guards_each_ledger_query_while_observed_cost_sums_queries() {
    let mut context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (query, _) =
        query_cell_at_path::<Field, _>(&context.query, &[0], None, &context.cost_model).unwrap();
    let one_read = query.gas_cost;
    assert!(one_read.read_time > RunningCost::ZERO.read_time);
    let one_query_limit = RunningCost {
        read_time: one_read.read_time,
        compute_time: one_read.compute_time,
        bytes_written: u64::MAX,
        bytes_deleted: u64::MAX,
    };
    context.gas_limit = Some(one_query_limit);

    let (frame, ()) = CircuitFrame::new(context)
        .try_witness_metered(|context, meter| {
            assert_eq!(meter.read_cell::<Field>(&[0])?, Field::from(0_u64));
            assert_eq!(meter.read_cell::<Field>(&[0])?, Field::from(0_u64));
            Ok((context.private_state, ()))
        })
        .unwrap();
    let result = frame.finish(());
    assert_eq!(result.gas_cost, one_read + one_read);
    assert!(result.gas_cost.read_time > one_query_limit.read_time);

    // Ordinary circuit reads use the same upstream per-query limit.
    let first = read_cell(result.context).unwrap();
    let second = read_cell(first.context).unwrap();
    assert_eq!(first.gas_cost, one_read);
    assert_eq!(second.gas_cost, one_read);

    let mut rejected = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    rejected.gas_limit = Some(RunningCost {
        read_time: RunningCost::ZERO.read_time,
        ..one_query_limit
    });
    assert!(matches!(
        write_secret(rejected, &FallibleSecret, Field::from(2_u64)),
        Err(CompactError::LedgerQueryRejected(_))
    ));
}
