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

use compact_rust_bytes_identity_fixture::pure_circuits::bytes_identity;
use midnight_base_crypto::fab::{
    Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom,
};
use midnight_compact_runtime::FixedBytes;

#[test]
fn generated_bytes_use_ledger_fixed_array_alignment_and_normalization() {
    let value = FixedBytes::<4>::new([1, 2, 0, 0]);
    assert_eq!(bytes_identity(value).unwrap(), value);
    assert_eq!(
        FixedBytes::<4>::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 4 })
    );
    assert_eq!(
        AlignedValue::from(value).value,
        Value(vec![ValueAtom(vec![1, 2])])
    );
}
