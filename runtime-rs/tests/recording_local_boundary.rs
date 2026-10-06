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

//! Public local-helper sealing, separate from generated helper admission tests.
use midnight_base_crypto::{cost_model::CostDuration, time::Timestamp};
use midnight_coin_structure::coin::{PublicAddress, TokenType};
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{
    CircuitContext, CircuitResult, ConstructorContext, ConstructorResult, RunningCost,
};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, CoinCommitment, ContractAddress, HashOutput, StateValue, constructor_cell,
};
use midnight_compact_runtime::recording::RecordingFrame;

fn context() -> CircuitContext<u64> {
    ConstructorResult::new(
        ConstructorContext::new(7),
        ChargedState::new(StateValue::Array(vec![constructor_cell(false)].into())),
    )
    .into_circuit_context(ContractAddress::default())
}

fn local(context: CircuitContext<u64>) -> CircuitResult<u64, ()> {
    CircuitResult {
        context,
        result: (),
        gas_cost: RunningCost::ZERO,
        private_transcript_outputs: vec![],
    }
}

type Mutation = (&'static str, fn(CircuitContext<u64>) -> CircuitContext<u64>);

#[test]
fn each_execution_policy_and_call_context_change_is_rejected() {
    let mutations: &[Mutation] = &[
        ("gas limit", |mut c| {
            c.gas_limit = Some(RunningCost::ZERO);
            c
        }),
        ("cost model", |mut c| {
            c.cost_model.noop_constant =
                CostDuration::from_picoseconds(c.cost_model.noop_constant.into_picoseconds() + 1);
            c
        }),
        ("coin identity", |c| c.with_coin_public_key_bytes([5; 32])),
        ("query address", |mut c| {
            c.query.address = ContractAddress(HashOutput([5; 32]));
            c
        }),
        ("own address", |mut c| {
            c.query.call_context.own_address = ContractAddress(HashOutput([5; 32]));
            c
        }),
        ("block time", |mut c| {
            c.query.call_context.tblock =
                Timestamp::from_secs(c.query.call_context.tblock.to_secs() + 1);
            c
        }),
        ("block uncertainty", |mut c| {
            c.query.call_context.tblock_err += 1;
            c
        }),
        ("parent hash", |mut c| {
            c.query.call_context.parent_block_hash = HashOutput([5; 32]);
            c
        }),
        ("caller", |mut c| {
            c.query.call_context.caller = Some(PublicAddress::Contract(ContractAddress(
                HashOutput([5; 32]),
            )));
            c
        }),
        ("balance", |mut c| {
            c.query.call_context.balance = c.query.call_context.balance.insert(TokenType::Dust, 1);
            c
        }),
        ("commitment indices", |mut c| {
            c.query.call_context.com_indices = c
                .query
                .call_context
                .com_indices
                .insert(CoinCommitment(HashOutput([5; 32])), 3);
            c
        }),
        ("last block time", |mut c| {
            c.query.call_context.last_block_time =
                Timestamp::from_secs(c.query.call_context.last_block_time.to_secs() + 1);
            c
        }),
        ("wallet frontier", |mut c| {
            c.zswap_state.first_free += 1;
            c
        }),
    ];
    for (name, mutate) in mutations {
        let outcome = RecordingFrame::new(context()).call_local(|c| Ok(local(mutate(c))));
        match outcome {
            Err(CompactError::InvalidLedgerCell(message)) => assert_eq!(
                message, "local helper changed public or Zswap execution context",
                "{name}"
            ),
            _ => panic!("{name}: expected the local-helper context rejection"),
        }
    }
}

#[test]
fn consecutive_valid_helpers_accumulate_private_effects_in_order() {
    let initial = context();
    let query = initial.query.clone();
    let first_cost = RunningCost {
        bytes_written: 3,
        ..RunningCost::ZERO
    };
    let second_cost = RunningCost {
        bytes_written: 5,
        ..RunningCost::ZERO
    };
    let (frame, first) = RecordingFrame::new(initial)
        .call_local(|mut c| {
            c.private_state += 1;
            Ok(CircuitResult {
                context: c,
                result: 17,
                gas_cost: first_cost,
                private_transcript_outputs: vec![AlignedValue::from(true)],
            })
        })
        .unwrap();
    let (frame, second) = frame
        .call_local(|mut c| {
            assert_eq!(c.private_state, 8);
            c.private_state *= 2;
            Ok(CircuitResult {
                context: c,
                result: 23,
                gas_cost: second_cost,
                private_transcript_outputs: vec![AlignedValue::from(false)],
            })
        })
        .unwrap();
    assert_eq!((first, second), (17, 23));
    let recorded = frame.finish(());
    assert_eq!(recorded.execution.context.private_state, 16);
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        vec![AlignedValue::from(true), AlignedValue::from(false)]
    );
    assert_eq!(
        recorded.execution.gas_cost,
        RunningCost {
            bytes_written: 8,
            ..RunningCost::ZERO
        }
    );
    assert_eq!(recorded.execution.context.query.state, query.state);
    assert_eq!(recorded.execution.context.query.effects, query.effects);
    assert!(recorded.public.verify_ops().is_empty());
}

#[test]
fn helper_error_is_propagated_without_constructing_a_recorded_result() {
    let result = RecordingFrame::new(context()).call_local::<(), _>(|mut c| {
        c.private_state += 1;
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    });
    assert!(matches!(
        result,
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    ));
}
