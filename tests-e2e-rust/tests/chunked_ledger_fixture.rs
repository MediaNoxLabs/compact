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

//
// A29 executing constructor readback gate.
//
// chunked_ledger_fixture.compact declares 18 ledger fields — more than the
// 16-slot StateValue::Array cap — so the front end chunks the on-chain state
// into a nested pl-array shape (here: outer 2-slot array of 3 + 15 fields).
// Every read/write emission site derives its `idx_at_index` chain from that
// nested IR, but before A29 the `initial_state` scaffold flattened the same
// IR into a flat n-element `new_array`: the constructor then wrote nested
// paths into a flat scaffold, and the generated `ledger()` accessors could
// not read the resulting state.
//
// Byte-parity (codegen_regression) can never catch that class of bug — it
// locks the generated *text*, and a wrong-but-stable scaffold passes
// forever. This test is the semantic gate: EXECUTE the generated
// `initial_state`, then read constructor-written fields (both chunks) and
// default-seeded fields back through the generated accessors. It fails on
// any future divergence between the scaffold shape and the read/write path
// shape.

use compact_contract_chunked_ledger_fixture::{ledger, Contract};
use midnight_compact_runtime::*;

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

#[test]
fn chunked_ledger_initial_state_readback_via_ledger_accessors() {
    let contract: Contract<()> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let state = result.current_contract_state;
    let view = ledger(&state);

    // Constructor-written fields, one per chunk boundary side:
    // f00 lives in chunk [0], f16 / active in chunk [1].
    assert_eq!(view.f00().expect("f00"), 7u64);
    assert_eq!(view.f16().expect("f16"), 42u64);
    assert!(view.active().expect("active"));

    // Default-seeded fields readable across both chunks — these reads walk
    // the scaffold's nested arrays directly, so a flat (pre-A29) scaffold
    // fails here even before any constructor write.
    assert_eq!(view.f01().expect("f01"), 0u64);
    assert_eq!(view.f02().expect("f02"), 0u64);
    assert_eq!(view.f03().expect("f03"), 0u64);
    assert_eq!(view.f15().expect("f15"), 0u64);
}
