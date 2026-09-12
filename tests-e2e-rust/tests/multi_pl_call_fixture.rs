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
// Walker gap A4: multiple non-write public-ledger calls + a Cell write in one
// circuit body.
//
// `examples/multi_pl_call_fixture.compact` exports `record_update`, whose body
// is two `Counter.increment` public-ledger calls followed by a write to a
// `Uint<64>` ledger cell — the shape of midnight-did's `recordUpdate`.
//
// This test exists so `compact-contract-multi-pl-call-fixture` is actually
// compiled by `cargo build -p tests-e2e-rust --tests`: workspace membership
// alone adds no dependency edge, so without an entry in `[dev-dependencies]`
// (and this reference) the crate never reached the CI build gate. The
// generated `lib.rs` is the artifact under test — if the walker or emitter
// regresses to non-compiling Rust for this shape, this test stops building.
// The assertions additionally drive the generated ledger accessors on a
// freshly constructed state, so an emitter regression that dropped one of
// them fails here rather than only in `codegen_regression`'s byte compare.

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
fn multi_pl_call_fixture_initial_state_reads_zero() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let l = ledger(&init.current_contract_state);
    assert_eq!(l.ops().expect("ops"), 0);
    assert_eq!(l.ver().expect("ver"), 0);
    assert_eq!(l.updated().expect("updated"), 0);
}
