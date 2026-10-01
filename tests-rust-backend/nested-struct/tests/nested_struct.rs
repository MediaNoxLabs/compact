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

use compact_rust_nested_struct_fixture::pure_circuits::nested_identity;
use compact_rust_nested_struct_fixture::types::{Inner, Outer};
use midnight_compact_runtime::ledger::{DefaultDB, constructor_cell, read_cell};
use midnight_compact_runtime::{BinaryHashRepr, BoundedUint, FieldRepr, FixedBytes, FromFieldRepr};

#[test]
fn generated_nested_struct_round_trips_through_ledger_field_repr() {
    let value = Outer {
        tag: FixedBytes::new([1, 2, 0, 0]),
        inner: Inner {
            value: BoundedUint::<255>::new(200).unwrap(),
        },
    };
    assert_eq!(nested_identity(value.clone()).unwrap(), value);
    assert_eq!(
        Outer::from_field_repr(&value.field_vec()),
        Some(value.clone())
    );
    assert_eq!(value.binary_vec(), vec![1, 2, 0, 0, 200]);
    let cell = constructor_cell::<Outer, DefaultDB>(value.clone());
    assert_eq!(read_cell::<Outer, _>(&cell).unwrap(), value);
}
