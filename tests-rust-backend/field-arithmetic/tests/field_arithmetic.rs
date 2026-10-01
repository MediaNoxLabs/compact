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

use compact_rust_field_arithmetic_fixture::pure_circuits::{multiply, subtract};
use midnight_compact_runtime::Field;

#[test]
fn generated_field_arithmetic_uses_ledger_field_operations() {
    assert_eq!(
        subtract(Field::from(9_u64), Field::from(4_u64)).unwrap(),
        Field::from(5_u64)
    );
    assert_eq!(
        subtract(Field::from(0_u64), Field::from(1_u64)).unwrap(),
        -Field::from(1_u64)
    );
    assert_eq!(
        multiply(Field::from(7_u64), Field::from(6_u64)).unwrap(),
        Field::from(42_u64)
    );
}
