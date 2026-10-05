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
use compact_rust_field_observation_oracle_fixture::ledger_contract as c;
use midnight_base_crypto::fab::AlignedValue;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitContext, CircuitResult, ConstructorContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use serde_json::{Value, json};
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = [
        "direct",
        "snapshot",
        "via_field_helper",
        "via_snapshot_helper",
        "pair",
        "selected",
        "optional",
    ]
    .into_iter()
    .fold(HashMap::new(), |map, name| {
        map.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        )
    });
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut out = vec![];
    midnight_serialize::tagged_serialize(&state, &mut out).unwrap();
    hex::encode(out)
}
fn context(row: &Value) -> CircuitContext<Vec<u8>> {
    let mut ctx = c::initial_state(
        ConstructorContext::new(vec![]),
        runtime::Field::from(row["first"].as_u64().unwrap()),
        runtime::Field::from(row["second"].as_u64().unwrap()),
    )
    .unwrap()
    .into_circuit_context(ContractAddress::default());
    assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
    if row["zeroGas"] == true {
        ctx.gas_limit = Some(Default::default());
    }
    ctx
}
fn check<T: Clone + Into<AlignedValue>>(out: &CircuitResult<Vec<u8>, T>, row: &Value) {
    assert_eq!(
        json!(Into::<AlignedValue>::into(out.result.clone())),
        row["output"]
    );
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(row["before"], row["after"]);
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(json!(out.context.private_state), row["privateState"]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert!(out.context.circuit_zswap().inputs().is_empty());
    assert!(out.context.circuit_zswap().outputs().is_empty());
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(json!(out.gas_cost)[dim], sum);
        // TS wrapper exposes the last query cost; keep it separate from the sum.
        let last = row["queries"]
            .as_array()
            .unwrap()
            .last()
            .map_or("0", |q| q["gas"][dim].as_str().unwrap());
        assert_eq!(row["gas"][dim], last);
    }
}
fn compare<T: Clone + Into<AlignedValue>>(
    native: Result<CircuitResult<Vec<u8>, T>, runtime::CompactError>,
    recorded: Result<runtime::recording::RecordedCircuitResult<Vec<u8>, T>, runtime::CompactError>,
    row: &Value,
) {
    if row.get("error").is_some() {
        assert!(
            row["error"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("gas")
        );
        for error in [native.err().unwrap(), recorded.err().unwrap()] {
            assert!(
                matches!(error, runtime::CompactError::LedgerQueryRejected(_)),
                "{error:?}"
            );
            assert!(format!("{error}").to_lowercase().contains("gas"));
        }
        assert_eq!(row["queries"].as_array().unwrap().len(), 1);
        return;
    }
    let native = native.unwrap();
    let recorded = recorded.unwrap();
    check(&native, row);
    check(&recorded.execution, row);
    assert_eq!(json!(recorded.public.verify_ops()), row["publicTranscript"]);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, recorded.execution.context.query.state);
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            json!(replay.gas_cost)[dim].as_u64().unwrap().to_string(),
            row["replayGas"][dim]
        );
    }
    if row["name"] == "optional" && row["selected"] == false {
        assert!(recorded.public.verify_ops().is_empty());
    }
}
#[test]
fn direct_composite_helpers_order_and_selected_reads_match_typescript() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/field-observation-oracle.json"
    ))
    .unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 23);
    for row in rows.as_array().unwrap() {
        let selected = row["selected"].as_bool().unwrap();
        macro_rules! run { ($name:ident $(,$arg:expr)*) => { compare(c::$name(context(row) $(,$arg)*), c::recorded::$name(context(row) $(,$arg)*), row) }; }
        match row["name"].as_str().unwrap() {
            "direct" => run!(direct),
            "snapshot" => run!(snapshot),
            "via_field_helper" => run!(via_field_helper),
            "via_snapshot_helper" => run!(via_snapshot_helper),
            "pair" => run!(pair),
            "selected" => run!(selected, selected),
            "optional" => run!(optional, selected),
            _ => unreachable!(),
        }
    }
}
