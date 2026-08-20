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
// G1 executing assert gate.
//
// guarded_assert_arith_fixture.compact puts a struct-field projection in an
// operand of trapping unsigned arithmetic:
//
//   if (policy.enforceMaxAge) {
//     assert(currentTime - attestation.proof.createdAt <= policy.maxAge, "...");
//   }
//
// Compact's typer wraps the `-` in an underflow guard and let*-lifts the
// projection into its own temp, nesting TWO levels of assignment. Before G1
// the emitter rendered only the outer level and the whole pure-circuit body
// bailed with `no walker shape matched pure circuit body`.
//
// Byte-parity (codegen_regression) locks the generated text but cannot tell a
// correct lowering from a plausible-looking wrong one — a swapped subtraction
// operand, a temp bound from the wrong projection, or a guard reading a
// shadowed `let t` would all pass forever. This test is the semantic gate:
// EXECUTE the generated pure circuits with values that satisfy each assert
// and with values that trip it, so the arithmetic and the operand binding are
// pinned by behaviour.
//
// The private-state type is `()`; passing that unit into `CircuitContext::new`
// is the correct semantic use even though clippy's `unit_arg` lint flags it.
#![allow(clippy::unit_arg)]

use compact_contract_guarded_assert_arith_fixture::{
    pure_circuits, Attestation, Contract, StatusProof, VerifierPolicy,
};
use midnight_compact_runtime::*;

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

fn attestation(created_at: u64) -> Attestation {
    Attestation {
        proof: StatusProof {
            createdAt: created_at,
            issuer: 9,
        },
        hasExpiration: false,
        expiresAt: 0,
    }
}

fn policy(enforce: bool, max_age: u64) -> VerifierPolicy {
    VerifierPolicy {
        enforceMaxAge: enforce,
        maxAge: max_age,
    }
}

/// The guarded assert passes when the age is within the policy. `1_000 - 900 =
/// 100 <= 100`, so this also pins the boundary as inclusive — an off-by-one or
/// a swapped `<=` operand fails here.
#[test]
fn guarded_max_age_assert_passes_at_the_boundary() {
    pure_circuits::assert_fresh_enough(policy(true, 100), attestation(900), 1_000)
        .expect("age of exactly maxAge must satisfy the policy");
}

/// One unit past the policy trips the guarded assert. This is the assertion
/// that proves the subtraction operand really is `attestation.proof.createdAt`
/// and not some other temp: with `currentTime = 1_000` and `createdAt = 899`
/// the age is 101, which exceeds `maxAge = 100`.
#[test]
fn guarded_max_age_assert_trips_when_too_old() {
    let err = pure_circuits::assert_fresh_enough(policy(true, 100), attestation(899), 1_000)
        .expect_err("age of maxAge + 1 must trip the guarded assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "attestation exceeds the max-age policy"
        ),
        "expected the max-age policy message, got {err:?}"
    );
}

/// With the guard false the arithmetic is never reached, so an age far beyond
/// the policy is accepted. Proves the `if` still gates the assert after the
/// nested-assignment rendering change (the whole block, temps included, must
/// sit inside the branch).
#[test]
fn max_age_assert_is_skipped_when_the_guard_is_false() {
    pure_circuits::assert_fresh_enough(policy(false, 1), attestation(0), 1_000_000)
        .expect("a false guard must skip the max-age assert entirely");
}

/// The unguarded assert in the same contract behaves identically — the fix is
/// in the shared expression renderer, not in the branch emitter.
#[test]
fn unguarded_max_age_assert_passes_and_trips() {
    pure_circuits::assert_age_within(attestation(900), 1_000, 100)
        .expect("age within the limit must pass");

    let err = pure_circuits::assert_age_within(attestation(899), 1_000, 100)
        .expect_err("age beyond the limit must trip the assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "attestation age exceeds the limit"
        ),
        "expected the age-limit message, got {err:?}"
    );
}

/// The typer's own underflow guard fires before the subtraction wraps. Without
/// it, `900 - 1_000` on `u64` would wrap to a huge value and silently satisfy
/// `<= limit`, so this pins that the guard assert is emitted INSIDE the lifted
/// temp's block rather than dropped along with the nesting.
#[test]
fn subtraction_underflow_is_trapped_not_wrapped() {
    let err = pure_circuits::assert_age_within(attestation(1_000), 900, u64::MAX)
        .expect_err("currentTime before createdAt must trap, not wrap");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "result of subtraction would be negative"
        ),
        "expected the underflow guard to fire, got {err:?}"
    );
}

/// Both operands of `-` are projections here, so two lifted temps coexist in
/// one body. Checking the returned value (not just Ok/Err) proves the
/// uniquifier kept them apart — if both temps rendered to the same Rust name
/// the inner `let` would shadow the outer and the gap would compute 0.
#[test]
fn two_projection_operands_compute_the_real_difference() {
    let gap = pure_circuits::age_gap(attestation(1_000), attestation(900))
        .expect("newer >= older must pass the ordering assert");
    assert_eq!(
        gap, 100,
        "age gap must be newer.createdAt - older.createdAt"
    );

    let err = pure_circuits::age_gap(attestation(900), attestation(1_000))
        .expect_err("newer < older must trip the ordering assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m)
                if m == "newer attestation must not predate the older one"
        ),
        "expected the ordering message, got {err:?}"
    );
}

/// The impure caller propagates a failing guarded assert out through the `?`
/// the codegen appends at the pure-circuit call site, and commits the ledger
/// write only on the success path.
#[test]
fn impure_caller_propagates_the_guarded_assert_failure() {
    let contract: Contract<()> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(
        init.current_contract_state.clone(),
        init.current_private_state,
    );

    // `CircuitResults<(), ()>` is not `Debug`, which `expect_err`'s `T: Debug`
    // bound requires, so go through `.err().expect(...)`.
    #[allow(clippy::err_expect)]
    let err = contract
        .record_fresh_enough(ctx, policy(true, 100), attestation(899), 1_000)
        .err()
        .expect("a failing guarded assert must propagate out of the impure circuit");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "attestation exceeds the max-age policy"
        ),
        "expected the propagated max-age message, got {err:?}"
    );

    // Success path: same circuit, an age the policy accepts.
    let ok_ctx = CircuitContext::new(init.current_contract_state, ());
    contract
        .record_fresh_enough(ok_ctx, policy(true, 100), attestation(900), 1_000)
        .expect("an age within the policy must commit the ledger write");
}
