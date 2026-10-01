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

use midnight_compact_runtime::{Field, FieldRepr, Fr, FromFieldRepr, MemWrite};

#[derive(Debug, PartialEq, Eq, FieldRepr, FromFieldRepr)]
struct Pair {
    left: Field,
    right: Field,
}

#[test]
fn ledger_derive_round_trips_a_compact_struct() {
    let value = Pair {
        left: Field::from(3_u64),
        right: Field::from(5_u64),
    };
    let encoded = value.field_vec();
    assert_eq!(encoded.len(), 2);
    assert_eq!(Pair::from_field_repr(&encoded), Some(value));
}
