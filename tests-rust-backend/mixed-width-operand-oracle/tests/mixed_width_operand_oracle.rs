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

use compact_rust_mixed_width_operand_oracle_fixture::ledger_contract::{
    initial_state, recordMatching, recordPinned,
};
use compact_rust_mixed_width_operand_oracle_fixture::pure_circuits::{
    assertProductEQ, assertProductGE, assertProductGT, assertProductLE, assertProductLT,
    assertProductNE, guardedDiff, productMixed, sumMixed,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_counter};
use midnight_compact_runtime::{BoundedUint, CompactError};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn uint32(value: u128) -> BoundedUint<4294967295> {
    BoundedUint::new(value).unwrap()
}

type Comparison = fn(BoundedUint<4294967295>, BoundedUint<4294967295>) -> Result<(), CompactError>;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["recordPinned", "recordMatching"] {
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

fn outcome<T: ToString>(result: Result<T, CompactError>) -> serde_json::Value {
    match result {
        Ok(value) => serde_json::json!({ "ok": true, "value": value.to_string() }),
        Err(error) => serde_json::json!({ "ok": false, "message": error.to_string() }),
    }
}

#[test]
fn mixed_width_oracle_matches_typescript_at_boundaries() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/mixed-width-operand-oracle.json"
    ))
    .unwrap();

    let constructor = initial_state(ConstructorContext::new(()), uint32(20), uint32(4)).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    assert_eq!(
        outcome(initial_state(ConstructorContext::new(()), uint32(1), uint32(1)).map(|_| "")),
        oracle["constructorUnderflow"]
    );

    let comparisons: [(&str, Comparison); 6] = [
        ("LE", assertProductLE),
        ("LT", assertProductLT),
        ("GT", assertProductGT),
        ("GE", assertProductGE),
        ("EQ", assertProductEQ),
        ("NE", assertProductNE),
    ];
    for (operator, compare) in comparisons {
        for (index, (q, y)) in [
            (1_073_741_823, 4_294_967_292),
            (1_073_741_824, 4_294_967_295),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                outcome(compare(uint32(q), uint32(y)).map(|_| "")),
                oracle["comparisons"][operator][index],
                "{operator} case {index}"
            );
        }
    }
    assert_eq!(
        outcome(
            sumMixed(uint32(4_294_967_295), BoundedUint::<255>::new(255).unwrap())
                .map(|value| value.value().to_string())
        ),
        oracle["sumMixed"]
    );
    assert_eq!(
        outcome(
            productMixed(uint32(4_294_967_295), BoundedUint::<255>::new(255).unwrap())
                .map(|value| value.value().to_string())
        ),
        oracle["productMixed"]
    );
    assert_eq!(
        outcome(guardedDiff(uint32(20), uint32(4)).map(|value| value.value().to_string())),
        oracle["guardedDiff"][0]
    );
    assert_eq!(
        outcome(guardedDiff(uint32(1), uint32(1)).map(|value| value.value().to_string())),
        oracle["guardedDiff"][1]
    );

    let context = constructor.into_circuit_context(ContractAddress::default());
    let pinned = recordPinned(context, uint32(4), uint32(20)).unwrap();
    let matched = recordMatching(
        pinned.context,
        BoundedUint::<255>::new(7).unwrap(),
        uint32(7),
    )
    .unwrap();
    let state = matched.context.query.state.get_ref().clone();
    let StateValue::Array(fields) = &state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(1).unwrap()).unwrap(), 2);
    assert_eq!(state_hex(state), oracle["stateAfterActionsHex"]);
    assert_eq!(
        outcome(
            recordPinned(
                matched.context,
                uint32(1_073_741_824),
                uint32(4_294_967_295)
            )
            .map(|_| "")
        ),
        oracle["recordPinnedFailure"]
    );
    let fresh_context = initial_state(ConstructorContext::new(()), uint32(20), uint32(4))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    assert_eq!(
        outcome(
            recordMatching(
                fresh_context,
                BoundedUint::<255>::new(7).unwrap(),
                uint32(8),
            )
            .map(|_| "")
        ),
        oracle["recordMatchingFailure"]
    );
}
