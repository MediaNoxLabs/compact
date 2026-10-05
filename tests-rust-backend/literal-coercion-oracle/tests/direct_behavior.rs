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
//! Direct exported calls, independently captured from unchanged TypeScript.
use compact_rust_literal_coercion_oracle_fixture::{pure_circuits as p, types};
use midnight_compact_runtime::{
    BoundedUint, Field, FixedBytes, FixedVector, JubjubPoint, ec_mul_generator,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn field(value: &Value) -> Field {
    value
        .as_str()
        .unwrap()
        .bytes()
        .fold(Field::from(0u64), |n, digit| {
            assert!(digit.is_ascii_digit());
            n * Field::from(10u64) + Field::from(u64::from(digit - b'0'))
        })
}
fn unsigned<const MAX: u128>(value: &Value) -> BoundedUint<MAX> {
    BoundedUint::new(value.as_str().unwrap().parse().unwrap()).unwrap()
}
fn field_json(value: Field) -> Value {
    json!(hex::encode(value.as_le_bytes()))
}
fn bytes_json(value: FixedBytes<32>) -> Value {
    json!(hex::encode(value.0))
}
fn vector_json<const N: usize>(value: FixedVector<Field, N>) -> Value {
    Value::Array(value.0.into_iter().map(field_json).collect())
}
fn point_json(value: JubjubPoint) -> Value {
    json!({"x":field_json(value.x().unwrap()),"y":field_json(value.y().unwrap())})
}
fn vector(value: &Value) -> FixedVector<Field, 2> {
    FixedVector::new([field(&value[0]), field(&value[1])])
}
#[test]
fn every_literal_export_has_an_independent_direct_typescript_result() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/oracle-direct-behavior.json"
    ))
    .unwrap();
    let expected_exports: BTreeSet<&str> = [
        "addFieldLiteral",
        "addFieldOnlyLiteral",
        "addHugeFieldLiteral",
        "addUintFieldOperand",
        "callArgFieldOnlyLiteral",
        "callArgHugeFieldLiteral",
        "callArgMaxUnsignedPlusOne",
        "cmpFieldOnlyLiteral",
        "cmpHugeFieldLiteral",
        "constFieldOnlyLiteral",
        "constHugeFieldLiteral",
        "constThenCall",
        "hashCallVectorArg",
        "hashConstVectorArg",
        "hashDefaultVectorArg",
        "hashNestedUintVarRefElem",
        "hashNestedVarRefVectorArg",
        "hashSameTypeNestedArg",
        "hashSameTypeVectorArg",
        "hashStructFieldVectorArg",
        "hashUintVarRefElem",
        "hashVarRefVectorArg",
        "hashZeroLiteral",
        "idf",
        "litLhsFieldLiteral",
        "mulHugeFieldLiteral",
        "nativeArgFieldOnlyLiteral",
        "nativeArgLiteral",
        "nestedFieldArith",
        "retFieldLiteral",
        "retFieldOnlyLiteral",
        "retHugeFieldLiteral",
        "retU128FieldLiteral",
        "sameTypeVec",
        "someFieldLiteral",
        "structLiteral",
        "structMemberHugeFieldLiteral",
        "subFieldLiteral",
        "subgroupCheck",
        "u128Rung",
        "uintToField",
        "vectorEltHugeFieldLiteral",
        "vectorEltWise",
    ]
    .into_iter()
    .collect();
    let captured_exports: BTreeSet<&str> = oracle["literal"]["exports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(captured_exports, expected_exports);
    assert_eq!(expected_exports.len(), 43);
    assert_eq!(oracle["literal"]["cases"].as_array().unwrap().len(), 91);
    let mut executed = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for row in oracle["literal"]["cases"].as_array().unwrap() {
        let name = row["export"].as_str().unwrap();
        let a = &row["args"];
        assert!(ids.insert(row["id"].as_str().unwrap()), "duplicate case");
        let actual = match name {
            "retFieldLiteral" => field_json(p::retFieldLiteral().unwrap()),
            "constThenCall" => field_json(p::constThenCall().unwrap()),
            "retU128FieldLiteral" => field_json(p::retU128FieldLiteral().unwrap()),
            "retHugeFieldLiteral" => field_json(p::retHugeFieldLiteral().unwrap()),
            "constHugeFieldLiteral" => field_json(p::constHugeFieldLiteral().unwrap()),
            "callArgHugeFieldLiteral" => field_json(p::callArgHugeFieldLiteral().unwrap()),
            "callArgFieldOnlyLiteral" => field_json(p::callArgFieldOnlyLiteral().unwrap()),
            "constFieldOnlyLiteral" => field_json(p::constFieldOnlyLiteral().unwrap()),
            "retFieldOnlyLiteral" => field_json(p::retFieldOnlyLiteral().unwrap()),
            "callArgMaxUnsignedPlusOne" => field_json(p::callArgMaxUnsignedPlusOne().unwrap()),
            "idf" => field_json(p::idf(field(&a[0])).unwrap()),
            "addFieldLiteral" => field_json(p::addFieldLiteral(field(&a[0])).unwrap()),
            "addHugeFieldLiteral" => field_json(p::addHugeFieldLiteral(field(&a[0])).unwrap()),
            "litLhsFieldLiteral" => field_json(p::litLhsFieldLiteral(field(&a[0])).unwrap()),
            "subFieldLiteral" => field_json(p::subFieldLiteral(field(&a[0])).unwrap()),
            "mulHugeFieldLiteral" => field_json(p::mulHugeFieldLiteral(field(&a[0])).unwrap()),
            "nestedFieldArith" => field_json(p::nestedFieldArith(field(&a[0])).unwrap()),
            "addFieldOnlyLiteral" => field_json(p::addFieldOnlyLiteral(field(&a[0])).unwrap()),
            "hashZeroLiteral" => bytes_json(p::hashZeroLiteral().unwrap()),
            "hashConstVectorArg" => bytes_json(p::hashConstVectorArg().unwrap()),
            "hashCallVectorArg" => bytes_json(p::hashCallVectorArg().unwrap()),
            "hashDefaultVectorArg" => bytes_json(p::hashDefaultVectorArg().unwrap()),
            "nativeArgLiteral" => point_json(p::nativeArgLiteral().unwrap()),
            "subgroupCheck" => point_json(p::subgroupCheck().unwrap()),
            "structLiteral" => json!({"f":field_json(p::structLiteral().unwrap().f)}),
            "structMemberHugeFieldLiteral" => {
                json!({"f":field_json(p::structMemberHugeFieldLiteral().unwrap().f)})
            }
            "someFieldLiteral" => {
                let out = p::someFieldLiteral().unwrap();
                json!({"is_some":out.is_some,"value":field_json(out.value)})
            }
            "vectorEltHugeFieldLiteral" => vector_json(p::vectorEltHugeFieldLiteral().unwrap()),
            "sameTypeVec" => vector_json(p::sameTypeVec().unwrap()),
            "uintToField" => field_json(p::uintToField(unsigned(&a[0])).unwrap()),
            "u128Rung" => field_json(p::u128Rung(unsigned(&a[0])).unwrap()),
            "vectorEltWise" => vector_json(p::vectorEltWise(unsigned(&a[0])).unwrap()),
            "addUintFieldOperand" => {
                field_json(p::addUintFieldOperand(field(&a[0]), unsigned(&a[1])).unwrap())
            }
            "cmpHugeFieldLiteral" => json!(p::cmpHugeFieldLiteral(field(&a[0])).unwrap()),
            "cmpFieldOnlyLiteral" => json!(p::cmpFieldOnlyLiteral(field(&a[0])).unwrap()),
            "hashUintVarRefElem" => bytes_json(p::hashUintVarRefElem(unsigned(&a[0])).unwrap()),
            "hashNestedUintVarRefElem" => {
                bytes_json(p::hashNestedUintVarRefElem(unsigned(&a[0])).unwrap())
            }
            "hashVarRefVectorArg" => bytes_json(
                p::hashVarRefVectorArg(FixedVector::new([unsigned(&a[0][0]), unsigned(&a[0][1])]))
                    .unwrap(),
            ),
            "hashNestedVarRefVectorArg" => bytes_json(
                p::hashNestedVarRefVectorArg(FixedVector::new([
                    unsigned(&a[0][0]),
                    unsigned(&a[0][1]),
                ]))
                .unwrap(),
            ),
            "hashSameTypeVectorArg" => bytes_json(p::hashSameTypeVectorArg(vector(&a[0])).unwrap()),
            "hashSameTypeNestedArg" => bytes_json(p::hashSameTypeNestedArg(vector(&a[0])).unwrap()),
            "hashStructFieldVectorArg" => bytes_json(
                p::hashStructFieldVectorArg(types::VecBox {
                    v: vector(&a[0]["v"]),
                })
                .unwrap(),
            ),
            "nativeArgFieldOnlyLiteral" => {
                let point = ec_mul_generator(field(&a[0])).unwrap();
                assert_eq!(point_json(point), row["point"]);
                point_json(p::nativeArgFieldOnlyLiteral(point).unwrap())
            }
            _ => panic!("unreviewed literal export {name}"),
        };
        assert_eq!(actual, row["result"], "{}", row["id"]);
        executed.insert(name);
    }
    assert_eq!(executed, expected_exports);
}
