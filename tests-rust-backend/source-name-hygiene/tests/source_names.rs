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

use compact_rust_source_name_hygiene_fixture::{pure_circuits, types};
use midnight_compact_runtime::fab::{AlignedValue, Value};
use midnight_compact_runtime::ledger::CellValue;
use midnight_compact_runtime::{Field, FieldRepr, FixedVector, FromFieldRepr};

fn value() -> types::Names {
    types::Names {
        option_value: types::Option {
            amount: Field::from(1_u64),
            active: true,
        },
        fr_value: types::Fr {
            amount: Field::from(2_u64),
            active: false,
        },
        memwrite_value: types::MemWrite {
            amount: Field::from(3_u64),
            active: true,
        },
        runtime_value: types::runtime {
            amount: Field::from(4_u64),
            active: false,
        },
        fieldrepr_value: types::FieldRepr {
            amount: Field::from(5_u64),
            active: true,
        },
        vec_value: types::Vec {
            amount: Field::from(6_u64),
            active: false,
        },
        result_value: types::Result {
            amount: Field::from(7_u64),
            active: true,
        },
        from_value: types::From {
            amount: Field::from(8_u64),
            active: false,
        },
        empty: types::Empty {},
        nested: FixedVector::new([
            types::Option {
                amount: Field::from(99_u64),
                active: false,
            },
            types::Option {
                amount: Field::from(100_u64),
                active: true,
            },
        ]),
    }
}

#[test]
fn source_names_remain_public_and_round_trip_through_generated_code() {
    let value = value();
    assert_eq!(pure_circuits::round_trip(value.clone()).unwrap(), value);
    let mut expected = Vec::new();
    for amount in 1..=8_u64 {
        expected.extend([Field::from(amount), Field::from(u64::from(amount % 2 == 1))]);
    }
    expected.extend([
        Field::from(99_u64),
        Field::from(0_u64),
        Field::from(100_u64),
        Field::from(1_u64),
    ]);
    assert_eq!(value.field_vec(), expected);
    assert_eq!(types::Names::from_field_repr(&expected), Some(value));
}

#[test]
fn generated_cell_codec_and_empty_struct_preserve_boundaries() {
    let value = value();
    let aligned: AlignedValue = value.clone().into();
    assert_eq!(
        types::Names::decode_cell_value(&aligned.value).unwrap(),
        value
    );
    let mut fields = value.field_vec();
    for length in 0..fields.len() {
        assert!(types::Names::from_field_repr(&fields[..length]).is_none());
    }
    fields.push(Field::from(0_u64));
    assert!(types::Names::from_field_repr(&fields).is_none());
    assert!(types::Empty::from_field_repr(&[]).is_some());
    assert!(types::Empty::from_field_repr(&[Field::from(0_u64)]).is_none());
    let empty: Value = types::Empty {}.into();
    assert!(empty.0.is_empty());
}
