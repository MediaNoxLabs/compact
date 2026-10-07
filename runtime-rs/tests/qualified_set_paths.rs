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

use midnight_compact_runtime as runtime;
use midnight_onchain_vm::ops::Op;
use runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use runtime::ledger::{
    self, ChargedState, CoinInfo, CoinRecipient, ContractAddress, DefaultDB, StateValue,
};
use runtime::recording::RecordingFrame;
use runtime::{CompactError, FixedBytes};

type QualifiedCell = (FixedBytes<32>, FixedBytes<32>, u128, u64);

fn coin() -> CoinInfo {
    ledger::coin_info_from_compact(FixedBytes([1; 32]), FixedBytes([2; 32]), 42)
}

fn recipient() -> CoinRecipient {
    ledger::coin_recipient_from_compact(true, FixedBytes([3; 32]), FixedBytes([0; 32]))
}

fn initial(path: &[u8], allocated: Option<u64>) -> CircuitContext<()> {
    let mut state = ledger::constructor_set::<DefaultDB>();
    for &index in path.iter().rev() {
        let mut fields = vec![ledger::constructor_cell(0_u64); index as usize];
        fields.push(state);
        state = StateValue::Array(fields.into());
    }
    let mut context = ConstructorResult::new(ConstructorContext::new(()), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default());
    if let Some(index) = allocated {
        context.query.call_context.com_indices = context
            .query
            .call_context
            .com_indices
            .insert(coin().commitment(&recipient()), index);
    }
    context
}

fn insert_and_check(path: &[u8], index: u64) {
    let native = initial(path, Some(index))
        .insert_qualified_coin_set::<QualifiedCell>(path, coin(), recipient())
        .unwrap();
    let frame = RecordingFrame::new(initial(path, Some(index)))
        .insert_qualified_coin_set::<QualifiedCell>(path, coin(), recipient())
        .unwrap();
    let recorded = frame.finish(());
    let expected = (FixedBytes([1; 32]), FixedBytes([2; 32]), 42_u128, index);
    let actual = ledger::member_set(
        &native.context.query,
        path,
        expected,
        None,
        &native.context.cost_model,
    )
    .unwrap();
    assert!(actual.1, "must store the real allocated index");
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &native.context.cost_model,
        )
        .unwrap();
    assert_eq!(native.context.query.state, replay.context.state);
    assert_eq!(native.context.query.effects, replay.context.effects);
    assert_eq!(recorded.public.verify_ops().len(), 10);
    assert!(
        matches!(recorded.public.verify_ops()[1], Op::Dup { n } if usize::from(n) == 2 * path.len() + 2)
    );
}

#[test]
fn allocated_root_set_preserves_known_positive() {
    insert_and_check(&[0], 7);
}

#[test]
fn allocated_chunked_set_executes_native_and_recorded_with_real_index() {
    insert_and_check(&[1, 14], 11);
}

#[test]
fn empty_root_and_largest_encodable_context_depth_execute() {
    insert_and_check(&[], 0);
    insert_and_check(&[0; 6], u64::MAX);
}

#[test]
fn unencodable_context_depth_refuses_before_commitment_lookup() {
    for length in [7, 15, 16, 255, 256] {
        let path = vec![0; length];
        let context: CircuitContext<()> = ConstructorResult::new(
            ConstructorContext::new(()),
            ChargedState::new(StateValue::Null),
        )
        .into_circuit_context(ContractAddress::default());
        let before = context.query.state.clone();
        let result = ledger::insert_qualified_coin_set::<QualifiedCell, _>(
            &context.query,
            path.as_slice(),
            coin(),
            recipient(),
            None,
            &context.cost_model,
        );
        assert!(
            matches!(result, Err(CompactError::LedgerQueryRejected(message))
            if message == "Execution(BoundsExceeded)")
        );
        assert_eq!(context.query.state, before);
        assert!(
            matches!(RecordingFrame::new(context).insert_qualified_coin_set::<QualifiedCell>(
            path.as_slice(), coin(), recipient()), Err(CompactError::LedgerQueryRejected(message))
            if message == "Execution(BoundsExceeded)")
        );
    }
}

#[test]
fn valid_chunked_path_still_requires_actual_commitment_allocation() {
    let path = &[1, 14];
    for wrong_recipient in [false, true] {
        let context = initial(path, if wrong_recipient { Some(7) } else { None });
        let target = if wrong_recipient {
            ledger::coin_recipient_from_compact(true, FixedBytes([4; 32]), FixedBytes([0; 32]))
        } else {
            recipient()
        };
        let native =
            context.insert_qualified_coin_set::<QualifiedCell>(path, coin(), target.clone());
        assert!(
            matches!(native, Err(CompactError::InvalidLedgerCell(message))
            if message.contains("Coin commitment not found"))
        );
        let recorded =
            RecordingFrame::new(initial(path, if wrong_recipient { Some(7) } else { None }))
                .insert_qualified_coin_set::<QualifiedCell>(path, coin(), target);
        assert!(
            matches!(recorded, Err(CompactError::InvalidLedgerCell(message))
            if message.contains("Coin commitment not found"))
        );
    }
}
