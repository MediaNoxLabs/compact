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

use compact_rust_struct_identity_fixture::pure_circuits::pair_identity;
use compact_rust_struct_identity_fixture::types::Pair;
use midnight_compact_runtime::{Field, FieldRepr, FromFieldRepr};

#[test]
fn generated_struct_identity_and_ledger_field_repr_round_trip() {
    let value = Pair {
        amount: Field::from(42_u64),
        active: true,
    };
    assert_eq!(pair_identity(value.clone()).unwrap(), value);
    assert_eq!(Pair::from_field_repr(&value.field_vec()), Some(value));
}
