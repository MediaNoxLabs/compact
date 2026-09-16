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
// Assert-parity proof.
//
// This is the dedicated test that proves the Rust codegen achieves
// parity with the TypeScript backend for failing `assert(cond, "msg")`
// inside a pure circuit: a failing assert yields a handleable
// `Err(CompactError::AssertionFailed)`, NOT a Rust panic.
//
// Before the Phase 1-6 codegen change, compactc --rust lowered Compact
// `assert` to a panicking `assert!`, so a failing assert aborted the
// host process — fundamentally different from the TS backend, which
// throws a catchable `CompactError`. Now pure circuits return
// `Result<T, CompactError>` and `assert` lowers to `compact_assert!`
// (which returns `Err(CompactError::AssertionFailed)`), so the failure
// is a normal, handleable `Result`.
//
// Three assertions, in order of increasing integration depth:
//   1. Direct pure-circuit call `require_true(false)` returns
//      `Err(AssertionFailed)` (not panic). The success case
//      `require_true(true)` returns `Ok(true)`.
//   2. Impure circuit `trigger_fail()` — which calls the pure circuit
//      and propagates its error via the `?` appended at the call site
//      — also returns `Err(AssertionFailed)` (not panic).
//   3. The matching impure circuit `trigger_ok()` succeeds and returns
//      `Ok`, confirming the `?`-propagation only fires on actual
//      assertion failure, not unconditionally.
//
// The whole point: none of these `#[should_panic]`. They assert on the
// `Err` variant of a returned `Result`.

// The test contract is `Contract<(), NoWitnesses>` — the private-state
// type is `()`. Passing that unit into `CircuitContext::new(state,
// private_state)` is the correct semantic use even though clippy's
// `unit_arg` lint views unit args as suspicious.
#![allow(clippy::unit_arg)]

use compact_contract_assert_parity::{pure_circuits, Contract};
use midnight_compact_runtime::{CompactError, ConstructorContext, NoWitnesses};

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: Default::default(),
        cost_model: midnight_compact_runtime::INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// A failing pure-circuit assert returns `Err(AssertionFailed)` rather
/// than panicking. This is the core parity guarantee.
#[test]
fn failing_pure_circuit_assert_is_err_not_panic() {
    let err = pure_circuits::require_true(false)
        .expect_err("require_true(false) must return Err(AssertionFailed), not Ok or panic");
    assert!(
        matches!(err, CompactError::AssertionFailed(ref msg) if msg == "must be true"),
        "expected AssertionFailed(\"must be true\"), got {err:?}"
    );
}

/// A passing pure-circuit assert returns `Ok`, proving `compact_assert!`
/// only short-circuits on a false condition.
#[test]
fn passing_pure_circuit_assert_is_ok() {
    let ok = pure_circuits::require_true(true)
        .expect("require_true(true) must return Ok(true), not Err");
    assert!(ok, "require_true(true) result must be true");
}

/// An impure circuit that calls a pure circuit whose assert fails
/// propagates the `Err(AssertionFailed)` out (via the `?` the codegen
/// appends at the pure-circuit call site) instead of panicking. This
/// exercises the full host-side call chain.
#[test]
fn failing_impure_caller_of_pure_circuit_assert_is_err_not_panic() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = midnight_compact_runtime::CircuitContext::new(
        init.current_contract_state,
        init.current_private_state,
    );
    // Cannot use `.expect_err(...)` here — `CircuitResults<(), ()>`
    // (the Ok variant returned by an impure circuit on the test
    // contract) does not implement `Debug`, which `expect_err`'s
    // `T: Debug` bound requires. Fall back to `.err().expect(...)`
    // and suppress clippy's suggestion locally.
    #[allow(clippy::err_expect)]
    let err = contract
        .trigger_fail(ctx)
        .err()
        .expect("trigger_fail() must return Err(AssertionFailed), not Ok or panic");
    assert!(
        matches!(err, CompactError::AssertionFailed(ref msg) if msg == "must be true"),
        "expected propagated AssertionFailed(\"must be true\"), got {err:?}"
    );
}

/// The matching impure caller that satisfies the assert succeeds,
/// confirming the `?`-propagation is conditional on the assert failing
/// and that the on-chain op program is still built and run for the
/// success path.
#[test]
fn succeeding_impure_caller_of_pure_circuit_assert_is_ok() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = midnight_compact_runtime::CircuitContext::new(
        init.current_contract_state,
        init.current_private_state,
    );
    contract
        .trigger_ok(ctx)
        .expect("trigger_ok() must return Ok, not Err or panic");
}
