// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//
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
// Inline-call type-scope executing gate.
//
// `inline_type_scope_fixture.compact` inlines a non-exported impure helper into
// its caller. The helper's formal types must drive the native-hash
// scalar-vs-aggregate decision, not the caller's same-named bindings:
//
//   - `checkScalarScope`: the helper's `v: Field` collides with the caller's
//     `v: Vector<2, Field>`. If the caller's type leaked in, the scalar `x`
//     was flattened (`x[0]`) — `cargo build` fails (E0608). Type-checking this
//     crate is the compile half of the gate.
//   - `checkAggScope`: the helper's `w: Vector<4, Field>` collides with the
//     caller's shorter `w: Vector<2, Field>`. If the caller's type leaked in,
//     only two of the four leaves were hashed — code that COMPILES but is
//     silently wrong. This test seeds `hashCell` with the correct four-leaf
//     hash, so a truncated flatten fails the helper's assert.
//   - `checkNoCollisionScope`: with no caller binding of the helper's formal
//     name, the helper's own `Vector<2, Field>` was not recovered and the
//     whole `[Fr; 2]` was wrapped in `AlignedValue::from` (no `From` impl).
//
// Byte-parity (`codegen_regression`) fixes the text; this test is the semantic
// gate that would catch a plausible-but-wrong flatten.
//
// The private-state type is `()`; passing that unit into `CircuitContext::new`
// is the correct semantic use even though clippy's `unit_arg` lint flags it.
#![allow(clippy::unit_arg)]

use compact_contract_inline_type_scope_fixture::Contract;
use midnight_compact_runtime::*;

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// Every inlined helper hashes the argument it is invoked with, and `setHash`
/// seeds the ledger cell the helper compares against. A wrong scalar-vs-
/// aggregate decision makes `checkAggScope`'s assert fail (two leaves hashed
/// instead of four); a wrong scalar decision makes the crate fail to compile.
#[test]
fn inlined_helper_formals_use_the_callee_type_scope() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    // Scalar helper formal (`v: Field`) vs the caller's `v: Vector<2, Field>`.
    // A leak indexes the scalar `x` in Rust (E0608); the fixed form hashes it
    // as one Field leaf.
    let x = Fr::from(9u64);
    let scalar_hash =
        midnight_compact_runtime::std_lib::persistent_hash_aligned(&[AlignedValue::from(x)]);
    let seeded = contract
        .set_hash(ctx.clone(), scalar_hash)
        .expect("set_hash (scalar)");
    contract
        .check_scalar_scope(seeded.context, [Fr::from(1u64), Fr::from(2u64)], x)
        .expect("scalar helper formal must resolve to the callee's Field");

    // Aggregate helper formal (`w: Vector<4, Field>`) vs the caller's shorter
    // `w: Vector<2, Field>`: four leaves must be hashed, not two.
    let z = [
        Fr::from(1u64),
        Fr::from(2u64),
        Fr::from(3u64),
        Fr::from(4u64),
    ];
    let agg_hash = midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
        AlignedValue::from(z[0]),
        AlignedValue::from(z[1]),
        AlignedValue::from(z[2]),
        AlignedValue::from(z[3]),
    ]);
    let seeded = contract
        .set_hash(ctx.clone(), agg_hash)
        .expect("set_hash (aggregate)");
    contract
        .check_agg_scope(seeded.context, [Fr::from(5u64), Fr::from(6u64)], z)
        .expect("aggregate helper formal must resolve to the callee's four-leaf vector");

    // No caller binding of the helper's formal name: the helper's own
    // `Vector<2, Field>` must still be recovered.
    let v = [Fr::from(7u64), Fr::from(8u64)];
    let v_hash = midnight_compact_runtime::std_lib::persistent_hash_aligned(&[
        AlignedValue::from(v[0]),
        AlignedValue::from(v[1]),
    ]);
    let seeded = contract
        .set_hash(ctx.clone(), v_hash)
        .expect("set_hash (no-collision)");
    contract
        .check_no_collision_scope(seeded.context, v)
        .expect("unbound helper formal must still be recovered from the callee scope");
}
