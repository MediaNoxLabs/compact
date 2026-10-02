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

use compact_rust_witness_list_shapes_fixture::ledger_contract::{
    LedgerView, TryWitnesses, initial_state, push_choice, push_count, push_flag, push_packet,
    push_tag, read_choices, read_counts, read_flags, read_packets, read_tags,
};
use compact_rust_witness_list_shapes_fixture::types::{Choice, Packet};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{
    CircuitResult, ConstructorContext, RunningCost, WitnessContext, WitnessReadMeter,
};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue, metered_list_view};
use runtime::{BoundedUint, CompactError, FixedBytes};

struct ListWitness {
    populated: bool,
}

macro_rules! inspect {
    ($self:ident, $context:ident, $field:ident, $expected:expr) => {{
        let list = $context.ledger.$field()?;
        let head = list.head()?;
        let empty = list.is_empty()?;
        let length = list.length()?;
        assert_eq!(head, $expected);
        assert_eq!(empty, head.is_none());
        assert_eq!(length.value(), u128::from(!empty));
        Ok((*$context.private_state + 1, head.is_some()))
    }};
}

impl TryWitnesses<u64> for ListWitness {
    fn inspect_flags(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        inspect!(self, context, flags, self.populated.then_some(true))
    }
    fn inspect_counts(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        inspect!(
            self,
            context,
            counts,
            self.populated
                .then(|| BoundedUint::<65535>::new(42).unwrap())
        )
    }
    fn inspect_tags(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        inspect!(
            self,
            context,
            tags,
            self.populated.then_some(FixedBytes::new([1, 2, 3]))
        )
    }
    fn inspect_choices(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        inspect!(self, context, choices, self.populated.then_some(Choice::no))
    }
    fn inspect_packets(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        inspect!(
            self,
            context,
            packets,
            self.populated.then(|| Packet {
                tag: FixedBytes::new([4, 5, 6]),
                count: BoundedUint::<65535>::new(7).unwrap(),
            })
        )
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "push_flag",
        "push_count",
        "push_tag",
        "push_choice",
        "push_packet",
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

fn assert_prefix(meter: &WitnessReadMeter<'_>, queries: &serde_json::Value, count: usize) {
    let queries = queries.as_array().unwrap();
    assert_eq!(queries.len(), 3);
    let actual = serde_json::to_value(meter.gas_cost()).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = queries[..count]
            .iter()
            .map(|query| {
                query["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(
            actual[key].as_u64().unwrap(),
            expected,
            "{key} prefix {count}"
        );
    }
}

macro_rules! assert_projection {
    ($context:expr, $oracle:expr, $ty:ty, $index:expr, $expected:expr) => {{
        let meter = WitnessReadMeter::new(&$context);
        let list = metered_list_view::<$ty, _>(&meter, $index).unwrap();
        assert_eq!(list.head().unwrap(), $expected);
        assert_prefix(&meter, &$oracle["queries"], 1);
        assert_eq!(list.is_empty().unwrap(), $expected.is_none());
        assert_prefix(&meter, &$oracle["queries"], 2);
        assert_eq!(
            list.length().unwrap().value(),
            u128::from($expected.is_some())
        );
        assert_prefix(&meter, &$oracle["queries"], 3);
    }};
}

fn assert_result(actual: &CircuitResult<u64, bool>, oracle: &serde_json::Value, expected: bool) {
    assert_eq!(actual.result, expected);
    assert_eq!(actual.result, oracle["result"].as_bool().unwrap());
    assert_eq!(
        actual.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    assert_eq!(oracle["publicTranscript"], serde_json::json!([]));
    assert_eq!(oracle["reportedGas"]["readTime"], "0");
    assert_eq!(actual.private_transcript_outputs.len(), 1);
    let output = &actual.private_transcript_outputs[0];
    let atoms = output
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
    let actual_gas = serde_json::to_value(&actual.gas_cost).unwrap();
    let queries = oracle["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 3);
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
    }
}

#[test]
fn generated_list_witness_types_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-list-shapes.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(0_u64)).unwrap();
    let mut context = initial.into_circuit_context(ContractAddress::default());
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["initialState"]
    );
    let empty = ListWitness { populated: false };
    let before = &oracle["before"];
    for name in ["flags", "counts", "tags", "choices", "packets"] {
        assert_eq!(before[name]["observation"]["head"]["is_some"], false);
        assert_eq!(before[name]["observation"]["length"], "0");
    }
    assert_projection!(context, before["flags"], bool, 0, None::<bool>);
    let out = read_flags(context, &empty).unwrap();
    assert_result(&out, &before["flags"], false);
    context = out.context;
    assert_projection!(
        context,
        before["counts"],
        BoundedUint<65535>,
        1,
        None::<BoundedUint<65535>>
    );
    let out = read_counts(context, &empty).unwrap();
    assert_result(&out, &before["counts"], false);
    context = out.context;
    assert_projection!(
        context,
        before["tags"],
        FixedBytes<3>,
        2,
        None::<FixedBytes<3>>
    );
    let out = read_tags(context, &empty).unwrap();
    assert_result(&out, &before["tags"], false);
    context = out.context;
    assert_projection!(context, before["choices"], Choice, 3, None::<Choice>);
    let out = read_choices(context, &empty).unwrap();
    assert_result(&out, &before["choices"], false);
    context = out.context;
    assert_projection!(context, before["packets"], Packet, 4, None::<Packet>);
    let out = read_packets(context, &empty).unwrap();
    assert_result(&out, &before["packets"], false);
    context = out.context;
    context = push_flag(context, true).unwrap().context;
    context = push_count(context, BoundedUint::<65535>::new(42).unwrap())
        .unwrap()
        .context;
    context = push_tag(context, FixedBytes::new([1, 2, 3]))
        .unwrap()
        .context;
    context = push_choice(context, Choice::no).unwrap().context;
    context = push_packet(
        context,
        Packet {
            tag: FixedBytes::new([4, 5, 6]),
            count: BoundedUint::<65535>::new(7).unwrap(),
        },
    )
    .unwrap()
    .context;
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["populatedState"]
    );
    let populated = ListWitness { populated: true };
    let after = &oracle["after"];
    for (name, value) in [
        ("flags", serde_json::json!(true)),
        ("counts", serde_json::json!("42")),
        ("tags", serde_json::json!({"0": 1, "1": 2, "2": 3})),
        ("choices", serde_json::json!(1)),
        (
            "packets",
            serde_json::json!({"tag": {"0": 4, "1": 5, "2": 6}, "count": "7"}),
        ),
    ] {
        assert_eq!(after[name]["observation"]["head"]["is_some"], true);
        assert_eq!(after[name]["observation"]["head"]["value"], value);
        assert_eq!(after[name]["observation"]["length"], "1");
    }
    assert_projection!(context, after["flags"], bool, 0, Some(true));
    let out = read_flags(context, &populated).unwrap();
    assert_result(&out, &after["flags"], true);
    context = out.context;
    assert_projection!(
        context,
        after["counts"],
        BoundedUint<65535>,
        1,
        Some(BoundedUint::<65535>::new(42).unwrap())
    );
    let out = read_counts(context, &populated).unwrap();
    assert_result(&out, &after["counts"], true);
    context = out.context;
    assert_projection!(
        context,
        after["tags"],
        FixedBytes<3>,
        2,
        Some(FixedBytes::new([1, 2, 3]))
    );
    let out = read_tags(context, &populated).unwrap();
    assert_result(&out, &after["tags"], true);
    context = out.context;
    assert_projection!(context, after["choices"], Choice, 3, Some(Choice::no));
    let out = read_choices(context, &populated).unwrap();
    assert_result(&out, &after["choices"], true);
    context = out.context;
    assert_projection!(
        context,
        after["packets"],
        Packet,
        4,
        Some(Packet {
            tag: FixedBytes::new([4, 5, 6]),
            count: BoundedUint::<65535>::new(7).unwrap()
        })
    );
    let out = read_packets(context, &populated).unwrap();
    assert_result(&out, &after["packets"], true);
}

#[test]
fn rejected_list_query_propagates_through_generated_result() {
    let mut context = initial_state(ConstructorContext::new(0_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    context.gas_limit = Some(RunningCost::ZERO);
    let rejected = read_packets(context, &ListWitness { populated: false });
    assert!(matches!(
        rejected,
        Err(CompactError::LedgerQueryRejected(_))
    ));
}
