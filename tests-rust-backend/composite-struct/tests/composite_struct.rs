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

use compact_rust_composite_struct_fixture::pure_circuits::composite_identity;
use compact_rust_composite_struct_fixture::types::Composite;
use midnight_compact_runtime::ledger::{DefaultDB, constructor_cell, read_cell};
use midnight_compact_runtime::{Field, FieldRepr, FixedVector, FromFieldRepr};

#[test]
fn generated_vector_and_tuple_fields_round_trip() {
    let value = Composite {
        vector: FixedVector::new([Field::from(1_u64), Field::from(2_u64)]),
        pair: (Field::from(3_u64), true),
    };
    assert_eq!(composite_identity(value.clone()).unwrap(), value);
    assert_eq!(
        Composite::from_field_repr(&value.field_vec()),
        Some(value.clone())
    );
    let cell = constructor_cell::<Composite, DefaultDB>(value.clone());
    assert_eq!(read_cell::<Composite, _>(&cell).unwrap(), value);
}
