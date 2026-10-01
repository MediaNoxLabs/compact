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

use compact_rust_assert_parity_oracle_fixture::ledger_contract::{
    initial_state, trigger_fail, trigger_ok,
};
use compact_rust_assert_parity_oracle_fixture::pure_circuits::require_true;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["trigger_ok", "trigger_fail", "ping"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_assertion_oracle_returns_errors_without_panicking() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-parity-oracle.json"
    ))
    .unwrap();
    assert_eq!(require_true(true).unwrap(), oracle["pureTrue"]["ok"]);
    let pure_error = require_true(false).unwrap_err();
    assert!(matches!(
        pure_error,
        runtime::CompactError::AssertionFailed(_)
    ));
    assert_eq!(pure_error.to_string(), oracle["pureFalse"]["error"]);
    assert_eq!(oracle["pureFalse"]["compactError"], true);

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let ok = trigger_ok(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(ok.context.query.state.get_ref().clone()),
        oracle["afterTriggerOk"]
    );
    let flag =
        runtime::ledger::read_root_cell::<bool, _>(ok.context.query.state.get_ref(), 0).unwrap();
    assert_eq!(flag, oracle["flagAfterTriggerOk"]);
    let error = trigger_fail(ok.context)
        .err()
        .expect("assertion should fail");
    assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
    assert_eq!(error.to_string(), oracle["triggerFail"]["error"]);
    assert_eq!(oracle["triggerFail"]["compactError"], true);
    assert_eq!(oracle["afterTriggerFail"], oracle["afterTriggerOk"]);
}
