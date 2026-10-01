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

use midnight_base_crypto::fab::{AlignedValue, Value, ValueAtom};
use midnight_compact_runtime::ledger::CellValue;
use midnight_compact_runtime::{BinaryHashRepr, FieldRepr, FromFieldRepr, OpaqueBytes};

#[test]
fn opaque_byte_array_preserves_ledger_atom_and_hash_bytes() {
    let value = OpaqueBytes::from(vec![10, 11, 12]);
    let aligned = AlignedValue::from(value.clone());
    assert_eq!(aligned.value, Value(vec![ValueAtom(vec![10, 11, 12])]));
    assert_eq!(
        OpaqueBytes::decode_cell_value(&aligned.as_slice()),
        Ok(value.clone())
    );
    assert_eq!(value.binary_vec(), vec![10, 11, 12]);
    assert_eq!(value.field_size(), 1);
    assert_eq!(
        OpaqueBytes::from_field_repr(&value.field_vec()),
        Some(value)
    );
}
