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

// The generated pure circuits return `Result<(), _>`, so passing a
// unit-returning expression through is the correct semantic use even though
// clippy's `unit_arg` lint flags it.
#![allow(clippy::unit_arg)]
//
// Executing gate for examples/asset_registry_fixture.compact.
//
// Byte-parity (codegen_regression) locks the generated TEXT, which cannot
// see a wrong-but-stable scaffold, a decode that inverts the wrong
// encoding, or a swapped arithmetic operand. Everything below therefore
// RUNS the generated code:
//
//   - `initial_state_readback_via_ledger_accessors` — the >16-field
//     chunked scaffold (A29) plus the alignment-aware struct-cell decode
//     (A30/A31): the constructor writes fields on BOTH sides of the chunk
//     boundary and `registryId = kernel.self()`, and every scalar is read
//     back through the generated `ledger()` accessors.
//   - `constructor_flushes_writes_before_the_distinctness_assert` — the
//     read-your-writes invariant (A28), asserted structurally against the
//     generated source: a regeneration that reintroduces the
//     read-before-write and is committed as the new baseline would pass
//     byte-parity.
//   - the `custodian_*` tests — the same struct-cell decode over a FULL
//     32-byte atom rather than the constructor's all-zero one.
//   - the `map_*` / `set_*` tests — insert then look the value back up
//     through the contract's own circuits, so composite
//     `decode_via_field_repr::<AssetRecord>` / `::<CustodyGrant>` reads
//     execute against real state.
//   - the `guarded_*` tests — the struct-field projection inside trapping
//     arithmetic (G1), asserted both passing and tripping.

use compact_contract_asset_registry_fixture::{
    ledger, pure_circuits, AssetClass, AssetRecord, Contract, ContractAddress as RegistryAddress,
    CustodyGrant, FreshnessPolicy, Ledger, Provenance, RecordMutation, Witnesses,
};
use compact_runtime::std_lib::OpaqueString;
use compact_runtime::*;

mod common;

const TIMESTAMP: u64 = 1_700_000_000;
const MAX_AGE_SECONDS: u64 = 86_400;

/// Deterministic stub witnesses. The constructor calls
/// `local_operator_key` / `local_auditor_key` and then asserts they are
/// DISTINCT valid curve points, so the two must not collide.
struct StubWitnesses;

impl Witnesses<()> for StubWitnesses {
    fn local_operator_key<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), JubjubPoint) {
        ((), hash_to_curve(Fr::from(1u64)))
    }

    fn local_auditor_key<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), JubjubPoint) {
        ((), hash_to_curve(Fr::from(2u64)))
    }

    fn current_timestamp<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), u64) {
        ((), TIMESTAMP)
    }
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

fn contract() -> Contract<(), StubWitnesses> {
    Contract::new(StubWitnesses)
}

/// `label = pad(32, "asset-registry:v3")`: the UTF-8 bytes followed by
/// zero padding to the declared width.
fn expected_label() -> [u8; 32] {
    let mut out = [0u8; 32];
    let src = b"asset-registry:v3";
    out[..src.len()].copy_from_slice(src);
    out
}

/// Pack an ASCII label into the fixed-width `Bytes<32>` leaves the
/// structs use — same convention as Compact's `pad(32, "...")`.
fn code(label: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let src = label.as_bytes();
    assert!(src.len() <= 32, "label longer than the Bytes<32> leaf");
    out[..src.len()].copy_from_slice(src);
    out
}

fn record(label: &str, registered_at: u64, quantity: u64) -> AssetRecord {
    AssetRecord {
        code: code(label),
        // Empty: `OpaqueString::FIELD_SIZE == 0`, so only an empty
        // variable-length leaf round-trips through a composite decode.
        // `map_lookup_rejects_a_non_empty_variable_length_leaf` pins the
        // other half of that contract.
        note: OpaqueString::from(""),
        provenance: Provenance {
            facility: code("north-yard"),
            registeredAt: registered_at,
        },
        kind: AssetClass::Instrument,
        quantity,
    }
}

fn grant(label: &str, holder: RegistryAddress, granted_at: u64) -> CustodyGrant {
    CustodyGrant {
        code: code(label),
        holder,
        grantedAt: granted_at,
    }
}

fn policy(enforce: bool, max_age: u64) -> FreshnessPolicy {
    FreshnessPolicy {
        enforceMaxAge: enforce,
        maxAge: max_age,
    }
}

/// A full, non-zero 32-byte address — the complement of the constructor's
/// all-zero `kernel.self()` value, so the same
/// `decode_via_field_repr::<ContractAddress>` has to expand a saturated
/// atom as well as an empty one.
fn nonzero_address() -> RegistryAddress {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (i as u8).wrapping_add(1);
    }
    RegistryAddress { bytes }
}

/// A29 + A30/A31. 20 ledger fields put the state past the 16-slot
/// `StateValue::Array` cap, so the front end chunks it; the constructor
/// writes fields in BOTH chunks and every scalar is read back through the
/// generated accessors. A flat (unchunked) scaffold fails here even before
/// the first constructor write, because the reads walk the nested arrays
/// directly.
#[test]
fn initial_state_readback_via_ledger_accessors() {
    let result = contract().initial_state(ctor_ctx()).expect("initial_state");
    let view = ledger(&result.current_contract_state);

    // Chunk [0] — the leading fields, including both struct-typed cells.
    assert_eq!(view.schema_version().expect("schema_version"), 3u32);
    // `registryId = kernel.self()`: the constructor's query context runs
    // against the DEFAULT contract address, so the cell holds all zeros.
    // Stored atoms are normalised, so an all-zero 32-byte leaf normalises
    // to an EMPTY atom — the zero-length edge case of the alignment-aware
    // expansion in `decode_via_field_repr`.
    assert_eq!(
        view.registry_id().expect("registry_id"),
        RegistryAddress::default()
    );
    // Never written by the constructor: the default cell, decoded through
    // the same composite path.
    assert_eq!(
        view.custodian().expect("custodian"),
        RegistryAddress::default()
    );
    assert_eq!(
        view.operator_key().expect("operator_key"),
        hash_to_curve(Fr::from(1u64)),
    );
    assert_eq!(
        view.auditor_key().expect("auditor_key"),
        hash_to_curve(Fr::from(2u64)),
    );

    // Chunk [1] — everything at a global index past the boundary. These
    // are the reads a pre-A29 flat scaffold could not satisfy.
    //
    // `salt` is never written: an all-zero `Bytes<32>` cell, i.e. the
    // empty-atom case of the flat `decode_bytes::<32>` path.
    assert_eq!(view.salt().expect("salt"), [0u8; 32]);
    assert_eq!(view.label().expect("label"), expected_label());
    assert_eq!(view.created_at().expect("created_at"), TIMESTAMP);
    assert_eq!(view.updated_at().expect("updated_at"), TIMESTAMP);
    assert_eq!(
        view.max_age_seconds().expect("max_age_seconds"),
        MAX_AGE_SECONDS
    );
    assert_eq!(view.record_count().expect("record_count"), 0u64);
    assert!(view.open().expect("open"));
    assert!(!view.frozen().expect("frozen"));
    assert_eq!(view.revision().expect("revision"), 0u64);
    assert_eq!(view.write_count().expect("write_count"), 0u64);
}

/// A28 read-your-writes. The constructor writes `operatorKey` /
/// `auditorKey` from witnesses and then calls
/// `assertOperatorDistinctFromAuditor(operatorKey)`, whose argument read
/// AND whose own `auditorKey` read must both see the WITNESSED values. If
/// the codegen batches every write into one trailing `OpProgramVerify`,
/// both reads return `JubjubPoint::default()`, the two compare equal, and
/// the distinctness invariant is silently satisfied for every input.
///
/// `initial_state_readback_via_ledger_accessors` proves the writes land,
/// but not their ORDER relative to the call — and byte-parity would
/// happily lock a regenerated read-before-write as the new baseline. So
/// assert the ordering structurally, against the generated source.
#[test]
fn constructor_flushes_writes_before_the_distinctness_assert() {
    const LIB: &str = include_str!("../contracts/asset-registry-fixture/lib.rs");

    let body = common::generated_fn_body(LIB, "initial_state");

    let assert_at = body
        .find("assert_operator_distinct_from_auditor")
        .expect("constructor calls the distinctness assert helper");
    let flush_at = body.find("let qctx = _ctor_flush_").unwrap_or_else(|| {
        panic!(
            "constructor must flush pending writes (`let qctx = _ctor_flush_*`) \
             before the distinctness assert — read-before-write regression (A28)"
        )
    });
    assert!(
        flush_at < assert_at,
        "write-flush (byte {flush_at}) must precede the distinctness assert \
         (byte {assert_at}); the assert would otherwise read the unmodified \
         initial ledger (A28 regression)"
    );

    // The assert's operator-key argument is itself a ledger read; it must
    // also sit after the flush so it sees the witnessed value.
    if let Some(carg_at) = body.find("_carg_") {
        assert!(
            flush_at < carg_at,
            "the assert argument's ledger read (byte {carg_at}) must follow the \
             write-flush (byte {flush_at}) so it reads the written operator key"
        );
    }
}

/// The same composite decode over a SATURATED atom. `setCustodian` writes
/// a caller-supplied 32-byte address, so the read back through
/// `decode_via_field_repr::<ContractAddress>` has to expand a full-width
/// atom — the opposite end of the range from the constructor's all-zero
/// `kernel.self()` value.
#[test]
fn custodian_roundtrips_a_full_width_address() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let expected = nonzero_address();
    let after = contract
        .set_custodian(ctx, expected.clone())
        .expect("set_custodian");
    let state = after.context.current_query_context.state.clone();
    let view = ledger(&state);

    assert_eq!(view.custodian().expect("custodian"), expected);
    // `recordWrite()` bumped both counters and stamped the timestamp.
    assert_eq!(view.write_count().expect("write_count"), 1u64);
    assert_eq!(view.revision().expect("revision"), 1u64);
    assert_eq!(view.updated_at().expect("updated_at"), TIMESTAMP);
    // The constructor-written cells are untouched by the circuit write.
    assert_eq!(
        view.registry_id().expect("registry_id"),
        RegistryAddress::default()
    );
    assert_eq!(view.created_at().expect("created_at"), TIMESTAMP);
}

/// Map-of-struct insert then lookup. `setRecord` inserts an `AssetRecord`
/// (a struct with a nested struct, an enum and a variable-length
/// `Compress` leaf); `assertStoredRecordFresh` reads it back through
/// `decode_via_field_repr::<AssetRecord>` and hands it to the guarded pure
/// circuit — so the composite decode has to invert the composite encode
/// exactly, or the recovered `registeredAt` trips the freshness assert.
#[test]
fn map_of_struct_insert_then_lookup_roundtrips() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let registered_at = TIMESTAMP - 10;
    let after = contract
        .set_record(
            ctx,
            OpaqueString::from("crate-7"),
            record("crate-7", registered_at, 12),
            RecordMutation::Insert,
        )
        .expect("set_record insert");
    let ctx = after.context;

    {
        let view = ledger(&ctx.current_query_context.state);
        assert_eq!(view.record_count().expect("record_count"), 1u64);
    }

    // Reads the value back out of the map: `registeredAt` must survive the
    // roundtrip, or `currentTime - registeredAt <= maxAge` is wrong.
    let after = contract
        .assert_stored_record_fresh(
            ctx,
            OpaqueString::from("crate-7"),
            policy(true, 10),
            TIMESTAMP,
        )
        .expect("stored record must be within the max-age policy");
    let ctx = after.context;

    // One second older than the policy allows: the value really did come
    // back out of the map rather than being defaulted.
    #[allow(clippy::err_expect)]
    let err = contract
        .assert_stored_record_fresh(
            ctx,
            OpaqueString::from("crate-7"),
            policy(true, 9),
            TIMESTAMP,
        )
        .err()
        .expect("a stored record past the policy must trip the guarded assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "record exceeds the max-age policy"
        ),
        "expected the max-age message, got {err:?}"
    );
}

/// A missing key must trip the membership assert rather than decode a
/// default value.
#[test]
fn map_lookup_of_a_missing_key_trips_the_membership_assert() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    #[allow(clippy::err_expect)]
    let err = contract
        .assert_stored_record_fresh(
            ctx,
            OpaqueString::from("absent"),
            policy(false, 0),
            TIMESTAMP,
        )
        .err()
        .expect("looking up an absent key must trip the membership assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "record does not exist"
        ),
        "expected the membership message, got {err:?}"
    );
}

/// The other half of the variable-length-leaf contract. `note` is
/// `Opaque<"string">`, whose `FIELD_SIZE` is 0, so a NON-empty value has
/// no slot in the target type's field-repr. The composite decode must
/// fail LOUDLY on the size check rather than silently mis-slice the
/// remaining leaves (which would hand the freshness assert a garbage
/// `registeredAt` and quietly pass or fail for the wrong reason).
#[test]
fn map_lookup_rejects_a_non_empty_variable_length_leaf() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let mut noted = record("crate-9", TIMESTAMP, 1);
    noted.note = OpaqueString::from("inspected");
    let after = contract
        .set_record(
            ctx,
            OpaqueString::from("crate-9"),
            noted,
            RecordMutation::Insert,
        )
        .expect("set_record insert");

    #[allow(clippy::err_expect)]
    let err = contract
        .assert_stored_record_fresh(
            after.context,
            OpaqueString::from("crate-9"),
            policy(false, 0),
            TIMESTAMP,
        )
        .err()
        .expect("a non-empty variable-length leaf must not decode silently");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m)
                if m.contains("cannot round-trip through from_field_repr")
        ),
        "expected the loud variable-length-leaf error, got {err:?}"
    );
}

/// The map value here NESTS a `ContractAddress`, so the composite decode
/// must walk a `Bytes<32>` leaf inside a struct inside a map value.
#[test]
fn map_of_struct_with_nested_address_roundtrips() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let after = contract
        .set_custody_grant(
            ctx,
            OpaqueString::from("bay-3"),
            grant("bay-3", nonzero_address(), TIMESTAMP - 5),
            RecordMutation::Insert,
        )
        .expect("set_custody_grant insert");
    let ctx = after.context;

    let after = contract
        .assert_grant_effective(ctx, OpaqueString::from("bay-3"), TIMESTAMP)
        .expect("a grant made in the past must be effective");
    let ctx = after.context;

    // `grantedAt` came back out of the nested struct: as of one second
    // BEFORE it was granted the assert must trip.
    #[allow(clippy::err_expect)]
    let err = contract
        .assert_grant_effective(ctx, OpaqueString::from("bay-3"), TIMESTAMP - 6)
        .err()
        .expect("a grant dated after `asOf` must trip");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "grant is not yet effective"
        ),
        "expected the effectivity message, got {err:?}"
    );
}

/// Set insert, membership and removal, driven through the contract's own
/// circuits so the `member` reads execute against real state.
#[test]
fn set_insert_membership_and_removal() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let after = contract
        .set_record(
            ctx,
            OpaqueString::from("crate-7"),
            record("crate-7", TIMESTAMP, 1),
            RecordMutation::Insert,
        )
        .expect("set_record insert");
    let ctx = after.context;

    // Add to the watch list, then adding again must trip on the `member`
    // read — proving the first insert really landed.
    let after = contract
        .set_watch(
            ctx,
            OpaqueString::from("crate-7"),
            compact_contract_asset_registry_fixture::ListMutation::Add,
        )
        .expect("watch add");
    let ctx = after.context;

    #[allow(clippy::err_expect)]
    let err = contract
        .set_watch(
            ctx.clone(),
            OpaqueString::from("crate-7"),
            compact_contract_asset_registry_fixture::ListMutation::Add,
        )
        .err()
        .expect("adding an already-watched record must trip");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "record is already watched"
        ),
        "expected the already-watched message, got {err:?}"
    );

    // A watched record cannot be removed — the `!watchList.member(id)`
    // read has to see the insert.
    #[allow(clippy::err_expect)]
    let err = contract
        .remove_record(ctx.clone(), OpaqueString::from("crate-7"))
        .err()
        .expect("removing a watched record must trip");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "record is still watched"
        ),
        "expected the still-watched message, got {err:?}"
    );

    // Drop the watch, then the removal succeeds and the key lands in the
    // retired set.
    let after = contract
        .set_watch(
            ctx,
            OpaqueString::from("crate-7"),
            compact_contract_asset_registry_fixture::ListMutation::Drop,
        )
        .expect("watch drop");
    contract
        .remove_record(after.context, OpaqueString::from("crate-7"))
        .expect("an unwatched record must be removable");
}

/// G1: the `if`-guarded assert subtracts a TWO-level struct-field
/// projection. `1_000 - 900 = 100 <= 100` pins the boundary as inclusive,
/// so a swapped operand or an off-by-one fails here.
#[test]
fn guarded_max_age_assert_passes_at_the_boundary() {
    pure_circuits::assert_record_fresh_enough(policy(true, 100), record("x", 900, 1), 1_000)
        .expect("an age of exactly maxAge must satisfy the policy");
}

/// One unit past the policy trips it — the assertion that proves the
/// subtraction operand really is `record.provenance.registeredAt`.
#[test]
fn guarded_max_age_assert_trips_when_too_old() {
    let err =
        pure_circuits::assert_record_fresh_enough(policy(true, 100), record("x", 899, 1), 1_000)
            .expect_err("an age of maxAge + 1 must trip the guarded assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "record exceeds the max-age policy"
        ),
        "expected the max-age message, got {err:?}"
    );
}

/// With the guard false the arithmetic is never reached — the whole
/// lifted-temp block must sit inside the `if`.
#[test]
fn guarded_max_age_assert_is_skipped_when_the_guard_is_false() {
    pure_circuits::assert_record_fresh_enough(policy(false, 1), record("x", 0, 1), 1_000_000)
        .expect("a false guard must skip the max-age assert entirely");
}

/// The typer's underflow guard fires before the subtraction wraps: on
/// `u64`, `900 - 1_000` would wrap to a huge value and silently satisfy
/// `<= maxAge`.
#[test]
fn subtraction_underflow_is_trapped_not_wrapped() {
    let err = pure_circuits::assert_record_fresh_enough(
        policy(true, u64::MAX),
        record("x", 1_000, 1),
        900,
    )
    .expect_err("a registration time in the future must trap, not wrap");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m)
                if m == "registration time cannot be in the future"
        ),
        "expected the ordering guard to fire, got {err:?}"
    );
}

/// Projections on BOTH operands of `-`, so two lifted temps coexist in one
/// body. Checking the returned VALUE proves the uniquifier kept them
/// apart: shared names would shadow and the gap would compute 0.
#[test]
fn two_projection_operands_compute_the_real_difference() {
    let gap = pure_circuits::registration_gap(record("a", 1_000, 1), record("b", 900, 1))
        .expect("newer >= older must pass the ordering assert");
    assert_eq!(
        gap, 100,
        "gap must be newer.registeredAt - older.registeredAt"
    );

    let err = pure_circuits::registration_gap(record("a", 900, 1), record("b", 1_000, 1))
        .expect_err("newer < older must trip the ordering assert");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m)
                if m == "newer record must not predate the older one"
        ),
        "expected the ordering message, got {err:?}"
    );
}

/// An impure circuit calling the guarded pure one: the failure has to
/// propagate out through the `?` the codegen appends at the call site,
/// and the ledger write must happen only on the success path.
#[test]
fn impure_caller_propagates_the_guarded_assert_failure() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(
        init.current_contract_state.clone(),
        init.current_private_state,
    );

    #[allow(clippy::err_expect)]
    let err = contract
        .accept_if_fresh(ctx, policy(true, 100), record("x", 899, 1), 1_000)
        .err()
        .expect("a failing guarded assert must propagate out of the impure circuit");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "record exceeds the max-age policy"
        ),
        "expected the propagated max-age message, got {err:?}"
    );

    let ok_ctx = CircuitContext::new(init.current_contract_state, ());
    let after = contract
        .accept_if_fresh(ok_ctx, policy(true, 100), record("x", 900, 1), 1_000)
        .expect("an age within the policy must commit the ledger write");
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.write_count().expect("write_count"), 1u64);
}

/// The enum guard: `AssetClass::Unspecified` is rejected, any other
/// variant accepted. Pins that the enum discriminant survives the
/// Compact -> Rust lowering.
#[test]
fn unspecified_asset_class_is_rejected() {
    pure_circuits::assert_record_class_known(record("x", 0, 1))
        .expect("a specified class must be accepted");

    let mut unspecified = record("x", 0, 1);
    unspecified.kind = AssetClass::Unspecified;
    let err = pure_circuits::assert_record_class_known(unspecified)
        .expect_err("an unspecified class must be rejected");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "asset class must be specified"
        ),
        "expected the asset-class message, got {err:?}"
    );
}
