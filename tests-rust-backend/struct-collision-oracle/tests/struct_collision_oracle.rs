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

use compact_rust_struct_collision_oracle_fixture::ledger_contract::{
    initial_state, runAlpha, runBeta,
};
use compact_rust_struct_collision_oracle_fixture::pure_circuits::{runWrapAlpha, runWrapBeta};
use compact_rust_struct_collision_oracle_fixture::types::{
    Inner, InnerCompact1, Rec, RecCompact1, Wrap, WrapCompact1,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["runAlpha", "runBeta"] {
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
fn distinct_struct_layouts_and_nested_fields_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/struct-collision-oracle.json"
    ))
    .unwrap();
    let alpha = RecCompact1 {
        alpha: Field::from(5_u64),
    };
    let beta = Rec {
        beta: Field::from(7_u64),
    };
    assert_eq!(alpha.alpha, Field::from(5_u64));
    assert_eq!(beta.beta, Field::from(7_u64));
    let wrapped_alpha = WrapCompact1 {
        inner: InnerCompact1 {
            a: Field::from(11_u64),
        },
    };
    let wrapped_beta = Wrap {
        inner: Inner { b: true },
    };
    assert_eq!(wrapped_alpha.inner.a, Field::from(11_u64));
    assert!(wrapped_beta.inner.b);
    assert_eq!(
        runWrapAlpha(Field::from(11_u64)).unwrap(),
        Field::from(
            oracle["wrapAlpha"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
    assert_eq!(runWrapBeta(true).unwrap(), oracle["wrapBeta"]);

    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let context = constructor.into_circuit_context(ContractAddress::default());
    let alpha_result = runAlpha(context, Field::from(5_u64)).unwrap();
    let beta_result = runBeta(alpha_result.context, Field::from(7_u64)).unwrap();
    assert_eq!(
        state_hex(beta_result.context.query.state.get_ref().clone()),
        oracle["afterWritesHex"]
    );
}
