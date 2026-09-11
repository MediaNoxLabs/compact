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
// uint_field_coercion_fixture executing gate.
//
// Byte-parity (codegen_regression) locks the emitted coercion TEXT; this
// gate proves the text yields the right VALUES. Two failure modes are
// invisible to a byte-compare alone:
//
//   * a `Uint<128>` scalar above u64::MAX must embed losslessly. The
//     coercion picks `as u128`; a fall-back to `as u64` would compile and
//     silently truncate the high bits, so the round-trip uses a value with
//     bits above 64 set.
//   * a `Vector<2, Field>` written from `Uint<32>` elements must read back
//     as the same field elements. The wrong (pre-fix) emission still
//     compiles via `new_cell_array`'s generic `Into<AlignedValue>` bound,
//     so the only Rust-side signal is the value: `u32::MAX` and a second
//     high-pattern element would not survive a 4-byte-aligned or
//     truncating write.
//
// The private-state type is `()`; passing that unit into the constructor
// context is the correct semantic use even though clippy's `unit_arg` lint
// flags it.
#![allow(clippy::unit_arg)]

use compact_contract_uint_field_coercion_fixture::{ledger, pure_circuits, Contract};
use midnight_compact_runtime::*;

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// A `Uint<128>` above u64::MAX must survive the scalar Field coercion:
/// `Fr::from((x) as u128)` embeds it without reduction, and the read-back
/// equals the field element built from the same integer.
#[test]
fn wide_uint_scalar_write_round_trips() {
    let wide: u128 = (1u128 << 100) + 12345;
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract
        .initial_state(ctor_ctx(), wide, 0, 0)
        .expect("initial_state");
    let view = ledger(&result.current_contract_state);
    assert_eq!(view.wide_field().expect("wide_field"), Fr::from(wide));
}

/// A `Vector<2, Field>` ledger written from `Uint<32>` elements must read
/// back as the same field elements at values that would not survive a
/// truncating or wrongly-aligned write.
#[test]
fn vector_field_write_round_trips() {
    let a: u32 = u32::MAX;
    let b: u32 = 0xDEAD_BEEF;
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract
        .initial_state(ctor_ctx(), 0, a, b)
        .expect("initial_state");
    let view = ledger(&result.current_contract_state);
    assert_eq!(
        view.vec_field().expect("vec_field"),
        [Fr::from(a as u64), Fr::from(b as u64)]
    );
    // The ctor-route call-binding branch binds the pure-circuit result raw and
    // shadows it with the coercion (`let w = pick_uint(a)?; let w =
    // Fr::from((w) as u64);`); the read-back must be the Field value.
    assert_eq!(view.via_call().expect("via_call"), Fr::from(a as u64));
}

/// The pure route must coerce every shape: the tuple literal, the
/// let*-lifted `const` whose tuple is buried in a `seq`, and the nested
/// aggregate. The u128 value keeps the `as u128` rung honest.
#[test]
fn pure_route_coercion_shapes() {
    let x: u128 = (1u128 << 100) + 7;
    assert_eq!(
        pure_circuits::return_wide(x).expect("return_wide"),
        Fr::from(x)
    );
    assert_eq!(
        pure_circuits::return_vector(7).expect("return_vector"),
        [Fr::from(7u64), Fr::from(7u64)]
    );
    assert_eq!(
        pure_circuits::lifted_vector(9).expect("lifted_vector"),
        [Fr::from(9u64), Fr::from(9u64)]
    );
    assert_eq!(
        pure_circuits::nested_vector(x).expect("nested_vector"),
        [[Fr::from(x)], [Fr::from(x)]]
    );
}
