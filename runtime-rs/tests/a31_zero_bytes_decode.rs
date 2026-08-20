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
// A31 regression gate (MediaNoxLabs/compact#15).
//
// An all-zero `Bytes<N>` value normalises to a ZERO-byte atom (upstream
// `ValueAtom::normalize` strips trailing zeros), while its `Bytes{n}`
// alignment still demands `ceil(n/31)` field-repr `Fr`s
// (`[u8; 32]::FIELD_SIZE == 2`). `decode_via_field_repr` must derive the
// `Fr` count from the ALIGNMENT SEGMENT KIND — `Bytes{n}` always expands
// to `ceil(n/31)` `Fr`s, zero-padding whatever normalisation stripped;
// only `Compress` (variable-length) leaves take their count from the
// atom's own byte length (empty atom → zero `Fr`s) — never from atom
// emptiness. A decoder that shortcuts an empty atom to zero `Fr`s decodes
// every non-zero address while failing exactly on the all-zero one.
//
// These tests pin the value across every read shape issue #15 names:
//
// 1. a locally built cell (`new_cell`), decoded directly — the LOCAL
//    reproduction;
// 2. the same value carried through a `tagged_serialize` /
//    `tagged_deserialize` round-trip of a full `ContractState` — the
//    shape every indexer-fed reader sees (the CHAIN reproduction);
// 3. the VM read path (`query_for_read` + `popeq`) that generated
//    `ledger().<field>()` accessors use, fed from the DESERIALISED
//    state.
//
// The all-zero multi-leaf struct shape lives with the decoder's unit
// tests (`std_lib::adts::tests::
// via_field_repr_roundtrips_all_zero_bytes32_bearing_struct`), next to
// the hand-built `alignment=b32, atoms=[-]` shape
// (`via_field_repr_decodes_normalized_empty_bytes32_atom`).

use compact_runtime::std_lib::decode_via_field_repr;
use compact_runtime::*;
use midnight_serialize::{tagged_deserialize, tagged_serialize};
use midnight_storage::storage::HashMap;

/// Clone the `AlignedValue` stored in a `StateValue::Cell`.
fn cell_contents(sv: &StateValue<DefaultDB>) -> AlignedValue {
    match sv {
        StateValue::Cell(c) => (**c).clone(),
        other => panic!("expected StateValue::Cell, got {other:?}"),
    }
}

/// Assert the A31 premise: the stored atom really is the
/// normalise-stripped EMPTY atom. If an upstream encoding change ever
/// stops producing this shape, the tests below would silently stop
/// covering A31 — fail loudly instead.
fn assert_normalized_empty(label: &str, av: &AlignedValue) {
    assert_eq!(
        av.value.0,
        vec![ValueAtom(vec![])],
        "[{label}] expected the all-zero Bytes<32> value to normalise to a \
         single empty atom"
    );
}

/// A `ContractState` whose data array holds one cell: an all-zero
/// `ContractAddress` (`Bytes<32>`), as a generated `initial_state`
/// would store it.
fn all_zero_address_state() -> ContractState<DefaultDB> {
    let cell = new_cell::<DefaultDB, _>(ContractAddress::default());
    let data = ChargedState::new(new_array::<DefaultDB>(vec![cell]));
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"noop".to_vec()),
        ContractOperation::new(None),
    );
    ContractState {
        data,
        operations,
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    }
}

/// Shape 1 (LOCAL): `new_cell(all-zero ContractAddress)` decoded
/// directly.
#[test]
fn all_zero_contract_address_cell_decodes() {
    let sv = new_cell::<DefaultDB, _>(ContractAddress::default());
    let av = cell_contents(&sv);
    assert_normalized_empty("new_cell", &av);
    assert_eq!(
        decode_via_field_repr::<ContractAddress>(&av).expect("all-zero address decode"),
        ContractAddress::default()
    );
}

/// Shape 2 (CHAIN): the all-zero cell carried through
/// `tagged_serialize`/`tagged_deserialize` of a full `ContractState` —
/// the byte stream an indexer hands back — then decoded from the
/// deserialised state.
#[test]
fn all_zero_contract_address_survives_contract_state_round_trip() {
    let state = all_zero_address_state();

    let mut buf = Vec::new();
    tagged_serialize(&state, &mut buf).expect("tagged_serialize");
    let state2: ContractState<DefaultDB> =
        tagged_deserialize(&mut &buf[..]).expect("tagged_deserialize");

    let cell = match state2.data.get_ref() {
        StateValue::Array(arr) => arr.get(0).expect("data[0]"),
        other => panic!("expected StateValue::Array, got {other:?}"),
    };
    let av = cell_contents(cell);
    assert_normalized_empty("round-trip", &av);
    assert_eq!(
        decode_via_field_repr::<ContractAddress>(&av).expect("all-zero address decode"),
        ContractAddress::default()
    );
}

/// Shape 3 (accessor): the VM read program generated `ledger().<field>()`
/// accessors run — `dup` / `idx` / `popeq` under `ResultModeGather` —
/// over the DESERIALISED state, decoding the `GatherEvent::Read` payload.
#[test]
fn all_zero_contract_address_decodes_via_vm_read() {
    let state = all_zero_address_state();
    let mut buf = Vec::new();
    tagged_serialize(&state, &mut buf).expect("tagged_serialize");
    let state2: ContractState<DefaultDB> =
        tagged_deserialize(&mut &buf[..]).expect("tagged_deserialize");

    let qctx = QueryContext::new(state2.data.clone(), ContractAddress::default());
    let ops = OpProgramGather::<DefaultDB>::new()
        .dup(0)
        .idx_at_index(0u8, false)
        .popeq(true)
        .build();
    let results = query_for_read(&qctx, &ops, None, &initial_cost_model()).expect("query");
    let av = match results.events.last() {
        Some(onchain_vm::result_mode::GatherEvent::Read(av)) => av.clone(),
        other => panic!("expected a GatherEvent::Read, got {other:?}"),
    };
    assert_normalized_empty("vm read", &av);
    assert_eq!(
        decode_via_field_repr::<ContractAddress>(&av).expect("all-zero address decode"),
        ContractAddress::default()
    );
}
