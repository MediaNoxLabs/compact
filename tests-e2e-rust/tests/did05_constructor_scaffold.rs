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
// A29/A30 did.compact 0.5.0 constructor scaffold tests.
//
// did.compact 0.5.0 declares 19 ledger fields, so the front end chunks the
// on-chain state (StateValue::Array caps at 16 slots) into an outer 2-slot
// array: fields 0..4 in `[0]`, fields 4..19 in `[1]`. Before A29 the
// `initial_state` scaffold flattened that IR into a FLAT 19-element
// `new_array` while every read/write site used the nested chunked paths —
// executing the constructor produced state the generated `ledger()`
// accessors could not read. The generic executing gate for A29 lives in
// tests/chunked_ledger_fixture.rs; this file carries the did-05-specific
// gate:
//
// `did05_initial_state_readback_via_ledger_accessors` — the full executing
// readback, including `id`. The constructor does `id = kernel.self()`,
// whose generated read decodes the address cell via
// `decode_via_field_repr::<ContractAddress>`. Until A30 that decode
// converted AlignedValue atoms to Frs 1:1 while cells are
// ALIGNMENT-encoded (`[u8; 32]::FIELD_SIZE == 2` Frs, but the cell yields
// one 32-byte atom -> 1 Fr), so `initial_state` could never complete
// (tracked in the MediaNoxLabs/compact#3 "second decode-path finding"
// comment thread). A30 makes the decode alignment-aware; this test now
// runs un-ignored and also asserts the `ledger().id()` readback. (The
// pre-A30 pin test `did05_initial_state_blocked_only_by_kernel_self_decode`
// asserted the then-current decode failure and was removed with A30.)

use compact_contract_did_05::{
    ledger, Contract, ContractAddress as DidContractAddress, Ledger, Witnesses,
};
use compact_runtime::*;

/// Deterministic stub witnesses. The constructor invokes
/// `local_controller_public_key`, `local_recovery_authority_public_key` and
/// `current_timestamp`; the two keys must be DISTINCT valid curve points or
/// the constructor's own
/// `assertControllerPublicKeyDistinctFromRecoveryAuthority` fails.
/// `get_schnorr_reduction` is only used by signature-checking circuits, not
/// the constructor.
struct StubWitnesses;

const TIMESTAMP: u64 = 1_700_000_000;

impl Witnesses<()> for StubWitnesses {
    fn get_schnorr_reduction<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
        _challenge_hash: Fr,
    ) -> ((), (u8, u128)) {
        ((), (0u8, 0u128))
    }

    fn local_controller_public_key<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
    ) -> ((), JubjubPoint) {
        ((), hash_to_curve(Fr::from(1u64)))
    }

    fn local_recovery_authority_public_key<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
    ) -> ((), JubjubPoint) {
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

/// Full executing readback: run initial_state with stub witnesses, read the
/// fields back through the generated ledger() accessors — including `id`,
/// which the constructor assigns from `kernel.self()` and whose read goes
/// through the A30 alignment-aware `decode_via_field_repr::<ContractAddress>`.
#[test]
fn did05_initial_state_readback_via_ledger_accessors() {
    let contract: Contract<(), StubWitnesses> = Contract::new(StubWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let state = result.current_contract_state;
    let view = ledger(&state);

    // Chunk [0]: constructor-assigned scalars (global field indices 0..4).
    assert_eq!(view.contract_version().expect("contract_version"), 2u32);
    // `id = kernel.self()`: the constructor context queries against the
    // default (all-zero) contract address, so that is what the cell must
    // hold — decoded into the CONTRACT's `ContractAddress { bytes }`
    // struct. The all-zero address is also the empty-normalised-atom edge
    // case of the A30 decode (32-byte leaf whose atom normalises empty).
    assert_eq!(view.id().expect("id"), DidContractAddress::default());
    assert_eq!(
        view.controller_public_key().expect("controller_public_key"),
        hash_to_curve(Fr::from(1u64)),
    );
    assert_eq!(
        view.recovery_authority_public_key()
            .expect("recovery_authority_public_key"),
        hash_to_curve(Fr::from(2u64)),
    );

    // Chunk [1]: scalars at global indices >= 4 — these are the reads that
    // could not work against the pre-A29 flat scaffold.
    assert_eq!(view.version().expect("version"), 0u64);
    assert_eq!(view.created().expect("created"), TIMESTAMP);
    assert_eq!(view.updated().expect("updated"), TIMESTAMP);
    assert!(!view.deactivated().expect("deactivated"));
    assert!(view.active().expect("active"));
    assert_eq!(view.operation_count().expect("operation_count"), 0u64);
}
