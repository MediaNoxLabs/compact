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
// Registration gate for the multi-pl-call fixture crate.
//
// `multi_pl_call_fixture.compact` is registered in `codegen_regression`'s
// FIXTURES table, so the byte-parity gate byte-compares its committed
// `lib.rs`. Byte-parity alone does NOT prove the emitted Rust compiles —
// that only happens if the crate is also a `tests-e2e-rust` dev-dependency,
// which is what pulls it into `cargo build -p tests-e2e-rust --tests` (see
// the comment in `tests-e2e-rust/Cargo.toml` and the invariant test
// `every_fixture_crate_is_a_dev_dependency` in `codegen_regression.rs`).
//
// This file is the honest reference that keeps the dev-dependency edge from
// being removed as "unused": it imports the generated crate and executes its
// generated `initial_state` scaffold, reading every ledger cell back through
// the generated accessors. The constructor writes nothing, so all three
// cells (two `Counter`s and one `Uint<64>`) must read back as zero.

use compact_contract_multi_pl_call_fixture::{ledger, Contract};
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
fn multi_pl_call_initial_state_starts_at_zero() {
    let contract: Contract<()> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let view = ledger(&result.current_contract_state);

    assert_eq!(view.ops().expect("ops"), 0u64);
    assert_eq!(view.ver().expect("ver"), 0u64);
    assert_eq!(view.updated().expect("updated"), 0u64);
}
