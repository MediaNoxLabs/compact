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

use midnight_compact_runtime::fab::{Aligned, Value};
use midnight_compact_runtime::ledger::CellValue;
use midnight_compact_runtime::{BinaryHashRepr, FieldRepr, FromFieldRepr, OpaqueString};

#[test]
fn opaque_string_uses_ledger_compress_encoding() {
    assert_eq!(OpaqueString::alignment(), <Vec<u8> as Aligned>::alignment());
    for text in [
        "",
        "registry-key",
        "a longer key spanning more than thirty-one bytes",
    ] {
        let string = OpaqueString::from(text);
        let bytes = text.as_bytes().to_vec();
        let value: Value = string.clone().into();
        assert_eq!(value, Value::from(bytes.clone()));
        assert_eq!(OpaqueString::decode_cell_value(&value).unwrap(), string);
        assert_eq!(string.field_vec(), bytes.as_slice().field_vec());
        assert_eq!(string.binary_vec(), bytes);
        if !text.is_empty() {
            assert!(OpaqueString::from_field_repr(&string.field_vec()).is_none());
        }
    }
    assert_eq!(OpaqueString::FIELD_SIZE, 0);
    assert_eq!(
        OpaqueString::from_field_repr(&[]),
        Some(OpaqueString::default())
    );
    assert!(OpaqueString::decode_cell_value(&Value::from(vec![0xff])).is_err());
}
