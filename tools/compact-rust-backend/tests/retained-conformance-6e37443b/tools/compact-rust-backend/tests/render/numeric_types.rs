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

//! Numeric bounds, casts, canonical literals, and primitive input types.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, Parameter, PureCircuit, SCHEMA_VERSION,
    StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn field_cast_requires_an_unsigned_operand() {
    let contract = identity(
        Type::Field,
        Expr::FieldCast {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::ExpectedUnsigned(Type::Field))
    );
}

#[test]
fn explicit_field_to_bytes32_checks_operand_and_records_only_typed_counter_read() {
    let mut pure = identity(
        Type::Bytes { length: 32 },
        Expr::FieldToBytes32 {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    let source = render(&pure).unwrap();
    assert!(source.contains("as_le_bytes()"));
    assert!(source.contains("runtime::FixedBytes"));
    pure.circuits[0].body = Expr::FieldToBytes32 {
        value: Box::new(Expr::Boolean { value: true }),
    };
    assert_eq!(
        render(&pure),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );

    let mut stateful = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "instance".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "snapshot".into(),
            internal: false,
            parameters: vec![],
            actions: vec![],
            result: Type::Bytes { length: 32 },
            return_value: StateReturn::Expression {
                value: Expr::FieldToBytes32 {
                    value: Box::new(Expr::FieldCast {
                        value: Box::new(Expr::CounterRead {
                            field: "instance".into(),
                            index: 0,
                        }),
                    }),
                },
            },
        }],
    };
    let rendered = render_with_capabilities(&stateful).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains("record_read(frame)?"));

    stateful.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert_eq!(
        render(&stateful),
        Err(RenderError::UnknownLedgerField("instance".into()))
    );
}

#[test]
fn rejects_wrong_native_hash_opening_and_conversion_types() {
    let field = Expr::Parameter {
        name: "value".into(),
    };
    let contract = identity(
        Type::Bytes { length: 32 },
        Expr::PersistentCommit {
            value: Box::new(field.clone()),
            opening: Box::new(field.clone()),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );

    let contract = identity(
        Type::Field,
        Expr::DegradeToTransient {
            value: Box::new(field),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_field_as_jubjub_point_coordinate_input() {
    let contract = identity(
        Type::Field,
        Expr::JubjubPointX {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::JubjubPoint,
            actual: Type::Field,
        })
    );
}

#[test]
fn unsigned_add_and_cast_reject_field_inputs() {
    let mut contract = identity(
        Type::Unsigned { max: "511".into() },
        Expr::UnsignedAdd {
            max: "511".into(),
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "255".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
    contract.circuits[0].body = Expr::UnsignedCast {
        max: "511".into(),
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
}

#[test]
fn unsigned_literal_checks_the_declared_maximum() {
    let mut contract = identity(
        Type::Unsigned { max: "255".into() },
        Expr::UnsignedLiteral {
            value: "255".into(),
            max: "255".into(),
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("BoundedUint::<255>::new(255u128)")
    );
    contract.circuits[0].body = Expr::UnsignedLiteral {
        value: "256".into(),
        max: "255".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidUnsignedLiteral {
            value: "256".into(),
            max: "255".into(),
        })
    );
}

#[test]
fn field_literal_requires_canonical_ledger_field_value() {
    let mut contract = identity(Type::Field, Expr::FieldLiteral { value: "42".into() });
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from(42u128)")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "042".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral("042".into()))
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "340282366920938463463374607431768211456".into(),
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from_le_bytes")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "115792089237316195423570985008687907853269984665640564039457584007913129639935"
            .into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral(
            "115792089237316195423570985008687907853269984665640564039457584007913129639935".into()
        ))
    );
}

#[test]
fn refuses_to_add_a_boolean_to_a_field() {
    let contract = identity(
        Type::Field,
        Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::Boolean { value: true }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn rejects_noncanonical_or_unsupported_unsigned_maxima() {
    for max in [
        "08",
        "-1",
        "452312848583266388373324160190187140051835877600158453279131187530910662656",
    ] {
        let contract = Contract {
            schema_version: SCHEMA_VERSION,
            type_aliases: vec![],
            constructor: None,
            witnesses: vec![],
            ledger_fields: vec![],
            circuits: vec![PureCircuit {
                source: None,
                internal: false,
                name: "id_u".into(),
                parameters: vec![Parameter {
                    name: "value".into(),
                    ty: Type::Unsigned { max: max.into() },
                }],
                result: Type::Unsigned { max: max.into() },
                body: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            stateful_circuits: vec![],
        };
        assert_eq!(
            render(&contract),
            Err(RenderError::InvalidUnsignedMaximum(max.into()))
        );
    }
}

#[test]
fn wide_addition_validates_every_bound_and_does_not_admit_other_wide_operators() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../wide-add-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let out = render_with_capabilities(&contract).unwrap();
    assert!(out.source.contains("runtime::add_wide_unsigned"));
    assert!(out.source.contains("runtime::narrow_wide_uint"));
    for mode in 0..6 {
        let mut wrong = contract.clone();
        let c = wrong
            .circuits
            .iter_mut()
            .find(|c| c.name == "add128")
            .unwrap();
        let Expr::UnsignedCast { value, .. } = &mut c.body else {
            panic!()
        };
        let Expr::UnsignedAdd { max, left, right } = value.as_mut() else {
            panic!()
        };
        match mode {
            0 => *max = "0680564733841876926926749214863536422910".into(),
            1 => {
                *max = "452312848583266388373324160190187140051835877600158453279131187530910662656"
                    .into()
            }
            2 => **left = Expr::Boolean { value: false },
            3 => {
                let Expr::UnsignedCast { max, .. } = left.as_mut() else {
                    panic!()
                };
                *max = "-1".into();
            }
            4 => {
                **value = Expr::UnsignedSubtract {
                    max: max.clone(),
                    left: left.clone(),
                    right: right.clone(),
                }
            }
            5 => {
                **value = Expr::UnsignedMultiply {
                    max: max.clone(),
                    left: left.clone(),
                    right: right.clone(),
                }
            }
            _ => unreachable!(),
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "invalid wide arithmetic mode {mode}"
        );
    }
    let mut wrong = contract.clone();
    let c = wrong
        .circuits
        .iter_mut()
        .find(|c| c.name == "mixed")
        .unwrap();
    c.result = Type::Boolean;
    c.body = Expr::Compare {
        operator: compact_rust_backend::ir::ComparisonOperator::Less,
        left: Box::new(Expr::Parameter { name: "a".into() }),
        right: Box::new(Expr::Parameter { name: "b".into() }),
    };
    assert!(render_with_capabilities(&wrong).is_err());
    // Stateful arithmetic shares the same admission checks.
    for kind in ["unsigned_subtract", "unsigned_multiply", "compare"] {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../wide-add-schema20-ir.json")).unwrap();
        fn replace(value: &mut serde_json::Value, kind: &str) {
            if let Some(obj) = value.as_object_mut() {
                if obj.get("kind").and_then(|v| v.as_str()) == Some("unsigned_add") {
                    obj.insert("kind".into(), kind.into());
                    if kind == "compare" {
                        obj.remove("max");
                        obj.insert("operator".into(), "less".into());
                    }
                    return;
                }
                for v in obj.values_mut() {
                    replace(v, kind);
                }
            } else if let Some(values) = value.as_array_mut() {
                for v in values {
                    replace(v, kind);
                }
            }
        }
        replace(&mut value["stateful_circuits"], kind);
        let mut wrong: Contract = serde_json::from_value(value).unwrap();
        wrong.schema_version = SCHEMA_VERSION;
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "stateful wide {kind}"
        );
    }
}
