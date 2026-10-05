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

use compact_rust_literal_coercion_oracle_fixture::ledger_contract::initial_state;
use compact_rust_literal_coercion_oracle_fixture::pure_circuits::{
    callArgFieldOnlyLiteral, callArgMaxUnsignedPlusOne, hashDefaultVectorArg,
    hashNestedUintVarRefElem, hashSameTypeNestedArg, hashUintVarRefElem, retFieldOnlyLiteral,
    subgroupCheck, vectorEltWise,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{DefaultDB, StateValue};
use midnight_compact_runtime::{
    BoundedUint, Field, FixedVector, JubjubPoint, ec_mul_generator, persistent_hash,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn oracle_constructor_and_selected_pure_circuits_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/literal-coercion-oracle.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let vector = FixedVector::new([Field::from(5_u64), Field::from(0_u64)]);
    assert_eq!(
        hex::encode(hashSameTypeNestedArg(vector).unwrap().into_array()),
        oracle["sameTypeNestedHashHex"]
    );
    assert_eq!(
        hex::encode(hashDefaultVectorArg().unwrap().into_array()),
        oracle["defaultVectorHashHex"]
    );
    assert_eq!(
        vectorEltWise(BoundedUint::<4294967295>::new(7).unwrap()).unwrap(),
        FixedVector::new([Field::from(7_u64); 2])
    );
    assert_eq!(oracle["vectorEltWise"], serde_json::json!(["7", "7"]));
    assert_eq!(
        callArgFieldOnlyLiteral().unwrap(),
        retFieldOnlyLiteral().unwrap()
    );
    assert_eq!(
        oracle["fieldOnly"],
        "819310549611346726241370945440405716213240158234039660170669895299022906775"
    );
}

fn point_json(point: JubjubPoint) -> serde_json::Value {
    serde_json::json!({
        "x": hex::encode(point.x().unwrap().as_le_bytes()),
        "y": hex::encode(point.y().unwrap().as_le_bytes()),
    })
}

#[test]
fn pure_literal_boundary_and_cryptographic_coercions_match_corrected_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/literal-coercion-boundaries.json"
    ))
    .unwrap();

    let mut boundary_bytes = [0_u8; 32];
    boundary_bytes[31] = 1; // 2^248, the first Field-only literal.
    let boundary = Field::from_le_bytes(&boundary_bytes).unwrap();
    assert_eq!(callArgMaxUnsignedPlusOne().unwrap(), boundary);
    assert_eq!(oracle["boundary"]["fieldHex"], hex::encode(boundary_bytes));
    assert_eq!(
        oracle["boundary"]["decimal"],
        "452312848583266388373324160190187140051835877600158453279131187530910662656"
    );

    for row in oracle["hashes"].as_array().unwrap() {
        let input = row["input"].as_str().unwrap().parse::<u8>().unwrap();
        let typed = BoundedUint::<255>::new(u128::from(input)).unwrap();
        let flat = hashUintVarRefElem(typed).unwrap();
        let nested = hashNestedUintVarRefElem(typed).unwrap();
        let fields = FixedVector::new([Field::from(u64::from(input)), Field::from(0_u64)]);
        let flat_reference = persistent_hash(fields.clone());
        let nested_reference = persistent_hash(FixedVector::new([fields]));
        let byte_reference = persistent_hash(FixedVector::new([
            typed,
            BoundedUint::<255>::new(0).unwrap(),
        ]));
        assert_eq!(hex::encode(flat.into_array()), row["flat"], "flat {input}");
        assert_eq!(
            hex::encode(nested.into_array()),
            row["nested"],
            "nested {input}"
        );
        assert_eq!(
            hex::encode(flat_reference.into_array()),
            row["flatReference"]
        );
        assert_eq!(
            hex::encode(nested_reference.into_array()),
            row["nestedReference"]
        );
        assert_eq!(hex::encode(byte_reference.into_array()), row["byteAligned"]);
        assert_ne!(row["flat"], row["byteAligned"], "alignment {input}");
    }

    let subgroup = subgroupCheck().unwrap();
    let generator = ec_mul_generator(Field::from(1_u64)).unwrap();
    assert_eq!(subgroup, generator);
    assert_eq!(point_json(subgroup), oracle["subgroup"]);
    assert_eq!(point_json(generator), oracle["generator"]);
}
