// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Executing gate for the embedded-curve builtins (`examples/ec_ops_fixture.compact`).
//!
//! The runtime's curve operations are fallible: a `JubjubPoint` is a bare
//! coordinate pair, and `ec_add` / `ec_mul` / `ec_neg` validate it as a
//! prime-order group element on use. The emitter appends `?` at those call
//! sites and nowhere else (`ec_mul_generator`, `construct_jubjub_point` are
//! total). These tests pin both halves: the values agree with the runtime
//! called directly, and an operation on a pair outside the subgroup is an
//! `Err`, not a panic.

use compact_contract_ec_ops_fixture::pure_circuits;
use midnight_compact_runtime::*;

fn point(k: u64) -> JubjubPoint {
    ec_mul_generator(JubjubScalar::from(k))
}

#[test]
fn add_mul_neg_agree_with_the_runtime() {
    let p = point(3);
    let q = point(7);
    assert_eq!(
        pure_circuits::add(p, q).expect("add"),
        ec_add(p, q).expect("runtime add")
    );
    assert_eq!(pure_circuits::add(p, q).expect("add"), point(10));
    assert_eq!(
        pure_circuits::mul(p, Fr::from(5u64)).expect("mul"),
        point(15)
    );
    let n = pure_circuits::neg(p).expect("neg");
    assert_eq!(ec_add(p, n).expect("p + (-p)"), point(0));
    assert_eq!(
        pure_circuits::double_then_neg(p).expect("double then neg"),
        pure_circuits::neg(point(6)).expect("neg 6G")
    );
}

#[test]
fn generator_construct_and_projections_are_total() {
    let g5 = pure_circuits::gen(Fr::from(5u64)).expect("gen");
    assert_eq!(g5, point(5));
    let (x, y) = pure_circuits::coords(g5).expect("coords");
    let rebuilt = pure_circuits::construct(x, y).expect("construct");
    assert_eq!(rebuilt, g5);
    // `construct` accepts any pair: it is the bare coordinate pair, not a
    // validated group element.
    let off_curve =
        pure_circuits::construct(Fr::from(1u64), Fr::from(2u64)).expect("construct any pair");
    assert_eq!(
        pure_circuits::coords(off_curve).expect("coords"),
        (Fr::from(1u64), Fr::from(2u64))
    );
}

#[test]
fn a_pair_outside_the_subgroup_is_an_error_not_a_panic() {
    // (0, 0) satisfies the twisted-Edwards equation but is not in the
    // prime-order subgroup; the old validated alias panicked on it.
    let zero = pure_circuits::construct(Fr::from(0u64), Fr::from(0u64)).expect("construct");
    assert!(
        pure_circuits::neg(zero).is_err(),
        "ecNeg on a non-subgroup pair must fail"
    );
    assert!(
        pure_circuits::add(zero, point(1)).is_err(),
        "ecAdd on a non-subgroup pair must fail"
    );
    assert!(
        pure_circuits::mul(zero, Fr::from(2u64)).is_err(),
        "ecMul on a non-subgroup pair must fail"
    );
    // An off-curve pair fails the same way.
    let off = pure_circuits::construct(Fr::from(1u64), Fr::from(2u64)).expect("construct");
    assert!(pure_circuits::add(off, off).is_err());
}
