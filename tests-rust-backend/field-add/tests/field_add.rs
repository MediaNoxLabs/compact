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

use compact_rust_field_add_fixture::pure_circuits::field_add;
use midnight_compact_runtime::Field;

#[test]
fn field_addition_uses_the_ledger_field_implementation() {
    assert_eq!(
        field_add(Field::from(3_u64), Field::from(5_u64)).unwrap(),
        Field::from(8_u64)
    );
}
