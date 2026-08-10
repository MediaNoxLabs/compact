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
// A29 did.compact 0.5.0 constructor scaffold tests.
//
// did.compact 0.5.0 declares 19 ledger fields, so the front end chunks the
// on-chain state (StateValue::Array caps at 16 slots) into an outer 2-slot
// array: fields 0..4 in `[0]`, fields 4..19 in `[1]`. Before A29 the
// `initial_state` scaffold flattened that IR into a FLAT 19-element
// `new_array` while every read/write site used the nested chunked paths —
// executing the constructor produced state the generated `ledger()`
// accessors could not read. The generic executing gate for A29 lives in
// tests/chunked_ledger_fixture.rs (a fixture whose constructor can run to
// completion today); this file carries the did-05-specific gates:
//
// 1. `did05_initial_state_readback_via_ledger_accessors` — the full
//    executing readback. Currently #[ignore]d: did-05's constructor does
//    `id = kernel.self()`, whose generated read decodes the address cell via
//    `decode_via_field_repr::<ContractAddress>` — the KNOWN-BROKEN
//    field-repr-vs-alignment decode (`[u8; 32]::FIELD_SIZE == 2` Frs, but
//    the cell yields one 32-byte atom -> 1 Fr), tracked as a separate
//    follow-up in the MediaNoxLabs/compact#3 comment thread alongside A29.
//    Un-ignore when that decode fix lands.
//
// 2. `did05_initial_state_blocked_only_by_kernel_self_decode` — pins the
//    CURRENT failure mode: initial_state must fail with exactly the known
//    kernel.self() decode error, nothing else. Before A29 this call could
//    only die on the flat-vs-nested shape mismatch (or "succeed" into
//    unreadable state); if the decode follow-up lands, this test starts
//    failing on purpose — flip test 1 on and delete this one.

use compact_contract_did_05::{ledger, Contract, Ledger, Witnesses};
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
/// scalar fields back through the generated ledger() accessors. `id()` is
/// deliberately not asserted — same decode follow-up as the ignore reason.
#[test]
#[ignore = "blocked on kernel.self() ContractAddress decode (field-repr vs alignment, \
            see MediaNoxLabs/compact#3 comment thread); un-ignore when the decode \
            follow-up lands"]
fn did05_initial_state_readback_via_ledger_accessors() {
    let contract: Contract<(), StubWitnesses> = Contract::new(StubWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let state = result.current_contract_state;
    let view = ledger(&state);

    // Chunk [0]: constructor-assigned scalars (global field indices 0..4).
    assert_eq!(view.contract_version().expect("contract_version"), 2u32);
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

/// Pin the current, known-and-tracked failure mode of executing the did-05
/// constructor: the `id = kernel.self()` read's field-repr decode. Anything
/// else (e.g. a VM error from a scaffold/write shape mismatch — the A29
/// symptom) fails this test.
#[test]
fn did05_initial_state_blocked_only_by_kernel_self_decode() {
    let contract: Contract<(), StubWitnesses> = Contract::new(StubWitnesses);
    match contract.initial_state(ctor_ctx()) {
        Ok(_) => panic!(
            "initial_state unexpectedly succeeded: the kernel.self() decode \
             follow-up has landed — un-ignore \
             did05_initial_state_readback_via_ledger_accessors and delete \
             this pin test"
        ),
        Err(e) => {
            let msg = format!("{e:?}");
            assert!(
                msg.contains("decode_via_field_repr"),
                "initial_state failed, but NOT with the known kernel.self() \
                 field-repr decode error — possible scaffold/write shape \
                 regression (A29): {msg}"
            );
        }
    }
}
