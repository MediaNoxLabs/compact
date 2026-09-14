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
// Conditional (ternary) expression coverage gate.
//
// `ternary_cond_fixture.compact` puts a conditional expression in every
// sub-expression position and body route the language reaches. This test
// is the "executable probe" arm of the coverage matrix (declared in the
// change's tasks.md): byte-parity (`codegen_regression`) locks the
// generated TEXT, and these assertions lock the MEANING — a lowering
// that compiles but eagerly evaluates both arms, drops a guard, or
// narrows a value to the wrong width cannot pass.
//
// Three families of assertion:
//   1. State-byte parity — the constructor (both arms) and each
//      ledger-writing impure route (`walkerWrite`, `streamIncrement`,
//      `streamWrite`). The serialized `ContractState` bytes must equal
//      the TS capture (tests-e2e-rust/fixtures/ternary-cond-fixture-ts-state.json),
//      so an alignment/width divergence in a ledger-write cell is caught
//      where a decoded-value check would not be.
//   2. Laziness — `constUnannotatedSeqLifted(true, 0)` must NOT evaluate
//      the underflowing `a - 1` (the branch guard lives INSIDE the taken
//      arm), while `(true, 5)` computes `4` and `(false, 0)` skips it.
//   3. Per-position round-trips in the pure route, plus the impure routes
//      driven directly (ledger read-back).
//
// The witness-call position is exercised through the supported
// TOP-LEVEL BINDING shape (`const w = echoField(c ? 1 : 2);`); a witness
// call as a general sub-expression is a pre-existing refusal
// (`witness-inline`, see docs/rust-backend-limitations.md).
#![allow(clippy::unit_arg)]

use compact_contract_ternary_cond_fixture::{ledger, pure_circuits, Contract, Ledger, Witnesses};
use midnight_compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// TS reference state
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct TernaryCondTsReference {
    #[serde(rename = "afterInit")]
    after_init: TernaryCondStep,
    #[serde(rename = "afterInitFalse")]
    after_init_false: TernaryCondStep,
    #[serde(rename = "afterWalkerConstAnnotated")]
    after_walker_const_annotated: TernaryCondStep,
    #[serde(rename = "afterWalkerCompareEq")]
    after_walker_compare_eq: TernaryCondStep,
    #[serde(rename = "afterWalkerCallPure")]
    after_walker_call_pure: TernaryCondStep,
    #[serde(rename = "afterWalkerStructMember")]
    after_walker_struct_member: TernaryCondStep,
    #[serde(rename = "afterWalkerWrite")]
    after_walker_write: TernaryCondStep,
    #[serde(rename = "afterStreamIncrement")]
    after_stream_increment: TernaryCondStep,
    #[serde(rename = "afterStreamCompareEq")]
    after_stream_compare_eq: TernaryCondStep,
    #[serde(rename = "afterStreamWrite")]
    after_stream_write: TernaryCondStep,
}

#[derive(Deserialize, Debug)]
struct TernaryCondStep {
    #[serde(rename = "stateHex")]
    state_hex: String,
}

impl TernaryCondStep {
    fn state_bytes(&self) -> Vec<u8> {
        hex::decode(&self.state_hex).expect("decode fixture hex")
    }
}

fn fixture() -> TernaryCondTsReference {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/ternary-cond-fixture-ts-state.json"
    ))
    .expect("read ternary-cond fixture");
    serde_json::from_str(&raw).expect("parse ternary-cond fixture")
}

// Constructor / circuit arguments — MUST match
// fixtures/capture-ternary-cond-fixture.mjs.
const CTOR_C_TRUE: bool = true;
const CTOR_D_TRUE: bool = true;
const CTOR_X_TRUE: u64 = 111;
const CTOR_C_FALSE: bool = false;
const CTOR_D_FALSE: bool = false;
const CTOR_X_FALSE: u64 = 222;
const WALKER_C: bool = true;
const WALKER_X: u64 = 555;
const COMPARE_X: u8 = 1;
const STREAM_WRITE_C: bool = false;
const STREAM_WRITE_X: u64 = 777;

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// Deterministic `Witnesses` impl: `echo_field` echoes its argument, so
/// `witnessArg` writes the selected arm straight through.
struct FixtureWitnesses;

impl Witnesses<()> for FixtureWitnesses {
    fn echo_field<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>, x: Fr) -> ((), Fr) {
        ((), x)
    }
}

/// The four EXPORTED (impure) circuits — the operations map the TS
/// `initialState()` produces (pure circuits live in `pure_circuits` and
/// are not part of the dispatch map).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    for name in [
        "streamAssertEq",
        "streamCallPure",
        "streamCallWitness",
        "streamCompareEq",
        "streamConstAnnotated",
        "streamIncrement",
        "streamNativeArg",
        "streamNestedIf",
        "streamStructMember",
        "streamVectorElement",
        "streamWrite",
        "walkerCallPure",
        "walkerCompareEq",
        "walkerConstAnnotated",
        "walkerInlineWrite",
        "walkerNativeArg",
        "walkerNestedIf",
        "walkerStructMember",
        "walkerVectorElement",
        "walkerWrite",
        "witnessArg",
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

fn state_bytes(state: ChargedState<midnight_storage::DefaultDB>) -> Vec<u8> {
    let envelope = make_envelope(state);
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");
    buf
}

fn assert_state_parity(
    rust_state: ChargedState<midnight_storage::DefaultDB>,
    ts_step: &TernaryCondStep,
    label: &str,
) {
    let rust_bytes = state_bytes(rust_state);
    let ts_bytes = ts_step.state_bytes();
    assert_eq!(
        rust_bytes,
        ts_bytes,
        "{label}: Rust state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        rust_bytes.len(),
        hex::encode(&rust_bytes),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

// ---------------------------------------------------------------------------
// 1. State-byte parity (ledger-writing routes)
// ---------------------------------------------------------------------------

/// Constructor route, `c = true`: `fieldCell = 111`, `wideCell = 10`,
/// `vecCell = [1, 3]`. The `Uint<64>` cell is the alignment detector —
/// a 1-byte atom would decode to `10` but serialize differently.
#[test]
fn ternary_cond_init_true_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let result = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    assert_state_parity(
        result.current_contract_state,
        &ts.after_init,
        "constructor (c=true)",
    );
}

/// Constructor route, `c = false`: the other arm of every conditional
/// ledger-write cell (`fieldCell = 222`, `wideCell = 20`, `vecCell = [2, 4]`).
#[test]
fn ternary_cond_init_false_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let result = contract
        .initial_state(
            ctor_ctx(),
            CTOR_C_FALSE,
            CTOR_D_FALSE,
            Fr::from(CTOR_X_FALSE),
        )
        .expect("initial_state");
    assert_state_parity(
        result.current_contract_state,
        &ts.after_init_false,
        "constructor (c=false)",
    );
}

/// Impure-walker route: a terminal ledger-cell write whose value is a
/// conditional.
#[test]
fn ternary_cond_walker_write_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .walker_write(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            WALKER_C,
            Fr::from(WALKER_X),
        )
        .expect("walker_write");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_walker_write,
        "walkerWrite",
    );
}

/// Impure-walker route: const RHS with a conditional, written to a cell.
#[test]
fn ternary_cond_walker_const_annotated_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .walker_const_annotated(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            WALKER_C,
        )
        .expect("walker_const_annotated");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_walker_const_annotated,
        "walkerConstAnnotated",
    );
}

/// Impure-walker route: a conditional as a comparison (equality) operand.
#[test]
fn ternary_cond_walker_compare_eq_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .walker_compare_eq(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            WALKER_C,
            COMPARE_X,
        )
        .expect("walker_compare_eq");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_walker_compare_eq,
        "walkerCompareEq",
    );
}

/// Impure-walker route: a conditional as a call argument to a pure circuit.
#[test]
fn ternary_cond_walker_call_pure_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .walker_call_pure(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            WALKER_C,
        )
        .expect("walker_call_pure");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_walker_call_pure,
        "walkerCallPure",
    );
}

/// Impure-walker route: a conditional as a struct-literal member.
#[test]
fn ternary_cond_walker_struct_member_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .walker_struct_member(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            WALKER_C,
        )
        .expect("walker_struct_member");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_walker_struct_member,
        "walkerStructMember",
    );
}

/// Impure-streaming route: a ledger read (gather) interleaved with a
/// conditional inline `increment` and a conditional cell write.
#[test]
fn ternary_cond_stream_increment_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_increment(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_increment");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_stream_increment,
        "streamIncrement",
    );
}

/// Impure-streaming route: a ledger read interleaved with a conditional
/// comparison (equality) operand written to a cell.
#[test]
fn ternary_cond_stream_compare_eq_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_compare_eq(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_compare_eq");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_stream_compare_eq,
        "streamCompareEq",
    );
}

/// Impure-streaming route: a non-terminal ledger call carrying a
/// conditional, followed by a cell write.
#[test]
fn ternary_cond_stream_write_byte_parity() {
    let ts = fixture();
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_write(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            STREAM_WRITE_C,
            Fr::from(STREAM_WRITE_X),
        )
        .expect("stream_write");
    assert_state_parity(
        out.context.current_query_context.state,
        &ts.after_stream_write,
        "streamWrite",
    );
}

// ---------------------------------------------------------------------------
// 2. Laziness (both directions)
// ---------------------------------------------------------------------------

/// The branch-local underflow guard must live INSIDE the taken arm: an
/// eager lowering would evaluate `a - 1` for the untaken branch and trap
/// `(false, 0)`, and a dropped guard would let `(true, 0)` wrap instead
/// of trapping.
#[test]
fn branch_local_guard_is_lazy() {
    // Taken underflow arm traps.
    let err = pure_circuits::const_unannotated_seq_lifted(true, 0)
        .expect_err("a - 1 with a = 0 must trip the underflow guard");
    assert!(
        matches!(err, CompactError::AssertionFailed(ref m) if m == "result of subtraction would be negative"),
        "expected the subtraction guard, got {err:?}"
    );
    // Taken arm computes.
    assert_eq!(
        pure_circuits::const_unannotated_seq_lifted(true, 5).expect("5 - 1"),
        4
    );
    // Untaken arm is never evaluated: `a = 0` would underflow, but the
    // condition is false so the subtraction is skipped.
    assert_eq!(
        pure_circuits::const_unannotated_seq_lifted(false, 0).expect("untaken arm must not trap"),
        0
    );
}

/// The language-spec laziness shape (`compact-reference-proto.mdx:2114`):
/// `pick(c) { return c > 5 ? c - 10 : c; }`. With `c = 3` the untaken
/// `c - 10` (which would underflow) is never evaluated; with `c = 12` the
/// taken branch computes `2`.
#[test]
fn pick_is_lazy() {
    assert_eq!(
        pure_circuits::pick(3).expect("untaken branch must not trap"),
        3
    );
    assert_eq!(pure_circuits::pick(12).expect("taken branch computes"), 2);
}

// ---------------------------------------------------------------------------
// 3. Per-position round-trips (pure route)
// ---------------------------------------------------------------------------

/// const RHS, annotated, both-literal arms.
#[test]
fn const_annotated_both_literal_round_trips() {
    assert_eq!(
        pure_circuits::const_annotated_both_literal(true).expect("true"),
        1
    );
    assert_eq!(
        pure_circuits::const_annotated_both_literal(false).expect("false"),
        2
    );
}

/// Return tail with mixed literal/expr arms.
#[test]
fn return_tail_mixed_round_trips() {
    assert_eq!(pure_circuits::return_tail_mixed(true, 42).expect("lit"), 5);
    assert_eq!(
        pure_circuits::return_tail_mixed(false, 42).expect("expr"),
        42
    );
}

/// Nested conditional in the return tail (recursive arms).
#[test]
fn nested_conditional_round_trips() {
    assert_eq!(
        pure_circuits::return_tail_nested(true, true).expect("tt"),
        1
    );
    assert_eq!(
        pure_circuits::return_tail_nested(true, false).expect("tf"),
        2
    );
    assert_eq!(
        pure_circuits::return_tail_nested(false, true).expect("ft"),
        3
    );
    assert_eq!(
        pure_circuits::return_tail_nested(false, false).expect("ff"),
        4
    );
}

/// Assert argument: both the passing and failing sides of the
/// conditional condition.
#[test]
fn assert_arg_both_sides() {
    pure_circuits::assert_arg(true, 5).expect("5 > 1 must pass");
    pure_circuits::assert_arg(false, 5).expect("5 > 2 must pass");
    let err = pure_circuits::assert_arg(false, 2).expect_err("2 > 2 must fail");
    assert!(
        matches!(err, CompactError::AssertionFailed(ref m) if m == "ternary assert"),
        "expected the ternary assert, got {err:?}"
    );
}

/// Arithmetic operand.
#[test]
fn arith_operand_round_trips() {
    assert_eq!(pure_circuits::arith_operand(true, 10).expect("+1"), 11);
    assert_eq!(pure_circuits::arith_operand(false, 10).expect("+0"), 10);
}

/// Comparison operand.
#[test]
fn cmp_operand_round_trips() {
    assert!(pure_circuits::cmp_operand(true, 1).expect("1 == 1"));
    assert!(!pure_circuits::cmp_operand(true, 2).expect("2 != 1"));
    assert!(pure_circuits::cmp_operand(false, 0).expect("0 == 0"));
}

/// Call argument to a pure user circuit.
#[test]
fn call_arg_pure_round_trips() {
    assert_eq!(
        pure_circuits::call_arg_pure(true, Fr::from(7u64), Fr::from(8u64)).expect("a"),
        Fr::from(7u64)
    );
    assert_eq!(
        pure_circuits::call_arg_pure(false, Fr::from(7u64), Fr::from(8u64)).expect("b"),
        Fr::from(8u64)
    );
}

/// Call argument to `some<T>` (a ctor call).
#[test]
fn call_arg_ctor_round_trips() {
    assert_eq!(
        pure_circuits::call_arg_ctor(true, Fr::from(7u64), Fr::from(8u64)).expect("some a"),
        midnight_compact_runtime::std_lib::some(Fr::from(7u64))
    );
    assert_eq!(
        pure_circuits::call_arg_ctor(false, Fr::from(7u64), Fr::from(8u64)).expect("some b"),
        midnight_compact_runtime::std_lib::some(Fr::from(8u64))
    );
}

/// Struct-literal member.
#[test]
fn struct_member_round_trips() {
    assert_eq!(
        pure_circuits::struct_member(true).expect("member 1").f,
        Fr::from(1u64)
    );
    assert_eq!(
        pure_circuits::struct_member(false).expect("member 2").f,
        Fr::from(2u64)
    );
}

/// Struct-valued (non-`Copy`) arms — the Rust `if` moves the selected
/// arm.
#[test]
fn struct_valued_arms_round_trips() {
    let s1 = compact_contract_ternary_cond_fixture::Box { f: Fr::from(3u64) };
    let s2 = compact_contract_ternary_cond_fixture::Box { f: Fr::from(4u64) };
    assert_eq!(
        pure_circuits::struct_valued_arms(true, s1.clone(), s2.clone())
            .expect("s1")
            .f,
        Fr::from(3u64)
    );
    assert_eq!(
        pure_circuits::struct_valued_arms(false, s1, s2)
            .expect("s2")
            .f,
        Fr::from(4u64)
    );
}

/// Vector element (per-element conditional coercion into `Field`).
#[test]
fn vector_element_round_trips() {
    assert_eq!(
        pure_circuits::vector_element(true).expect("true"),
        [Fr::from(1u64), Fr::from(3u64)]
    );
    assert_eq!(
        pure_circuits::vector_element(false).expect("false"),
        [Fr::from(2u64), Fr::from(4u64)]
    );
}

/// Native argument.
#[test]
fn native_arg_round_trips() {
    assert_eq!(
        pure_circuits::native_arg(true, Fr::from(5u64), Fr::from(6u64)).expect("a"),
        midnight_compact_runtime::hash_to_curve(Fr::from(5u64))
    );
    assert_eq!(
        pure_circuits::native_arg(false, Fr::from(5u64), Fr::from(6u64)).expect("b"),
        midnight_compact_runtime::hash_to_curve(Fr::from(6u64))
    );
}

/// Field-typed arms.
#[test]
fn field_typed_arms_round_trips() {
    assert_eq!(
        pure_circuits::field_typed_arms(true, Fr::from(9u64), Fr::from(10u64)).expect("f1"),
        Fr::from(9u64)
    );
    assert_eq!(
        pure_circuits::field_typed_arms(false, Fr::from(9u64), Fr::from(10u64)).expect("f2"),
        Fr::from(10u64)
    );
}

/// Enum-valued arms.
#[test]
fn enum_valued_round_trips() {
    assert_eq!(
        pure_circuits::enum_valued(true).expect("red"),
        compact_contract_ternary_cond_fixture::Color::red
    );
    assert_eq!(
        pure_circuits::enum_valued(false).expect("green"),
        compact_contract_ternary_cond_fixture::Color::green
    );
}

/// Literal arm above `i32::MAX` (but within `u64`).
#[test]
fn literal_above_i32_round_trips() {
    assert_eq!(
        pure_circuits::literal_above_i32(true).expect("big"),
        3_000_000_000u64
    );
    assert_eq!(pure_circuits::literal_above_i32(false).expect("zero"), 0u64);
}

/// Literal arm above `u64::MAX` (the `Uint<128>` rung).
#[test]
fn literal_above_u64_round_trips() {
    assert_eq!(
        pure_circuits::literal_above_u64(true).expect("u128 max"),
        u128::MAX
    );
    assert_eq!(
        pure_circuits::literal_above_u64(false).expect("zero"),
        0u128
    );
}

/// Arms of differing Uint widths — the narrower arm is widened, not
/// truncated.
#[test]
fn differing_uint_widths_round_trips() {
    assert_eq!(
        pure_circuits::differing_uint_widths(false, 5, 4_000_000_000).expect("wide b"),
        4_000_000_000u32
    );
    assert_eq!(
        pure_circuits::differing_uint_widths(true, 5, 4_000_000_000).expect("narrow a"),
        5u32
    );
}

/// Uint-ranged arm flowing into a `Field` position.
#[test]
fn uint_arm_into_field_round_trips() {
    assert_eq!(
        pure_circuits::uint_arm_into_field(true, 7, 9).expect("a"),
        Fr::from(7u64)
    );
    assert_eq!(
        pure_circuits::uint_arm_into_field(false, 7, 9).expect("b"),
        Fr::from(9u64)
    );
}

// ---------------------------------------------------------------------------
// 4. Impure routes driven directly (ledger read-back)
// ---------------------------------------------------------------------------

/// `walkerWrite` writes the selected arm into `fieldCell`.
#[test]
fn walker_write_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .walker_write(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
            Fr::from(555u64),
        )
        .expect("walker_write");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(555u64));
    assert_eq!(view.wide_cell().expect("wide_cell"), 10u64);
}

/// `streamIncrement` reads `flag` (false) and takes the else arms:
/// `ops += 4`, `wideCell = 20`.
#[test]
fn stream_increment_takes_the_else_arms() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_increment(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_increment");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.ops().expect("ops"), 4u64);
    assert_eq!(view.wide_cell().expect("wide_cell"), 20u64);
}

/// `streamWrite` conditionally increments `ops` (`c = false` → `+2`) then
/// writes `fieldCell`.
#[test]
fn stream_write_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_write(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            false,
            Fr::from(777u64),
        )
        .expect("stream_write");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.ops().expect("ops"), 2u64);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(777u64));
}

/// `witnessArg`: the conditional argument to the witness call selects the
/// arm, and the echoed value is written into `fieldCell`.
#[test]
fn witness_arg_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");

    let out_true = contract
        .witness_arg(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            true,
        )
        .expect("witness_arg true");
    let view = ledger(&out_true.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(1u64));

    let out_false = contract
        .witness_arg(
            CircuitContext::new(init.current_contract_state.clone(), ()),
            false,
        )
        .expect("witness_arg false");
    let view = ledger(&out_false.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(2u64));
}

/// `walkerConstAnnotated`: the annotated const binding's conditional value
/// is written to `fieldCell`.
#[test]
fn walker_const_annotated_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, expected) in [(true, 1u64), (false, 2u64)] {
        let out = contract
            .walker_const_annotated(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
            )
            .expect("walker_const_annotated");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.field_cell().expect("field_cell"), Fr::from(expected));
    }
}

/// `walkerCompareEq`: the conditional comparison (equality) operand selects
/// the boolean written to `fieldCell`.
#[test]
fn walker_compare_eq_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    // (x, c, expected): `x == (c ? 1 : 0)`
    for (x, c, expected) in [(1u8, true, 1u64), (2u8, true, 0u64), (0u8, false, 1u64)] {
        let out = contract
            .walker_compare_eq(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
                x,
            )
            .expect("walker_compare_eq");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.field_cell().expect("field_cell"), Fr::from(expected));
    }
}

/// `walkerCallPure`: the conditional argument of a pure call is written to
/// `fieldCell`.
#[test]
fn walker_call_pure_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, expected) in [(true, 1u64), (false, 2u64)] {
        let out = contract
            .walker_call_pure(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
            )
            .expect("walker_call_pure");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.field_cell().expect("field_cell"), Fr::from(expected));
    }
}

/// `walkerStructMember`: the conditional struct-literal member is written to
/// `fieldCell`.
#[test]
fn walker_struct_member_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, expected) in [(true, 1u64), (false, 2u64)] {
        let out = contract
            .walker_struct_member(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
            )
            .expect("walker_struct_member");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.field_cell().expect("field_cell"), Fr::from(expected));
    }
}

/// `streamCompareEq`: `flag` seeds false, so the conditional comparison
/// operand selects `0 == 1` → false, written to `fieldCell`.
#[test]
fn stream_compare_eq_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_compare_eq(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_compare_eq");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(0u64));
}

/// `walkerVectorElement`: conditional vector elements are written to
/// `vecCell`.
#[test]
fn walker_vector_element_selects_the_arms() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, expected) in [
        (true, [Fr::from(1u64), Fr::from(3u64)]),
        (false, [Fr::from(2u64), Fr::from(4u64)]),
    ] {
        let out = contract
            .walker_vector_element(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
            )
            .expect("walker_vector_element");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.vec_cell().expect("vec_cell"), expected);
    }
}

/// `walkerNativeArg`: the conditional native argument selects the curve
/// point whose X coordinate is written to `fieldCell`.
#[test]
fn walker_native_arg_selects_the_arms() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, expected) in [
        (
            true,
            midnight_compact_runtime::jubjub_point_x(midnight_compact_runtime::hash_to_curve(
                Fr::from(1u64),
            )),
        ),
        (
            false,
            midnight_compact_runtime::jubjub_point_x(midnight_compact_runtime::hash_to_curve(
                Fr::from(2u64),
            )),
        ),
    ] {
        let out = contract
            .walker_native_arg(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
            )
            .expect("walker_native_arg");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.field_cell().expect("field_cell"), expected);
    }
}

/// `walkerNestedIf`: the nested conditional selects the arm written to
/// `fieldCell`.
#[test]
fn walker_nested_if_selects_the_arms() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, d, expected) in [
        (true, true, 1u64),
        (true, false, 2u64),
        (false, true, 3u64),
        (false, false, 4u64),
    ] {
        let out = contract
            .walker_nested_if(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
                d,
            )
            .expect("walker_nested_if");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.field_cell().expect("field_cell"), Fr::from(expected));
    }
}

/// `walkerInlineWrite`: the conditional inline `increment` value selects
/// the amount added to `ops`.
#[test]
fn walker_inline_write_selects_the_arms() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    for (c, expected) in [(true, 1u64), (false, 2u64)] {
        let out = contract
            .walker_inline_write(
                CircuitContext::new(init.current_contract_state.clone(), ()),
                c,
            )
            .expect("walker_inline_write");
        let view = ledger(&out.context.current_query_context.state);
        assert_eq!(view.ops().expect("ops"), expected);
    }
}

/// `streamCallPure`: `flag` seeds false, so the conditional argument to a
/// pure call selects `idf(2)` → `2`, written to `fieldCell`.
#[test]
fn stream_call_pure_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_call_pure(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_call_pure");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(2u64));
}

/// `streamVectorElement`: `flag` seeds false, so the conditional vector
/// elements select `[2, 4]`, written to `vecCell`.
#[test]
fn stream_vector_element_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_vector_element(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_vector_element");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(
        view.vec_cell().expect("vec_cell"),
        [Fr::from(2u64), Fr::from(4u64)]
    );
}

/// `streamNativeArg`: `flag` seeds false, so the conditional native
/// argument selects `hashToCurve(2)`, whose X coordinate is written to
/// `fieldCell`.
#[test]
fn stream_native_arg_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_native_arg(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_native_arg");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(
        view.field_cell().expect("field_cell"),
        midnight_compact_runtime::jubjub_point_x(midnight_compact_runtime::hash_to_curve(
            Fr::from(2u64)
        ))
    );
}

/// `streamStructMember`: `flag` seeds false, so the conditional struct
/// member selects `2`, written to `fieldCell`.
#[test]
fn stream_struct_member_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_struct_member(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_struct_member");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(2u64));
}

/// `streamCallWitness`: `flag` seeds false, so the conditional witness
/// argument selects `2`, echoed through `echoField` and written to
/// `fieldCell`.
#[test]
fn stream_call_witness_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_call_witness(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_call_witness");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(2u64));
}

/// `walkerReturnTail` / `walkerCallCtor` carry a ternary in return-tail /
/// call-argument position; with no ledger ops they lower to pure circuits.
#[test]
fn pure_fallthrough_return_probes_round_trip() {
    assert_eq!(
        pure_circuits::walker_return_tail(true).expect("return tail true"),
        Fr::from(1u64)
    );
    assert_eq!(
        pure_circuits::walker_return_tail(false).expect("return tail false"),
        Fr::from(2u64)
    );
    assert_eq!(
        pure_circuits::walker_call_ctor(true).expect("call ctor true"),
        midnight_compact_runtime::std_lib::some(Fr::from(1u64))
    );
    assert_eq!(
        pure_circuits::walker_call_ctor(false).expect("call ctor false"),
        midnight_compact_runtime::std_lib::some(Fr::from(2u64))
    );
}

/// `streamConstAnnotated`: `flag` seeds false, so the annotated const's
/// conditional value selects `2`, incremented into `ops`; `fieldCell`
/// holds the constant `1`.
#[test]
fn stream_const_annotated_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_const_annotated(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_const_annotated");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.ops().expect("ops"), 2u64);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(1u64));
}

/// `streamAssertEq`: `flag` seeds false, so the conditional assert (an
/// equality) holds.
#[test]
fn stream_assert_eq_holds() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_assert_eq(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_assert_eq");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(1u64));
}

/// `streamNestedIf`: `flag` seeds false, so the nested conditional selects
/// `4`, written to `fieldCell`.
#[test]
fn stream_nested_if_selects_the_arm() {
    let contract: Contract<(), FixtureWitnesses> = Contract::new(FixtureWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), CTOR_C_TRUE, CTOR_D_TRUE, Fr::from(CTOR_X_TRUE))
        .expect("initial_state");
    let out = contract
        .stream_nested_if(CircuitContext::new(init.current_contract_state.clone(), ()))
        .expect("stream_nested_if");
    let view = ledger(&out.context.current_query_context.state);
    assert_eq!(view.field_cell().expect("field_cell"), Fr::from(4u64));
}
