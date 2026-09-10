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
// Ternary executing gate.
//
// ternary_cond_fixture.compact puts conditional (ternary) expressions in
// every sub-expression position and body route:
//
//   return position (`pick`), const RHS in a pure circuit
//   (`const_pick`), assert argument (`assert_day`), arithmetic operand
//   (`decrement`), `&&`/`||` branch values (`and_or_branches`), nested
//   + struct-valued branches (`choose_struct`), enum-valued branches
//   (`choose_label`), an impure ledger-writing circuit
//   (`record_pick`), and the constructor (`initial_state`'s `initial`).
//
// Byte-parity (codegen_regression) locks the generated TEXT but cannot
// tell a lazy `if c { e1 } else { e2 }` from an eager
// evaluate-both-then-select: both are valid Rust with identical bytes
// in the places parity sees, but an eager lowering evaluates the
// UNSELECTED branch, and the language spec requires that it never be
// evaluated. This test is the semantic gate for that: `pick(9)` must
// return 9 even though its untaken then-branch would trip the underflow
// guard (`9 - 10` cannot exist as a Uint<64>) — an eager lowering fails
// this test, a lazy one cannot.
//
// The private-state type is `()`; passing that unit into
// `CircuitContext::new` is the correct semantic use even though
// clippy's `unit_arg` lint flags it.
#![allow(clippy::unit_arg)]

use compact_contract_ternary_cond_fixture::{ledger, pure_circuits, Choice, Contract, Label};
use midnight_compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use tests_e2e_rust::SmallFixtureTsReference;

/// The ternary fixture's TS reference additionally carries an
/// `afterRecordLiteralPick` snapshot (see capture-ternary-cond-fixture.mjs):
/// the state after executing `recordLiteralPick(true)` from the post-init
/// state on the TS driver. `SmallFixtureTsReference` models the shared
/// afterInit-only shape, so parse the extended document locally.
#[derive(serde::Deserialize)]
struct TernaryTsReference {
    #[serde(rename = "afterRecordLiteralPick")]
    after_record_literal_pick: tests_e2e_rust::SmallFixtureStepSnapshot,
}

impl TernaryTsReference {
    fn load() -> Self {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/ternary-cond-fixture-ts-state.json"
        ))
        .expect("read ternary fixture json");
        serde_json::from_str(&raw).expect("parse ternary fixture json")
    }
}

/// Constructor argument used on BOTH sides of the byte-parity capture:
/// 20 > 15, so the constructor's ternary takes its then branch and the
/// guarded `start - 10` computes 10. The branch TEXT is byte-locked by
/// codegen_regression; the taken-branch CHOICE is asserted in
/// `constructor_ternary_writes_the_taken_branch` below.
const CAPTURE_START: u64 = 20;

fn fixture() -> SmallFixtureTsReference {
    SmallFixtureTsReference::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/ternary-cond-fixture-ts-state.json"
    ))
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// Build a `ContractState` envelope around a freshly minted `ChargedState`.
/// `ternary_cond_fixture` exports one IMPURE circuit (`recordPick`), so the
/// operations map must register one entry under that name to match the
/// TS-side `initialState()` output (the seven pure circuits live in
/// `pure_circuits` and are not part of the dispatch map).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    // One entry per exported impure circuit — the TS reference's
    // initialState() pre-registers them all, so byte-parity requires the
    // same four here.
    for name in [
        "recordPick",
        "recordLiteralPick",
        "recordFieldPick",
        "recordFieldBranches",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    ContractState {
        data,
        operations,
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    }
}

/// The constructor route's byte-parity gate: the TS backend's
/// `initialState(ctx, 20n)` and the Rust `initial_state(ctx, 20)` must
/// produce identical `ContractState` bytes. The constructor body
/// contains a ternary with a guarded `-`, so this pins that route's
/// lowering against the TS reference, not just against yesterday's
/// Rust.
#[test]
fn ternary_cond_fixture_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let envelope = make_envelope(result.current_contract_state.clone());
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_init.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// The constructor's ternary took its then branch at `CAPTURE_START`:
/// `lastPick` must hold the guarded `start - 10` result, and `picks`
/// (never written by the constructor) its zero seed. Pins the taken
/// branch's VALUE, which byte-parity alone cannot judge.
#[test]
fn constructor_ternary_writes_the_taken_branch() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");
    let view = ledger(&result.current_contract_state);
    assert_eq!(view.last_pick().expect("last_pick"), 10);
    assert_eq!(view.picks().expect("picks"), 0);
}

/// THE laziness oracle: `c = 9` selects the else branch, so the
/// then-branch `c - 10` — an underflowing subtraction guarded by
/// `compact_assert!(c >= 10)` INSIDE the branch — must never be
/// evaluated. An eager both-branches lowering computes the guard
/// anyway, trips `9 >= 10`, and returns `Err`; the lazy Rust `if` the
/// backend emits returns `Ok(9)`.
#[test]
fn pick_untaken_branch_does_not_trap() {
    assert_eq!(
        pure_circuits::pick(9).expect("c = 9 must take the else branch"),
        9
    );
}

/// The taken direction of the same oracle: `c = 20` selects the then
/// branch, the guard passes, and the subtraction really computes — so
/// laziness is not laziness-by-dropping-the-branch.
#[test]
fn pick_taken_branch_computes() {
    assert_eq!(
        pure_circuits::pick(20).expect("c = 20 must take the then branch"),
        10
    );
}

/// The const-RHS route (the digital-passport failure shape) in both
/// directions, plus the inverse trap: `c = 0` TAKES the guarded then
/// branch (`0 <= 2`), so the underflow guard must fire there — proving
/// the guard lives inside the branch it protects, neither hoisted out
/// (which would break `pick(9)` above) nor dropped (which would wrap).
#[test]
fn const_pick_takes_each_branch_and_traps_when_taken() {
    assert_eq!(
        pure_circuits::const_pick(5).expect("c = 5 must take the else branch"),
        5
    );
    assert_eq!(
        pure_circuits::const_pick(2).expect("c = 2 must take the then branch"),
        1
    );
    let err =
        pure_circuits::const_pick(0).expect_err("c = 0 takes the guarded branch and must trap");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "result of subtraction would be negative"
        ),
        "expected the underflow guard to fire, got {err:?}"
    );
}

/// The UNANNOTATED const + both-literal-arms + return route — the
/// dogfood-review regression shapes. The lowering inlines the literal-if
/// at the tail (`Ok(if hot { 10 } else { 20 })`, byte-identical to the
/// no-const `pick` form) so the return context sizes the arms; a
/// let-bound i32 default fails E0308 here, and the big-literal twin
/// (`5000000000 > i32::MAX`) cannot even default. Pins both arm values
/// of both circuits.
#[test]
fn literal_pick_const_routes_size_arms_from_the_return_context() {
    assert_eq!(pure_circuits::literal_pick(true).expect("hot arm"), 10u64);
    assert_eq!(pure_circuits::literal_pick(false).expect("cold arm"), 20u64);
    assert_eq!(
        pure_circuits::big_literal_pick(true).expect("big hot arm"),
        5_000_000_000u64
    );
    assert_eq!(
        pure_circuits::big_literal_pick(false).expect("big cold arm"),
        0u64
    );
}

/// Assert-argument route: the selected comparison is the one the assert
/// enforces. `29` is legal in a leap year and illegal otherwise — if
/// the wrong branch's bound applied, one of these two would fail.
#[test]
fn assert_day_applies_the_selected_bound() {
    pure_circuits::assert_day(29, true).expect("Feb 29 is legal in a leap year");
    pure_circuits::assert_day(28, false).expect("Feb 28 is always legal");

    let leap_err =
        pure_circuits::assert_day(30, true).expect_err("Feb 30 is illegal even in a leap year");
    assert!(
        matches!(
            leap_err,
            CompactError::AssertionFailed(ref m) if m == "day out of range for the month"
        ),
        "expected the leap bound to fire, got {leap_err:?}"
    );

    let plain_err =
        pure_circuits::assert_day(29, false).expect_err("Feb 29 is illegal in a non-leap year");
    assert!(
        matches!(
            plain_err,
            CompactError::AssertionFailed(ref m) if m == "day out of range for the month"
        ),
        "expected the non-leap bound to fire, got {plain_err:?}"
    );
}

/// Interior-operand route: `n - (flag ? 1 : 0)`. The operand is lifted
/// to a temp whose underflow guard reads the temp, so a mis-bound temp
/// (e.g. always 0) would silently pass; checking both flag values and
/// the n = 0 trap pins the operand's routing.
#[test]
fn decrement_subtracts_the_selected_operand() {
    assert_eq!(
        pure_circuits::decrement(10, true).expect("flag = true subtracts 1"),
        9
    );
    assert_eq!(
        pure_circuits::decrement(10, false).expect("flag = false subtracts 0"),
        10
    );
    let err = pure_circuits::decrement(0, true)
        .expect_err("0 - 1 must trip the underflow guard, not wrap");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "result of subtraction would be negative"
        ),
        "expected the underflow guard to fire, got {err:?}"
    );
}

/// `&&` / `||` as branch values. Both branches reduce to `b` for the
/// matching `a`, so the four combinations pin that each branch renders
/// its OWN operator shape rather than a shared or swapped one.
#[test]
fn and_or_branches_each_compute() {
    assert!(pure_circuits::and_or_branches(true, true).expect("then: true && true"));
    assert!(!pure_circuits::and_or_branches(true, false).expect("then: true && false"));
    assert!(pure_circuits::and_or_branches(false, true).expect("else: false || true"));
    assert!(!pure_circuits::and_or_branches(false, false).expect("else: false || false"));
}

/// Nested ternary with struct-valued branches: the outer else and BOTH
/// inner arms produce `Choice` values, so the arms must unify to one
/// Rust type (the spec's typed-branch-values requirement). The three
/// inputs select a different arm each: c = 9 the outer else, c = 20 the
/// inner else (`c - 5`), c = 40 the inner then (`c - 10`).
#[test]
fn choose_struct_unifies_nested_branch_arms() {
    assert_eq!(
        pure_circuits::choose_struct(9).expect("c = 9 takes the outer else"),
        Choice {
            low: true,
            value: 9
        }
    );
    assert_eq!(
        pure_circuits::choose_struct(20).expect("c = 20 takes the inner else"),
        Choice {
            low: false,
            value: 15
        }
    );
    assert_eq!(
        pure_circuits::choose_struct(40).expect("c = 40 takes the inner then"),
        Choice {
            low: false,
            value: 30
        }
    );
}

/// Enum-valued branches: each arm renders its variant constructor.
#[test]
fn choose_label_selects_the_variant() {
    assert_eq!(
        pure_circuits::choose_label(20).expect("c = 20 is hot"),
        Label::hot
    );
    assert_eq!(
        pure_circuits::choose_label(9).expect("c = 9 is cold"),
        Label::cold
    );
}

/// The impure (ledger-writing) route. `recordPick` asserts, evaluates a
/// const ternary, writes `lastPick`, and increments `picks` — the Err
/// side proves the assert fires before any write, and both success
/// sides prove the selected branch's value is what hits the ledger.
#[test]
fn record_pick_commits_the_selected_branch_and_propagates_failures() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    // `CircuitResults<(), ()>` is not `Debug`, which `expect_err`'s
    // `T: Debug` bound requires, so go through `.err().expect(...)`.
    #[allow(clippy::err_expect)]
    let err = contract
        .record_pick(
            CircuitContext::new(
                init.current_contract_state.clone(),
                init.current_private_state,
            ),
            0,
            true,
        )
        .err()
        .expect("c = 0 must trip the non-zero assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "pick input must be non-zero"
        ),
        "expected the non-zero assert, got {err:?}"
    );

    // Then-branch value (hot): `picked = 10` hits the ledger.
    let after_hot = contract
        .record_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            9,
            true,
        )
        .expect("c = 9, hot = true must commit");
    let view = ledger(&after_hot.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 10);
    assert_eq!(view.picks().expect("picks"), 1);

    // Else-branch value (not hot): `picked = c` hits the ledger.
    let after_cold = contract
        .record_pick(
            CircuitContext::new(
                after_hot.context.current_query_context.state,
                after_hot.context.current_private_state,
            ),
            7,
            false,
        )
        .expect("c = 7, hot = false must commit");
    let view = ledger(&after_cold.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 7);
    assert_eq!(view.picks().expect("picks"), 2);
}

/// Both-literal arms (the unsuffixed-i32 regression): `hot ? 10 : 20`
/// written to a Uint ledger. With both arms bare the generated
/// `let picked = if hot { 10 } else { 20 };` infers i32 and the write's
/// `Into<AlignedValue>` bound fails E0277 — the arms must carry the
/// declared width. Driving both flag values pins each arm's committed
/// VALUE, so a swapped or mis-typed arm cannot pass by compiling.
#[test]
fn record_literal_pick_commits_each_suffixed_arm() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let after_hot = contract
        .record_literal_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("hot = true must commit");
    let view = ledger(&after_hot.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 10);
    assert_eq!(view.picks().expect("picks"), 1);

    let after_cold = contract
        .record_literal_pick(
            CircuitContext::new(after_hot.context.current_query_context.state, ()),
            false,
        )
        .expect("hot = false must commit");
    let view = ledger(&after_cold.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 20);
    assert_eq!(view.picks().expect("picks"), 2);
}

/// The literal-arm WRITE path's byte-parity gate — the dogfood-review
/// regression. `recordLiteralPick`'s const ternary arms are both small
/// integer literals, so the binding's own inferred width is u8; without
/// the destination-field coercion the Rust backend committed a
/// 1-byte-aligned cell into `lastPick: Uint<64>` while the TS field
/// descriptor (`CompactTypeUnsignedInteger(2^64-1, 8)`) commits 8 bytes —
/// identical decoded values, divergent state bytes. The decoded-value
/// test below cannot see that; this executes the same circuit from the
/// same post-init state on both drivers and byte-compares the serialized
/// envelopes, so the alignment divergence fails here.
#[test]
fn record_literal_pick_byte_parity() {
    let ts_ref = TernaryTsReference::load();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let after = contract
        .record_literal_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("record_literal_pick(true) must commit");

    let envelope = make_envelope(after.context.current_query_context.state);
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_record_literal_pick.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust post-recordLiteralPick state bytes differ from TS reference\
\
Rust ({} B): {}\
\
TS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// The Field-typed twin of the literal-arm regression: a Field-annotated
/// const whose arms are both literals must render them as
/// `Fr::from(1u64)` / `Fr::from(0u64)` — a bare literal fails the
/// `Into<AlignedValue>` bound on a Field ledger.
#[test]
fn record_field_pick_commits_fr_literal_arms() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let after_true = contract
        .record_field_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("c = true must commit");
    let view = ledger(&after_true.context.current_query_context.state);
    assert_eq!(view.origin().expect("origin"), Fr::from(1u64));

    let after_false = contract
        .record_field_pick(
            CircuitContext::new(after_true.context.current_query_context.state, ()),
            false,
        )
        .expect("c = false must commit");
    let view = ledger(&after_false.context.current_query_context.state);
    assert_eq!(view.origin().expect("origin"), Fr::from(0u64));
}

/// Field-comparison ternary branches in the PURE route: each arm's
/// `!= 0` literal must render as `Fr::from(0u64)` — a bare `0` fails
/// E0308 against the Fr jubjubPointX/Y return. The generator's
/// coordinates are non-zero (both arms true); the twisted-Edwards
/// neutral element is (0, 1), so `JubjubPoint::default()` selects the
/// x-arm false side while its y stays non-zero.
#[test]
fn field_branches_compare_the_selected_coordinate() {
    let generator = JubjubPoint::generator();
    let identity = JubjubPoint::default();

    assert!(
        pure_circuits::field_branches(true, generator).expect("generator x != 0"),
        "generator's x coordinate must be non-zero"
    );
    assert!(
        pure_circuits::field_branches(false, generator).expect("generator y != 0"),
        "generator's y coordinate must be non-zero"
    );
    assert!(
        !pure_circuits::field_branches(true, identity).expect("identity x == 0"),
        "identity's x coordinate must be zero"
    );
    assert!(
        pure_circuits::field_branches(false, identity).expect("identity y == 1"),
        "identity's y coordinate is 1 (neutral element), non-zero"
    );
}

/// The impure twin of the field-branch comparison: the assert must fire
/// on the identity point (Err before any write) and pass on the
/// generator (Ok, incrementing `picks`).
#[test]
fn record_field_branches_asserts_the_selected_coordinate() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    #[allow(clippy::err_expect)]
    let err = contract
        .record_field_branches(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
            JubjubPoint::default(),
        )
        .err()
        .expect("identity point must trip the non-origin assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "point must be non-origin"
        ),
        "expected the non-origin assert, got {err:?}"
    );

    let after = contract
        .record_field_branches(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            false,
            JubjubPoint::generator(),
        )
        .expect("generator point must commit");
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.picks().expect("picks"), 1);
}
