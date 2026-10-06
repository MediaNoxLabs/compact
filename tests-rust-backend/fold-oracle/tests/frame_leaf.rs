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

use compact_rust_fold_oracle_fixture::{ledger_contract, ledger_slots};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, CircuitResult, ConstructorContext, RunningCost};

fn context() -> CircuitContext<u64> {
    ledger_contract::initial_state(ConstructorContext::new(42))
        .unwrap()
        .into_circuit_context(Default::default())
}

// Independent execution of the unchanged typed slot: no frame or generated call.
fn direct(context: CircuitContext<u64>) -> Result<CircuitResult<u64, ()>, runtime::CompactError> {
    ledger_slots::c.increment(context, 0)
}

fn assert_same(a: CircuitResult<u64, ()>, b: CircuitResult<u64, ()>) {
    assert_eq!(a.context.query.state, b.context.query.state);
    assert_eq!(a.context.query.effects, b.context.query.effects);
    assert_eq!(a.context.private_state, b.context.private_state);
    assert_eq!(a.gas_cost, b.gas_cost);
    assert_eq!(a.private_transcript_outputs, b.private_transcript_outputs);
    assert_eq!(a.context.gas_limit, b.context.gas_limit);
}

#[test]
fn native_leaf_preserves_slot_state_effects_private_output_and_gas() {
    assert_same(
        ledger_contract::ping(context()).unwrap(),
        direct(context()).unwrap(),
    );
}

#[test]
fn native_leaf_preserves_zero_budget_and_malformed_state_errors() {
    for invalid in [false, true] {
        let make = || {
            let mut context = context();
            if invalid {
                context.query.state = runtime::ledger::contract_state(vec![]);
            } else {
                context.gas_limit = Some(RunningCost::ZERO);
            }
            context
        };
        assert_eq!(
            ledger_contract::ping(make()).err().unwrap(),
            direct(make()).err().unwrap()
        );
    }
}

#[test]
fn zero_increment_retains_a_real_query_and_its_cost() {
    let before = context();
    let expected_state = before.query.state.clone();
    let result = ledger_contract::ping(before).unwrap();
    assert_eq!(result.context.query.state, expected_state);
    assert_ne!(result.gas_cost, RunningCost::ZERO);
    assert!(result.private_transcript_outputs.is_empty());
}
