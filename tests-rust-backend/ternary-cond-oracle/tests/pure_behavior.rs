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
//! Direct pure exports, including actionless exports normalized to pure by the compiler.
use compact_rust_ternary_cond_oracle_fixture::{pure_circuits as p, types};
use midnight_compact_runtime::{BoundedUint, CompactError, Field, FixedVector, JubjubPoint};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn field(v: &Value) -> Field {
    v.as_str().unwrap().bytes().fold(Field::from(0u64), |n, d| {
        assert!(d.is_ascii_digit());
        n * Field::from(10u64) + Field::from(u64::from(d - b'0'))
    })
}
fn uint<const MAX: u128>(v: &Value) -> BoundedUint<MAX> {
    BoundedUint::new(v.as_str().unwrap().parse().unwrap()).unwrap()
}
fn fj(v: Field) -> Value {
    json!(hex::encode(v.as_le_bytes()))
}
fn uj<const MAX: u128>(v: BoundedUint<MAX>) -> Value {
    fj(Field::from(v.value()))
}
fn mj(v: types::Maybe) -> Value {
    json!({"is_some":v.is_some,"value":fj(v.value)})
}
fn vj<const N: usize>(v: FixedVector<Field, N>) -> Value {
    Value::Array(v.0.into_iter().map(fj).collect())
}
fn pj(v: JubjubPoint) -> Value {
    json!({"x":fj(v.x().unwrap()),"y":fj(v.y().unwrap())})
}
fn invoke(name: &str, a: &Value) -> Result<Value, CompactError> {
    let condition = || a[0].as_bool().unwrap();
    Ok(match name {
        "idf" => fj(p::idf(field(&a[0]))?),
        "constAnnotatedBothLiteral" => uj(p::constAnnotatedBothLiteral(condition())?),
        "constUnannotatedSeqLifted" => uj(p::constUnannotatedSeqLifted(condition(), uint(&a[1]))?),
        "returnTailMixed" => uj(p::returnTailMixed(condition(), uint(&a[1]))?),
        "returnTailNested" => uj(p::returnTailNested(condition(), a[1].as_bool().unwrap())?),
        "assertArg" => {
            p::assertArg(condition(), uint(&a[1]))?;
            json!([])
        }
        "arithOperand" => uj(p::arithOperand(condition(), uint(&a[1]))?),
        "cmpOperand" => json!(p::cmpOperand(condition(), uint(&a[1]))?),
        "callArgPure" => fj(p::callArgPure(condition(), field(&a[1]), field(&a[2]))?),
        "callArgCtor" => mj(p::callArgCtor(condition(), field(&a[1]), field(&a[2]))?),
        "structMember" => json!({"f":fj(p::structMember(condition())?.f)}),
        "structValuedArms" => {
            json!({"f":fj(p::structValuedArms(condition(),types::Box{f:field(&a[1]["f"])},types::Box{f:field(&a[2]["f"])})?.f)})
        }
        "vectorElement" => vj(p::vectorElement(condition())?),
        "nativeArg" => pj(p::nativeArg(condition(), field(&a[1]), field(&a[2]))?),
        "fieldTypedArms" => fj(p::fieldTypedArms(condition(), field(&a[1]), field(&a[2]))?),
        "enumValued" => json!(match p::enumValued(condition())? {
            types::Color::red => 0,
            types::Color::green => 1,
            types::Color::blue => 2,
        }),
        "literalAboveI32" => uj(p::literalAboveI32(condition())?),
        "literalAboveU64" => uj(p::literalAboveU64(condition())?),
        "differingUintWidths" => uj(p::differingUintWidths(
            condition(),
            uint(&a[1]),
            uint(&a[2]),
        )?),
        "uintArmIntoField" => fj(p::uintArmIntoField(condition(), uint(&a[1]), uint(&a[2]))?),
        "pick" => uj(p::pick(uint(&a[0]))?),
        "walkerReturnTail" => fj(p::walkerReturnTail(condition())?),
        "walkerCallCtor" => mj(p::walkerCallCtor(condition())?),
        "nativeVectorElemTernary" => json!(hex::encode(p::nativeVectorElemTernary(condition())?.0)),
        "largeLiteralArithOperand" => uj(p::largeLiteralArithOperand(condition(), uint(&a[1]))?),
        _ => panic!("unreviewed export {name}"),
    })
}
#[test]
fn every_pure_ternary_export_matches_independent_typescript_paths_and_errors() {
    let capture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-pure-behavior.json"
    ))
    .unwrap();
    let expected: BTreeSet<&str> = [
        "arithOperand",
        "assertArg",
        "callArgCtor",
        "callArgPure",
        "cmpOperand",
        "constAnnotatedBothLiteral",
        "constUnannotatedSeqLifted",
        "differingUintWidths",
        "enumValued",
        "fieldTypedArms",
        "idf",
        "largeLiteralArithOperand",
        "literalAboveI32",
        "literalAboveU64",
        "nativeArg",
        "nativeVectorElemTernary",
        "pick",
        "returnTailMixed",
        "returnTailNested",
        "structMember",
        "structValuedArms",
        "uintArmIntoField",
        "vectorElement",
        "walkerCallCtor",
        "walkerReturnTail",
    ]
    .into_iter()
    .collect();
    assert_eq!(expected.len(), 25);
    assert_eq!(
        capture["exports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        expected
    );
    let cases = capture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 83);
    let mut seen = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut failures = 0;
    for row in cases {
        let name = row["export"].as_str().unwrap();
        let id = row["id"].as_str().unwrap();
        assert!(ids.insert(id), "duplicate {id}");
        seen.insert(name);
        let actual = invoke(name, &row["args"]);
        if row["ok"] == true {
            assert_eq!(
                actual.unwrap_or_else(|e| panic!("{id}: {e}")),
                row["result"],
                "{id}"
            );
        } else {
            failures += 1;
            let error = actual.expect_err(id);
            assert_eq!(row["errorClass"], "CompactError");
            match name {
                "assertArg" => {
                    assert!(
                        matches!(&error,CompactError::AssertionFailed(message) if message=="ternary assert")
                    );
                    assert_eq!(error.to_string(), row["error"]);
                }
                "constUnannotatedSeqLifted" | "pick" => {
                    assert!(
                        matches!(error, CompactError::UnsignedUnderflow),
                        "{id}: {error}"
                    );
                    // TS inserts a checked-subtraction assertion; Rust reports its typed arithmetic error.
                    assert_eq!(
                        row["error"],
                        "failed assert: result of subtraction would be negative"
                    );
                    assert_eq!(error.to_string(), "Compact unsigned arithmetic underflow");
                }
                _ => panic!("unreviewed failure {id}: {error}"),
            }
        }
    }
    assert_eq!(seen, expected);
    assert_eq!(failures, 8);
}
