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

use compact_rust_enum_identity_fixture::pure_circuits::{choice_identity, choose};
use compact_rust_enum_identity_fixture::types::Choice;
use midnight_compact_runtime::{BinaryHashRepr, Field, FieldRepr, FromFieldRepr};

#[test]
fn generated_enum_identity_and_ordinal_field_repr_round_trip() {
    assert_eq!(choice_identity(Choice::no).unwrap(), Choice::no);
    assert_eq!(Choice::no.field_vec(), vec![Field::from(1_u64)]);
    assert_eq!(
        Choice::from_field_repr(&Choice::yes.field_vec()),
        Some(Choice::yes)
    );
    assert_eq!(Choice::from_field_repr(&[Field::from(2_u64)]), None);
    assert_eq!(Choice::no.binary_vec(), vec![1]);
    assert_eq!(choose(true).unwrap(), Choice::yes);
    assert_eq!(choose(false).unwrap(), Choice::no);
}
