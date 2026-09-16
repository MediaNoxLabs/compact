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
// CPT-006 / compact#83: tuple_fixture.compact byte-parity + direct-invocation
// gate for tuple-typed values.
//
// The report: `compactc --target rust` declared a tuple return as a Rust
// tuple `(Fr, Fr)` but constructed the value as an array `[x, x]`, so the
// emitted crate failed with E0308 for every circuit returning a tuple type.
// This test can only exist once the crate compiles, which is the first
// half of the gate; the second half is that each value is the Rust tuple
// (or array) its declared type promises — with the trailing comma for
// arity 1 and `.0`/`.1` indexing through the coercion temp — checked by
// comparing against literal Rust values, so a future regression to the
// array spelling fails to build rather than passes quietly.
//
// Assertions:
//   1. `initial_state()` bytes match the TS reference (one Boolean ledger
//      slot, one operations-map entry for `ping`; the pure circuits add
//      nothing to the on-chain shape).
//   2. Each pure circuit returns the tuple / array value its type declares.

use compact_contract_tuple_fixture::Contract;
use midnight_compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use tests_e2e_rust::SmallFixtureTsReference;

fn fixture() -> SmallFixtureTsReference {
    SmallFixtureTsReference::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/tuple-fixture-ts-state.json"
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

fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None, None),
    );
    ContractState {
        data,
        operations,
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    }
}

#[test]
fn tuple_fixture_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");

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

#[test]
fn tuple_fixture_vector_return_stays_an_array() {
    // The control: a `Vector<2, Field>` return is `[Fr; 2]`, as before.
    use compact_contract_tuple_fixture::pure_circuits::as_vector;
    let x = Fr::from(7u64);
    assert_eq!(as_vector(x).unwrap(), [x, x]);
}

#[test]
fn tuple_fixture_homogeneous_tuple_return_is_a_tuple() {
    use compact_contract_tuple_fixture::pure_circuits::as_tuple;
    let x = Fr::from(7u64);
    assert_eq!(as_tuple(x).unwrap(), (x, x));
}

#[test]
fn tuple_fixture_heterogeneous_tuple_coerces_each_element() {
    // `[Field, Uint<16>]` from a `Uint<8>`: the first element widens to
    // the field, the second to u16. A tuple is the only Rust aggregate
    // that can hold both.
    use compact_contract_tuple_fixture::pure_circuits::hetero;
    assert_eq!(hetero(3u8).unwrap(), (Fr::from(3u64), 3u16));
}

#[test]
fn tuple_fixture_one_tuple_has_its_trailing_comma() {
    use compact_contract_tuple_fixture::pure_circuits::one_tuple;
    let x = Fr::from(11u64);
    assert_eq!(one_tuple(x).unwrap(), (x,));
}

#[test]
fn tuple_fixture_empty_tuple_is_unit() {
    use compact_contract_tuple_fixture::pure_circuits::empty_tuple;
    #[allow(clippy::unit_cmp)]
    {
        assert_eq!(empty_tuple().unwrap(), ());
    }
}

#[test]
fn tuple_fixture_coerced_literal_is_a_tuple() {
    // Every element coerced Uint<8> -> Field through the literal path.
    use compact_contract_tuple_fixture::pure_circuits::tuple_coerce;
    assert_eq!(tuple_coerce(5u8).unwrap(), (Fr::from(5u64), Fr::from(5u64)));
}

#[test]
fn tuple_fixture_var_ref_coerces_through_a_temp_with_dot_indexing() {
    // A declared tuple const returned at a wider tuple type takes the
    // temp-binding route: `{ let t = w; (Fr::from(t.0 as u64), ...) }`.
    // The report showed `t[0]` there, which does not compile on a tuple.
    use compact_contract_tuple_fixture::pure_circuits::tuple_var_ref;
    assert_eq!(
        tuple_var_ref(9u8).unwrap(),
        (Fr::from(9u64), Fr::from(9u64))
    );
}

#[test]
fn tuple_fixture_struct_vector_returned_as_a_tuple_moves_without_cloning() {
    // F-031 (PR #87 review): a `Vector<2, Pair>` const returned at a
    // `[Pair, Pair]` type. `Pair` derives `Clone`, not `Copy`, so the earlier
    // `(__compact_materialize[0], __compact_materialize[1])` was E0508 —
    // this test exists only because the crate now compiles — and the value
    // must be the two structs, in order, moved out through an array pattern
    // rather than copied.
    use compact_contract_tuple_fixture::pure_circuits::struct_vector_return;
    use compact_contract_tuple_fixture::Pair;
    let x = Fr::from(17u64);
    assert_eq!(
        struct_vector_return(x).unwrap(),
        (Pair { a: x, b: 1u8 }, Pair { a: x, b: 2u8 })
    );
}

#[test]
fn tuple_fixture_struct_vector_assigned_to_a_tuple_const_is_bridged() {
    // The second path: `const t: [Pair, Pair] = v;`. The assignment renderer
    // used to emit `let t = v;` with no kind bridge at all (E0308, found while
    // pinning F-031); it now renders the RHS at the const's declared type.
    use compact_contract_tuple_fixture::pure_circuits::struct_vector_to_tuple;
    use compact_contract_tuple_fixture::Pair;
    let x = Fr::from(19u64);
    assert_eq!(
        struct_vector_to_tuple(x).unwrap(),
        (Pair { a: x, b: 1u8 }, Pair { a: x, b: 2u8 })
    );
}

#[test]
fn tuple_fixture_tuple_const_flows_into_a_vector_position() {
    // The kind conversion in the other direction: a tuple-typed const
    // returned where a `Vector<2, Field>` is declared becomes `[t.0, t.1]`.
    use compact_contract_tuple_fixture::pure_circuits::tuple_to_vector;
    let x = Fr::from(13u64);
    assert_eq!(tuple_to_vector(x).unwrap(), [x, x]);
}
