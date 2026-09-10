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
//   (`record_pick`), the constructor (`initial_state`'s `initial`),
//   and the round-4 Field-join positions: nested call arguments
//   (`field_call_arg_pick` / `field_call_arg_both_lit`),
//   struct-member initialisers (`field_struct_pick`), the unannotated
//   both-literal const to a Field return (`const_field_pick` /
//   `picked_field_arith`), and the impure nested-call-argument twin
//   (`record_field_call_arg`).
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

/// The ternary fixture's TS reference additionally carries
/// `afterRecordLiteralPick`, `afterStreamLiteralPick`,
/// `afterStreamNarrowWrite`, and `afterRecordStructPick` snapshots (see
/// capture-ternary-cond-fixture.mjs): the state after executing the
/// named circuit from the prior captured state on the TS driver.
/// `SmallFixtureTsReference` models the shared afterInit-only shape,
/// so parse the extended document locally.
#[derive(serde::Deserialize)]
struct TernaryTsReference {
    #[serde(rename = "afterRecordLiteralPick")]
    after_record_literal_pick: tests_e2e_rust::SmallFixtureStepSnapshot,
    #[serde(rename = "afterStreamLiteralPick")]
    after_stream_literal_pick: tests_e2e_rust::SmallFixtureStepSnapshot,
    #[serde(rename = "afterStreamNarrowWrite")]
    after_stream_narrow_write: tests_e2e_rust::SmallFixtureStepSnapshot,
    #[serde(rename = "afterRecordStructPick")]
    after_record_struct_pick: tests_e2e_rust::SmallFixtureStepSnapshot,
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
/// `ternary_cond_fixture` exports several IMPURE circuits, so the
/// operations map must register one entry under each exported name to
/// match the TS-side `initialState()` output (the pure circuits live in
/// `pure_circuits` and are not part of the dispatch map).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    // One entry per exported impure circuit — the TS reference's
    // initialState() pre-registers them all, so byte-parity requires the
    // same eleven here.
    for name in [
        "recordPick",
        "recordLiteralPick",
        "recordFieldPick",
        "recordFieldBranches",
        "recordMixedFieldPick",
        "assertFieldEqOperand",
        "streamLiteralPick",
        "streamNarrowWrite",
        "inlineBigPick",
        "recordStructPick",
        "recordFieldCallArg",
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

// ---- dogfood-review round 2: mixed-arm Field joins, operand/return
// positions, streaming-route literals, and the streaming write width. --

/// PURE mixed-arm Field const: `flag ? x : 0` with x: Field must render
/// the literal arm as `Fr::from(0u64)` — an integer literal never
/// unifies with the other arm's `Fr` (E0308 at cargo build, compactc
/// exit 0 before the fix). Both directions pin the selected VALUE.
#[test]
fn mixed_field_join_returns_each_arm() {
    let x = Fr::from(42u64);
    assert_eq!(
        pure_circuits::mixed_field_join(true, x).expect("then arm"),
        x
    );
    assert_eq!(
        pure_circuits::mixed_field_join(false, x).expect("else arm"),
        Fr::from(0u64)
    );
}

/// RETURN position: the frontend lifts `return flag ? x : 0;` to an if
/// STATEMENT, so the literal becomes its own branch tail with no
/// binding type to size from — the circuit's Field return type must
/// drive the coercion there.
#[test]
fn return_field_returns_each_arm() {
    let x = Fr::from(7u64);
    assert_eq!(pure_circuits::return_field(true, x).expect("then arm"), x);
    assert_eq!(
        pure_circuits::return_field(false, x).expect("else arm"),
        Fr::from(0u64)
    );
}

/// The ternary as an OPERAND of a Field `==`: the whole if renders
/// inside the comparison, literal arms coerced.
#[test]
fn field_eq_operand_selects_the_literal_side() {
    let one = Fr::from(1u64);
    assert!(pure_circuits::field_eq_operand(one, true).expect("c: 1 == 1"));
    assert!(!pure_circuits::field_eq_operand(one, false).expect("c: 1 == 0"));
}

/// The ternary as an operand of FIELD arithmetic (`+` renders via the
/// Fr operators, not `wrapping_add`).
#[test]
fn field_add_operand_adds_the_selected_literal() {
    let ten = Fr::from(10u64);
    assert_eq!(
        pure_circuits::field_add_operand(ten, true).expect("10 + 1"),
        Fr::from(11u64)
    );
    assert_eq!(
        pure_circuits::field_add_operand(ten, false).expect("10 + 0"),
        ten
    );
}

/// The impure (walker-route) twin of `mixed_field_join`, committing the
/// mixed join to the Field ledger. Both directions execute and decode.
#[test]
fn record_mixed_field_pick_commits_each_arm() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let x = Fr::from(33u64);
    let after_true = contract
        .record_mixed_field_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            x,
            true,
        )
        .expect("flag = true must commit");
    let view = ledger(&after_true.context.current_query_context.state);
    assert_eq!(view.origin().expect("origin"), x);

    let after_false = contract
        .record_mixed_field_pick(
            CircuitContext::new(after_true.context.current_query_context.state, ()),
            x,
            false,
        )
        .expect("flag = false must commit");
    let view = ledger(&after_false.context.current_query_context.state);
    assert_eq!(view.origin().expect("origin"), Fr::from(0u64));
}

/// The impure twin of `field_eq_operand`: the walker's comparison
/// renderer carries the same Field-if coercion inside the assert. The
/// passing side commits; the failing side returns AssertionFailed.
#[test]
fn assert_field_eq_operand_passes_and_fails() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let one = Fr::from(1u64);
    let after_pass = contract
        .assert_field_eq_operand(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            one,
            true,
        )
        .expect("1 == 1 must commit");
    let view = ledger(&after_pass.context.current_query_context.state);
    assert_eq!(view.picks().expect("picks"), 1);

    let err = match contract.assert_field_eq_operand(
        CircuitContext::new(after_pass.context.current_query_context.state, ()),
        one,
        false,
    ) {
        Err(e) => e,
        Ok(_) => panic!("1 == 0 must trip the assert"),
    };
    assert!(
        matches!(err, CompactError::AssertionFailed(_)),
        "expected AssertionFailed, got {err:?}"
    );
}

/// The STREAMING route's both-literal const (`streamLiteralPick`): the
/// streaming const emitter predates the literal-arm coercion, so the
/// same `shown ? 10 : 20` the walker route suffixed emitted an
/// i32-defaulted if into `new_cell` (E0277). The non-terminal `if`
/// forces the streaming route; both flag directions commit their arm.
#[test]
fn stream_literal_pick_commits_each_suffixed_arm() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let after_true = contract
        .stream_literal_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("hot = true must commit");
    let view = ledger(&after_true.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 10);
    // picks: 1 (then branch) + 3 (terminal) = 4 — the write targets
    // lastPick, not picks.
    assert_eq!(view.picks().expect("picks"), 4);

    let after_false = contract
        .stream_literal_pick(
            CircuitContext::new(after_true.context.current_query_context.state, ()),
            false,
        )
        .expect("hot = false must commit");
    let view = ledger(&after_false.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 20);
    // picks: 4 + 2 (else branch) + 3 (terminal) = 9.
    assert_eq!(view.picks().expect("picks"), 9);
}

/// The streaming-route byte pin for the both-literal const write — the
/// twin of `record_literal_pick_byte_parity` through
/// emit-streaming-body.
#[test]
fn stream_literal_pick_byte_parity() {
    let ts_ref = TernaryTsReference::load();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    // Reproduce the capture chain: recordLiteralPick(true) then
    // streamLiteralPick(true) from its post-state.
    let after_record = contract
        .record_literal_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("record_literal_pick(true) must commit");
    let after = contract
        .stream_literal_pick(
            CircuitContext::new(after_record.context.current_query_context.state, ()),
            true,
        )
        .expect("stream_literal_pick(true) must commit");

    let envelope = make_envelope(after.context.current_query_context.state);
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_stream_literal_pick.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust post-streamLiteralPick state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// THE streaming width pin (the dogfood-review round-2 regression): a
/// Uint<8> value written into the Uint<64> `lastPick` field through the
/// streaming route. The pre-fix streaming cell-write had no
/// destination-width cast, committing a 1-byte-aligned cell where every
/// other route — and the TS field descriptor — commits 8: identical
/// decoded value, divergent state bytes. The decoded assertion below
/// passes either way; only this byte comparison against the TS
/// reference catches the divergence.
#[test]
fn stream_narrow_write_commits_at_field_width() {
    let ts_ref = TernaryTsReference::load();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    // Reproduce the capture chain: recordLiteralPick(true) ->
    // streamLiteralPick(true) -> streamNarrowWrite(true, 7).
    let after_record = contract
        .record_literal_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("record_literal_pick(true) must commit");
    let after_lit = contract
        .stream_literal_pick(
            CircuitContext::new(after_record.context.current_query_context.state, ()),
            true,
        )
        .expect("stream_literal_pick(true) must commit");
    let after = contract
        .stream_narrow_write(
            CircuitContext::new(after_lit.context.current_query_context.state, ()),
            true,
            7,
        )
        .expect("stream_narrow_write(true, 7) must commit");

    // Decoded value: the 8-bit 7 zero-extended into lastPick.
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 7);

    // Committed bytes: must match the TS reference exactly.
    let envelope = make_envelope(after.context.current_query_context.state);
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_stream_narrow_write.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust post-streamNarrowWrite state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// INLINE both-literal write with no const binding (`inlineBigPick`):
/// the write site itself must size the arms from the destination field
/// — bare arms default the if to i32 and `5000000000` overflows even
/// that. Both flag directions commit their arm.
#[test]
fn inline_big_pick_commits_each_arm() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let after_true = contract
        .inline_big_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("hot = true must commit");
    let view = ledger(&after_true.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 5_000_000_000);

    let after_false = contract
        .inline_big_pick(
            CircuitContext::new(after_true.context.current_query_context.state, ()),
            false,
        )
        .expect("hot = false must commit");
    let view = ledger(&after_false.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 0);
}

/// Bug-12 (dogfood review, round 3): a Rust `if` EXPRESSION moves the
/// taken arm's value out of its owner, so a struct-typed arm that is a
/// bare var-ref and is re-read after the ternary must carry
/// `.clone()` — pre-fix the pure route emitted `let picked = if c { s }
/// else { s2 };` followed by the asserts' reads of `s`/`s2`, failing
/// E0382 at cargo build while compactc exited 0 (this very test — the
/// generated crate — would not compile). Every other value position
/// (call args, Bug-6 let RHS) already cloned non-Copy var-refs; only
/// the two ternary if-clauses were exposed. Both formal arms are
/// re-read by the circuit's asserts, so BOTH arms need the clone; the
/// formals' types are recorded, so the clones are exact rather than
/// over-clones. The returned struct pins WHICH arm won.
#[test]
fn clone_struct_pick_reuses_both_formal_arms() {
    // Fresh values per call: the pure fn takes its args by value, so
    // the test itself would move them otherwise.
    let picked = pure_circuits::clone_struct_pick(
        true,
        Choice {
            low: false,
            value: 2,
        },
        Choice {
            low: true,
            value: 3,
        },
    )
    .expect("then arm picks s; the post-pick asserts re-read s and s2");
    assert!(!picked.low && picked.value == 2);

    let picked = pure_circuits::clone_struct_pick(
        false,
        Choice {
            low: false,
            value: 2,
        },
        Choice {
            low: true,
            value: 3,
        },
    )
    .expect("else arm picks s2");
    assert!(picked.low && picked.value == 3);
}

/// Bug-12, the let-lifted twin: `s` is a LOCAL, not a formal, so its
/// type is not in `current-formal-arg-types` and the clone decision is
/// type-blind (an unrecorded local always clones — safe over-clone).
/// The local IS re-read after the pick (`s.low` in the return), so the
/// clone is load-bearing here too: without it that read fails E0382.
#[test]
fn clone_struct_local_reuses_the_lifted_local() {
    assert!(pure_circuits::clone_struct_local(true).expect("then arm picks s (low = true)"));
    assert!(
        !pure_circuits::clone_struct_local(false).expect("else arm is a fresh ctor (low = false)")
    );
}

/// Bug-12 walker-route twin: the same both-formals shape through
/// ctor-expr-rust's if clause (the streaming route funnels into the
/// same clause, so it is covered too). Adding the impure circuit also
/// changes the TS `initialState()` envelope — it pre-registers every
/// impure circuit — so the whole ts-state.json was recaptured and this
/// executes the same circuit from the same post-init state on both
/// drivers to byte-compare the serialized envelopes, exactly like
/// `record_literal_pick_byte_parity` above.
#[test]
fn record_struct_pick_byte_parity() {
    let ts_ref = TernaryTsReference::load();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let after = contract
        .record_struct_pick(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
            Choice {
                low: false,
                value: 2,
            },
            Choice {
                low: true,
                value: 3,
            },
        )
        .expect("record_struct_pick(true, ..) must commit");

    // lastPick after init is 10 (CAPTURE_START's then branch); the
    // circuit writes picked.value (2 — the then arm IS s), with the
    // two asserts re-reading s/s2 before the write.
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.last_pick().expect("last_pick"), 2);

    let envelope = make_envelope(after.context.current_query_context.state);
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_record_struct_pick.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust post-recordStructPick state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

// ---------------------------------------------------------------------
// Round 4 (dogfood review): nested call arguments, struct-member
// initialisers, and the unannotated both-literal const to a Field
// return. Every position below emitted a BARE integer where `Fr` was
// required (E0308 at cargo build while compactc exited 0) before the
// callee-formal / struct-member / return-tail coercions landed; these
// executing tests pin both the compilation and the committed arm
// values.

/// NESTED CALL ARGUMENT (pure route, mixed arm): `fieldId(flag ? x :
/// 0)` — the callee's Field formal is the only type context the
/// position has, and an integer literal never unifies with the other
/// arm's `Fr`.
#[test]
fn field_call_arg_pick_returns_each_arm() {
    let x = Fr::from(42u64);
    assert_eq!(
        pure_circuits::field_call_arg_pick(true, x).expect("then arm"),
        x
    );
    assert_eq!(
        pure_circuits::field_call_arg_pick(false, x).expect("else arm"),
        Fr::from(0u64)
    );
}

/// NESTED CALL ARGUMENT (pure route, both-literal arms): nothing in
/// the arms gives inference a type, so both arms must render
/// `Fr::from(<n>u64)` from the callee's formal.
#[test]
fn field_call_arg_both_lit_returns_each_arm() {
    assert_eq!(
        pure_circuits::field_call_arg_both_lit(true).expect("then arm"),
        Fr::from(1u64)
    );
    assert_eq!(
        pure_circuits::field_call_arg_both_lit(false).expect("else arm"),
        Fr::from(0u64)
    );
}

/// STRUCT-LITERAL member initialisers: `f` carries the mixed arm, `g`
/// the both-literal arm — each member's declared Field type drives its
/// own coercion, independently of the other.
#[test]
fn field_struct_pick_coerces_each_member() {
    let x = Fr::from(9u64);
    let hot = pure_circuits::field_struct_pick(true, x).expect("then arms");
    assert_eq!(hot.f, x);
    assert_eq!(hot.g, Fr::from(1u64));

    let cold = pure_circuits::field_struct_pick(false, x).expect("else arms");
    assert_eq!(cold.f, Fr::from(0u64));
    assert_eq!(cold.g, Fr::from(0u64));
}

/// The UNANNOTATED `const picked = flag ? 1 : 0; return picked;` in a
/// Field circuit — the let*-lifted `(safe-cast tfield (seq (= picked
/// <lit-if>) picked))` tail. The seq wrapper escaped the return-tail
/// literal checks (which strip only safe-cast layers), so the tail fell
/// through to the type-context-free degenerate-seq inline and emitted
/// `Ok(if flag { 1 } else { 0 })` against `Result<Fr, _>`.
#[test]
fn const_field_pick_sizes_arms_from_the_field_return() {
    assert_eq!(
        pure_circuits::const_field_pick(true).expect("hot arm"),
        Fr::from(1u64)
    );
    assert_eq!(
        pure_circuits::const_field_pick(false).expect("cold arm"),
        Fr::from(0u64)
    );
}

/// The same unannotated const consumed by FIELD ARITHMETIC: the typer
/// wraps the Uint-typed binding in a safe-cast to Field at the `+`,
/// which the field-operand renderer materialises as
/// `Fr::from((picked) as u64)`.
#[test]
fn picked_field_arith_widens_the_unannotated_const() {
    let x = Fr::from(10u64);
    assert_eq!(
        pure_circuits::picked_field_arith(true, x).expect("hot arm: 1 + x"),
        Fr::from(11u64)
    );
    assert_eq!(
        pure_circuits::picked_field_arith(false, x).expect("cold arm: 0 + x"),
        x
    );
}

/// The IMPURE (walker-route) twin: the nested ternary as a call
/// argument in a const-binding, in both-literal form, and in
/// expression position (`origin.write(disclose(fieldId(..)))`) — all
/// three render through walker-side call-arg paths that previously
/// carried no Field context. The committed origin value pins the
/// expression-position write.
#[test]
fn record_field_call_arg_commits_the_mixed_join() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CAPTURE_START)
        .expect("initial_state");

    let x = Fr::from(5u64);
    let after = contract
        .record_field_call_arg(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            x,
            true,
        )
        .expect("record_field_call_arg(5, true) must commit");
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.origin().expect("origin"), x);

    let after_false = contract
        .record_field_call_arg(
            CircuitContext::new(after.context.current_query_context.state, ()),
            x,
            false,
        )
        .expect("record_field_call_arg(5, false) must commit");
    let view = ledger(&after_false.context.current_query_context.state);
    assert_eq!(view.origin().expect("origin"), Fr::from(0u64));
}
