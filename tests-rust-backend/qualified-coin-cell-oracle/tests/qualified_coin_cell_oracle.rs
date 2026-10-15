// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_qualified_coin_cell_oracle_fixture::{ledger_contract, types};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{CoinInfo, CoinRecipient, ContractAddress, DefaultDB, StateValue};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/qualified-coin-cell-oracle.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new()
        .insert(
            EntryPointBuf(b"write_coin".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"read_coin".to_vec()),
            ContractOperation::new(None),
        );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn coin() -> types::ShieldedCoinInfo {
    let mut nonce = [0; 32];
    nonce[..5].copy_from_slice(b"nonce");
    let mut color = [0; 32];
    color[..5].copy_from_slice(b"color");
    types::ShieldedCoinInfo {
        nonce: runtime::FixedBytes::new(nonce),
        color: runtime::FixedBytes::new(color),
        value: runtime::BoundedUint::new(42).unwrap(),
    }
}

fn recipient(left: bool) -> types::Either {
    let mut user_key = [0; 32];
    user_key[0] = 7;
    types::Either {
        is_left: left,
        left: types::ZswapCoinPublicKey {
            bytes: runtime::FixedBytes::new(if left { user_key } else { [0; 32] }),
        },
        right: types::ContractAddress {
            bytes: runtime::FixedBytes::new([0; 32]),
        },
    }
}

fn ledger_carriers(
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
) -> (CoinInfo, CoinRecipient) {
    (
        runtime::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value()),
        runtime::ledger::coin_recipient_from_compact(
            recipient.is_left,
            recipient.left.bytes,
            recipient.right.bytes,
        ),
    )
}

fn gas_matches(
    actual: runtime::context::RunningCost,
    queries: &serde_json::Value,
    reported: &serde_json::Value,
) {
    let actual = serde_json::to_value(actual).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = queries
            .as_array()
            .unwrap()
            .iter()
            .map(|query| {
                query["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(actual[dimension].as_u64().unwrap(), expected, "{dimension}");
        assert_eq!(
            actual[dimension].as_u64().unwrap(),
            reported[dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
    }
}

fn allocated(
    mut context: runtime::context::CircuitContext<()>,
    value: &types::ShieldedCoinInfo,
    target: &types::Either,
    index: u64,
) -> runtime::context::CircuitContext<()> {
    let (info, recipient) = ledger_carriers(value, target);
    context.query.call_context.com_indices = context
        .query
        .call_context
        .com_indices
        .insert(info.commitment(&recipient), index);
    context
}
fn initial() -> runtime::context::CircuitContext<()> {
    ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

#[test]
fn qualified_cell_write_matches_typescript_allocation_and_replacement() {
    let expected = oracle();
    for (name, left, index, occupied) in [
        ("right7", false, 7_u64, false),
        ("left11", true, 11, false),
        ("zero", false, 0, false),
        ("replacement", true, 17, true),
    ] {
        let row = &expected[name];
        let mut context = initial();
        assert_eq!(
            state_hex(context.query.state.get_ref().clone()),
            row["initialStateHex"]
        );
        if occupied {
            let first = coin();
            let target = recipient(false);
            let first =
                ledger_contract::write_coin(allocated(context, &first, &target, 7), first, target)
                    .unwrap();
            context = initial();
            context.query.state =
                runtime::ledger::ChargedState::new(first.context.query.state.get_ref().clone());
        }
        assert_eq!(
            state_hex(context.query.state.get_ref().clone()),
            row["beforeStateHex"]
        );
        let mut value = coin();
        if occupied {
            let mut nonce = value.nonce.into_array();
            nonce[5] = b'2';
            value.nonce = runtime::FixedBytes::new(nonce);
            value.value = runtime::BoundedUint::new(43).unwrap();
        }
        let target = recipient(left);
        let mut recording_context = initial();
        recording_context.query.state =
            runtime::ledger::ChargedState::new(context.query.state.get_ref().clone());
        let recorded = ledger_contract::recorded::write_coin(
            allocated(recording_context, &value, &target, index),
            value.clone(),
            target.clone(),
        )
        .unwrap();
        let result = ledger_contract::write_coin(
            allocated(context, &value, &target, index),
            value.clone(),
            target,
        )
        .unwrap();
        assert_eq!(
            state_hex(result.context.query.state.get_ref().clone()),
            row["afterStateHex"]
        );
        assert_eq!(
            serde_json::to_value(&result.context.query.effects).unwrap(),
            row["effects"]
        );
        assert!(result.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(recorded.execution.gas_cost, result.gas_cost);
        assert_eq!(
            recorded.execution.context.query.effects,
            result.context.query.effects
        );
        assert_eq!(
            recorded.execution.context.query.state.get_ref(),
            result.context.query.state.get_ref()
        );
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            row["writeQueries"][0]["ops"]
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
            replay.context.state.get_ref(),
            result.context.query.state.get_ref()
        );
        assert_eq!(replay.context.effects, result.context.query.effects);
        assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
        assert_eq!(row["privateCount"], 0);
        gas_matches(result.gas_cost, &row["writeQueries"], &row["writeGas"]);
        let read = ledger_contract::read_coin(result.context).unwrap().result;
        assert_eq!(read.nonce, value.nonce);
        assert_eq!(read.color, value.color);
        assert_eq!(read.value, value.value);
        assert_eq!(read.mt_index.value(), index as u128);
        assert_eq!(read.mt_index.value().to_string(), row["read"]["mt_index"]);
        assert_eq!(hex::encode(read.nonce.into_array()), row["read"]["nonce"]);
        assert_eq!(read.value.value().to_string(), row["read"]["value"]);
    }
    for (name, wrong) in [("missing", false), ("wrongRecipient", true)] {
        let value = coin();
        let right = recipient(false);
        let target = recipient(wrong);
        let context = if wrong {
            allocated(initial(), &value, &right, 7)
        } else {
            initial()
        };
        let error = ledger_contract::write_coin(context, value, target)
            .err()
            .unwrap();
        assert!(error.to_string().contains("Coin commitment not found"));
        let recorded_context = if wrong {
            allocated(initial(), &coin(), &right, 7)
        } else {
            initial()
        };
        let recorded_error =
            ledger_contract::recorded::write_coin(recorded_context, coin(), recipient(wrong))
                .err()
                .unwrap();
        assert!(
            recorded_error
                .to_string()
                .contains("Coin commitment not found")
        );
        assert!(
            expected[name]["error"]
                .as_str()
                .unwrap()
                .contains("Coin commitment not found")
        );
        assert!(expected[name]["queries"].as_array().unwrap().is_empty());
    }
    let value = coin();
    let target = recipient(false);
    let (info, recipient) = ledger_carriers(&value, &target);
    let wrong = runtime::slots::CellSlot::<bool>::new(&[0])
        .write_coin(allocated(initial(), &value, &target, 7), info, recipient)
        .err()
        .unwrap();
    assert!(
        wrong
            .to_string()
            .contains("alignment is not QualifiedShieldedCoinInfo")
    );
    let (info, recipient) = ledger_carriers(&value, &target);
    let wrong_recording = runtime::slots::CellSlot::<bool>::new(&[0])
        .record_write_coin(
            runtime::recording::RecordingFrame::new(allocated(initial(), &value, &target, 7)),
            info,
            recipient,
        )
        .err()
        .unwrap();
    assert!(
        wrong_recording
            .to_string()
            .contains("alignment is not QualifiedShieldedCoinInfo")
    );
}

#[test]
fn qualified_cell_parent_path_matches_independent_chunked_typescript() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/qualified-coin-cell-chunked-oracle.json"
    ))
    .unwrap();
    for (name, left, index, occupied) in [
        ("right7", false, 7_u64, false),
        ("left11", true, 11, false),
        ("zero", false, 0, false),
        ("replacement", true, 17, true),
    ] {
        let row = &expected[name];
        let bytes = hex::decode(row["beforeStateHex"].as_str().unwrap()).unwrap();
        let deployed: ContractState<DefaultDB> =
            midnight_serialize::tagged_deserialize(&mut bytes.as_slice()).unwrap();
        let mut context = initial();
        context.query.state = runtime::ledger::ChargedState::new(deployed.data.get_ref().clone());
        let mut value = coin();
        if occupied {
            let mut nonce = value.nonce.into_array();
            nonce[5] = b'2';
            value.nonce = runtime::FixedBytes::new(nonce);
            value.value = runtime::BoundedUint::new(43).unwrap();
        }
        let target = recipient(left);
        let (info, recipient) = ledger_carriers(&value, &target);
        let mut recording_context = initial();
        recording_context.query.state =
            runtime::ledger::ChargedState::new(context.query.state.get_ref().clone());
        let recorded = runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[1, 14])
            .record_write_coin(
                runtime::recording::RecordingFrame::new(allocated(
                    recording_context,
                    &value,
                    &target,
                    index,
                )),
                info,
                recipient.clone(),
            )
            .unwrap()
            .finish(());
        let result = runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[1, 14])
            .write_coin(allocated(context, &value, &target, index), info, recipient)
            .unwrap();
        assert_eq!(
            state_hex(result.context.query.state.get_ref().clone()),
            row["afterStateHex"]
        );
        assert_eq!(
            serde_json::to_value(&result.context.query.effects).unwrap(),
            row["effects"]
        );
        assert_eq!(recorded.execution.gas_cost, result.gas_cost);
        assert_eq!(
            recorded.execution.context.query.effects,
            result.context.query.effects
        );
        assert_eq!(
            recorded.execution.context.query.state.get_ref(),
            result.context.query.state.get_ref()
        );
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            row["writeQueries"][0]["ops"]
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
            replay.context.state.get_ref(),
            result.context.query.state.get_ref()
        );
        assert_eq!(replay.context.effects, result.context.query.effects);
        assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
        gas_matches(result.gas_cost, &row["writeQueries"], &row["writeGas"]);
        let read = runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[1, 14])
            .inspect(result.context.query.state.get_ref())
            .unwrap();
        assert_eq!(read.mt_index.value(), index as u128);
    }
}

#[test]
fn missing_allocation_preflight_is_distinct_from_vm_rejection() {
    let expected = oracle();
    let value = coin();
    let target = recipient(false);
    let missing = ledger_contract::write_coin(initial(), value.clone(), target.clone())
        .err()
        .unwrap();
    assert!(matches!(
        missing,
        runtime::CompactError::InvalidLedgerCell(_)
    ));
    assert_eq!(expected["missing"]["queryAttempts"], 0);
    assert_eq!(expected["wrongRecipient"]["queryAttempts"], 0);
    let mut context = allocated(initial(), &value, &target, 7);
    context.query.state = runtime::ledger::ChargedState::new(runtime::ledger::constructor_cell::<
        bool,
        DefaultDB,
    >(false));
    let rejected = ledger_contract::write_coin(context, value, target)
        .err()
        .unwrap();
    assert!(matches!(
        rejected,
        runtime::CompactError::LedgerQueryRejected(_)
    ));
    let mut recorded_context = allocated(initial(), &coin(), &recipient(false), 7);
    recorded_context.query.state = runtime::ledger::ChargedState::new(
        runtime::ledger::constructor_cell::<bool, DefaultDB>(false),
    );
    let recorded_rejected =
        ledger_contract::recorded::write_coin(recorded_context, coin(), recipient(false))
            .err()
            .unwrap();
    assert!(matches!(
        recorded_rejected,
        runtime::CompactError::LedgerQueryRejected(_)
    ));
    let (info, target) = ledger_carriers(&coin(), &recipient(false));
    let deep = runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[
        1, 2, 3, 4, 5, 6, 7, 8,
    ])
    .record_write_coin(
        runtime::recording::RecordingFrame::new(allocated(
            initial(),
            &coin(),
            &recipient(false),
            7,
        )),
        info,
        target,
    )
    .err()
    .unwrap();
    assert!(deep.to_string().contains("exceeds VM dup depth"));
    assert!(expected["malformed"]["error"].is_string());
    assert_eq!(expected["malformed"]["queryAttempts"], 1);
}

#[test]
fn current_generated_qualified_write_and_read_match_manual_frame_at_numeric_boundaries() {
    for left in [false, true] {
        for amount in [0, u128::MAX] {
            for index in [0, u64::MAX] {
                let mut value = coin();
                value.value = runtime::BoundedUint::new(amount).unwrap();
                let mut target = recipient(left);
                // The unselected carrier is not required to be zero. The selected
                // recipient alone determines the official commitment allocation.
                if left {
                    target.right.bytes = runtime::FixedBytes::new([91; 32]);
                } else {
                    target.left.bytes = runtime::FixedBytes::new([92; 32]);
                    target.right.bytes = runtime::FixedBytes::new([93; 32]);
                }
                let mut before = initial();
                if index != 0 {
                    let first_coin = coin();
                    let first_target = recipient(false);
                    before = ledger_contract::write_coin(
                        allocated(before, &first_coin, &first_target, 7),
                        first_coin,
                        first_target,
                    )
                    .unwrap()
                    .context;
                }
                let before_state = before.query.state.get_ref().clone();
                let make_context = || {
                    let mut context = initial();
                    context.query.state = runtime::ledger::ChargedState::new(before_state.clone());
                    allocated(context, &value, &target, index)
                };
                let native =
                    ledger_contract::write_coin(make_context(), value.clone(), target.clone())
                        .unwrap();
                let recorded = ledger_contract::recorded::write_coin(
                    make_context(),
                    value.clone(),
                    target.clone(),
                )
                .unwrap();
                // Assemble independently through the existing typed runtime slot.
                let (info, destination) = ledger_carriers(&value, &target);
                let manual =
                    runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[0])
                        .record_write_coin(
                            runtime::recording::RecordingFrame::new(make_context()),
                            info,
                            destination,
                        )
                        .unwrap()
                        .finish(());
                for execution in [&recorded.execution, &manual.execution] {
                    assert_eq!(
                        execution.context.query.state.get_ref(),
                        native.context.query.state.get_ref()
                    );
                    assert_eq!(
                        execution.context.query.effects,
                        native.context.query.effects
                    );
                    assert_eq!(execution.gas_cost, native.gas_cost);
                    assert_eq!(
                        execution.private_transcript_outputs,
                        native.private_transcript_outputs
                    );
                    assert!(execution.private_transcript_outputs.is_empty());
                    assert_eq!(
                        execution.context.circuit_zswap(),
                        native.context.circuit_zswap()
                    );
                }
                assert_eq!(
                    recorded.public.initial_intents(),
                    recorded.public.final_intents()
                );
                assert_eq!(recorded.public.initial().state.get_ref(), &before_state);
                assert_eq!(
                    serde_json::to_value(recorded.public.verify_ops()).unwrap(),
                    serde_json::to_value(manual.public.verify_ops()).unwrap()
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
                    replay.context.state.get_ref(),
                    native.context.query.state.get_ref()
                );
                assert_eq!(replay.context.effects, native.context.query.effects);
                assert_eq!(replay.gas_cost, native.gas_cost);

                let after_state = native.context.query.state.get_ref().clone();
                let read_context = || {
                    let mut context = initial();
                    context.query.state = runtime::ledger::ChargedState::new(after_state.clone());
                    context
                };
                let native_read = ledger_contract::read_coin(read_context()).unwrap();
                let recorded_read = ledger_contract::recorded::read_coin(read_context()).unwrap();
                let (frame, manual_value) =
                    runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[0])
                        .record_read(runtime::recording::RecordingFrame::new(read_context()))
                        .unwrap();
                let manual_read = frame.finish(manual_value);
                assert_eq!(native_read.result.nonce, value.nonce);
                assert_eq!(native_read.result.color, value.color);
                assert_eq!(native_read.result.value.value(), amount);
                assert_eq!(native_read.result.mt_index.value(), u128::from(index));
                for execution in [&recorded_read.execution, &manual_read.execution] {
                    assert_eq!(execution.result, native_read.result);
                    assert_eq!(
                        execution.context.query.state.get_ref(),
                        native_read.context.query.state.get_ref()
                    );
                    assert_eq!(
                        execution.context.query.effects,
                        native_read.context.query.effects
                    );
                    assert_eq!(execution.gas_cost, native_read.gas_cost);
                    assert_eq!(
                        execution.private_transcript_outputs,
                        native_read.private_transcript_outputs
                    );
                }
                assert_eq!(
                    serde_json::to_value(recorded_read.public.verify_ops()).unwrap(),
                    serde_json::to_value(manual_read.public.verify_ops()).unwrap()
                );
                let replay = recorded_read
                    .public
                    .initial()
                    .query(
                        recorded_read.public.verify_ops(),
                        None,
                        &recorded_read.execution.context.cost_model,
                    )
                    .unwrap();
                assert_eq!(
                    replay.context.state.get_ref(),
                    native_read.context.query.state.get_ref()
                );
                assert_eq!(replay.context.effects, native_read.context.query.effects);
                assert_eq!(replay.gas_cost, native_read.gas_cost);
            }
        }
    }
}

#[test]
fn current_generated_qualified_write_failures_match_manual_and_preserve_held_state() {
    for mode in 0..3 {
        let value = coin();
        let allocated_target = recipient(false);
        let mut requested_target = allocated_target.clone();
        if mode == 1 {
            requested_target = recipient(true);
        } else if mode == 2 {
            requested_target.right.bytes = runtime::FixedBytes::new([19; 32]);
        }
        let held = if mode == 0 {
            initial()
        } else {
            allocated(initial(), &value, &allocated_target, 7)
        }
        .query;
        let held_state = held.state.get_ref().clone();
        let held_effects = held.effects.clone();
        let context = || {
            let mut context = initial();
            context.query = held.clone();
            context
        };
        let native =
            ledger_contract::write_coin(context(), value.clone(), requested_target.clone())
                .err()
                .unwrap();
        let recorded = ledger_contract::recorded::write_coin(
            context(),
            value.clone(),
            requested_target.clone(),
        )
        .err()
        .unwrap();
        let (info, destination) = ledger_carriers(&value, &requested_target);
        let manual = runtime::slots::CellSlot::<types::QualifiedShieldedCoinInfo>::new(&[0])
            .record_write_coin(
                runtime::recording::RecordingFrame::new(context()),
                info,
                destination,
            )
            .err()
            .unwrap();
        assert!(matches!(
            native,
            runtime::CompactError::InvalidLedgerCell(_)
        ));
        assert!(native.to_string().contains("Coin commitment not found"));
        assert_eq!(recorded, native);
        assert_eq!(manual, native);
        assert_eq!(held.state.get_ref(), &held_state);
        assert_eq!(held.effects, held_effects);
    }
}
