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

use compact_rust_composite_zswap_transfer_oracle_fixture::{
    ledger_contract as transfer, types as transfer_types,
};
use compact_rust_stateful_struct_oracle_fixture::{
    ledger_contract as planned, types as planned_types,
};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::{CoinInfo, CoinRecipient, ContractAddress, HashOutput};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};
fn bytes(n: u8) -> [u8; 32] {
    let mut b = [0; 32];
    b[0] = n;
    b
}
struct Witnesses;
macro_rules! witness_impl {
    ($contract:ident) => {
        impl $contract::Witnesses<Vec<u8>> for Witnesses {
            fn next_value(
                &self,
                ctx: WitnessContext<'_, Vec<u8>, $contract::LedgerView<'_>>,
                tag: BoundedUint<255>,
            ) -> (Vec<u8>, BoundedUint<18446744073709551615>) {
                assert_eq!(ctx.contract_address.0.0, bytes(9));
                let value = tag.value() * 10 + ctx.private_state.len() as u128;
                let mut private = ctx.private_state.clone();
                private.push(tag.value() as u8);
                (private, BoundedUint::new(value).unwrap())
            }
        }
    };
}
witness_impl!(planned);
witness_impl!(transfer);
fn rows(data: &str) -> Vec<Value> {
    serde_json::from_str(data).unwrap()
}
fn initial_private(row: &Value) -> Vec<u8> {
    serde_json::from_value(row["initialPrivateState"].clone()).unwrap()
}
fn check<T: Clone + Into<runtime::fab::AlignedValue>>(out: CircuitResult<Vec<u8>, T>, row: &Value) {
    assert_eq!(
        json!(Into::<runtime::fab::AlignedValue>::into(out.result.clone())),
        row["output"]
    );
    let expected_state: midnight_onchain_state::state::ContractState<runtime::ledger::DefaultDB> =
        midnight_serialize::tagged_deserialize(
            &mut hex::decode(row["after"].as_str().unwrap())
                .unwrap()
                .as_slice(),
        )
        .unwrap();
    assert_eq!(
        out.context.query.state.get_ref(),
        expected_state.data.get_ref()
    );
    assert_eq!(json!(out.context.private_state), row["privateState"]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(json!(out.gas_cost)[dim], sum);
    }
    let plan = out.context.circuit_zswap();
    assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
    assert_eq!(plan.outputs().len(), 1);
    assert_eq!(
        plan.inputs().len(),
        row["plan"]["inputs"].as_array().unwrap().len()
    );
    let output = &plan.outputs()[0];
    let expected = runtime::ledger::coin_info_from_compact(
        FixedBytes::new(bytes(7)),
        FixedBytes::new(bytes(8)),
        row["value"].as_str().unwrap().parse().unwrap(),
    );
    assert_eq!(output.coin, expected);
    let recipient = if row["left"].as_bool().unwrap() {
        CoinRecipient::User(runtime::ledger::CoinPublicKey(HashOutput(bytes(5))))
    } else {
        CoinRecipient::Contract(ContractAddress(HashOutput(bytes(9))))
    };
    assert_eq!(output.recipient, recipient);
    let index = row["start"].as_str().unwrap().parse::<u64>().unwrap();
    assert_eq!(output.provisional_index, index);
    assert_eq!(
        out.context
            .query
            .call_context
            .com_indices
            .get(&expected.commitment(&recipient)),
        Some(&index)
    );
    if let Some(input) = plan.inputs().first() {
        assert_eq!(input.mt_index, 3);
        assert_eq!(
            CoinInfo {
                nonce: input.nonce,
                type_: input.type_,
                value: input.value
            },
            runtime::ledger::coin_info_from_compact(
                FixedBytes::new(bytes(6)),
                FixedBytes::new(bytes(8)),
                42
            )
        );
    }
}
macro_rules! coin {
    ($types:ident,$row:expr) => {
        $types::ShieldedCoinInfo {
            nonce: FixedBytes::new(bytes(7)),
            color: FixedBytes::new(bytes(8)),
            value: BoundedUint::new($row["value"].as_str().unwrap().parse().unwrap()).unwrap(),
        }
    };
}
macro_rules! recipient {
    ($types:ident,$row:expr) => {{
        let left = $row["left"].as_bool().unwrap();
        $types::Either {
            is_left: left,
            left: $types::ZswapCoinPublicKey {
                bytes: FixedBytes::new(bytes(if left { 5 } else { 0 })),
            },
            right: $types::ContractAddress {
                bytes: FixedBytes::new(bytes(if left { 0 } else { 9 })),
            },
        }
    }};
}
#[test]
fn original_planned_native_matches_zero_nonzero_recipients_and_private_order() {
    for row in rows(include_str!(
        "../../../runtime-rs/tests/fixtures/planned-zswap-oracle.json"
    )) {
        let mut ctx = planned::initial_state(ConstructorContext::new(initial_private(&row)))
            .unwrap()
            .into_circuit_context(ContractAddress(HashOutput(bytes(9))));
        ctx.set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
            .unwrap();
        let out = planned::planned(
            ctx,
            &Witnesses,
            coin!(planned_types, row),
            recipient!(planned_types, row),
        )
        .unwrap();
        assert_eq!(
            json!({"first":out.result.first.value().to_string(),"emitted":[],"address":{"bytes":out.result.address.bytes.0},"after":out.result.after.value().to_string()}),
            row["result"]
        );
        check(out, &row);
    }
}
#[test]
fn composite_transfer_native_matches_intent_kernel_and_witness_order() {
    for row in rows(include_str!(
        "../../../runtime-rs/tests/fixtures/composite-zswap-transfer-oracle.json"
    )) {
        let mut ctx = transfer::initial_state(ConstructorContext::new(initial_private(&row)))
            .unwrap()
            .into_circuit_context(ContractAddress(HashOutput(bytes(9))));
        ctx.set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
            .unwrap();
        let input = transfer_types::QualifiedShieldedCoinInfo {
            nonce: FixedBytes::new(bytes(6)),
            color: FixedBytes::new(bytes(8)),
            value: BoundedUint::new(42).unwrap(),
            mt_index: BoundedUint::new(3).unwrap(),
        };
        let out = transfer::transfer(
            ctx,
            &Witnesses,
            input,
            coin!(transfer_types, row),
            recipient!(transfer_types, row),
            FixedBytes::new(bytes(10)),
            FixedBytes::new(bytes(11)),
        )
        .unwrap();
        assert_eq!(
            json!({"first":out.result.first.value().to_string(),"consumed":[],"claimed_input":[],"emitted":[],"claimed_output":[],"address":{"bytes":out.result.address.bytes.0},"after":out.result.after.value().to_string()}),
            row["result"]
        );
        check(out, &row);
    }
}

fn check_recorded<T: Clone + Into<runtime::fab::AlignedValue>>(
    out: runtime::recording::RecordedCircuitResult<Vec<u8>, T>,
    row: &Value,
) {
    assert_eq!(json!(out.public.verify_ops()), row["publicTranscript"]);
    let replay = out
        .public
        .initial()
        .query(
            out.public.verify_ops(),
            None,
            &out.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, out.execution.context.query.state);
    assert_eq!(replay.context.effects, out.execution.context.query.effects);
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            json!(replay.gas_cost)[dim].as_u64().unwrap().to_string(),
            row["replayGas"][dim]
        );
    }
    check(out.execution, row);
}
#[test]
fn original_planned_recorded_matches_intent_unit_witnesses_kernel_and_replay() {
    for row in rows(include_str!(
        "../../../runtime-rs/tests/fixtures/planned-zswap-oracle.json"
    )) {
        let mut ctx = planned::initial_state(ConstructorContext::new(initial_private(&row)))
            .unwrap()
            .into_circuit_context(ContractAddress(HashOutput(bytes(9))));
        ctx.set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
            .unwrap();
        let out = planned::recorded::planned(
            ctx,
            &Witnesses,
            coin!(planned_types, row),
            recipient!(planned_types, row),
        )
        .unwrap();
        check_recorded(out, &row);
    }
}
#[test]
fn composite_transfer_recorded_matches_all_unit_members_kernel_and_replay() {
    for row in rows(include_str!(
        "../../../runtime-rs/tests/fixtures/composite-zswap-transfer-oracle.json"
    )) {
        let mut ctx = transfer::initial_state(ConstructorContext::new(initial_private(&row)))
            .unwrap()
            .into_circuit_context(ContractAddress(HashOutput(bytes(9))));
        ctx.set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
            .unwrap();
        let input = transfer_types::QualifiedShieldedCoinInfo {
            nonce: FixedBytes::new(bytes(6)),
            color: FixedBytes::new(bytes(8)),
            value: BoundedUint::new(42).unwrap(),
            mt_index: BoundedUint::new(3).unwrap(),
        };
        let out = transfer::recorded::transfer(
            ctx,
            &Witnesses,
            input,
            coin!(transfer_types, row),
            recipient!(transfer_types, row),
            FixedBytes::new(bytes(10)),
            FixedBytes::new(bytes(11)),
        )
        .unwrap();
        check_recorded(out, &row);
    }
}
