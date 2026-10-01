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

use midnight_compact_runtime::fab::{Aligned, AlignedValue, Alignment, AlignmentAtom, Value};
use midnight_compact_runtime::{
    BinaryHashRepr, FieldRepr, FromFieldRepr, WideUint, narrow_wide_uint,
};

type Uint248 = WideUint<{ (1_u128 << 120) - 1 }, { u128::MAX }>;
type CustomWide = WideUint<1, 10>;

#[test]
fn wide_unsigned_uses_declared_31_byte_ledger_alignment() {
    let maximum = Uint248::from_le_bytes(&[0xff; 31]).unwrap();
    assert_eq!(
        Uint248::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 31 })
    );
    assert_eq!(maximum.as_le_bytes(), &[0xff; 31]);
    assert_eq!(maximum.binary_len(), 31);
    assert_eq!(maximum.field_size(), 1);
    assert_eq!(
        Uint248::from_field_repr(&[maximum.as_field()]),
        Some(maximum)
    );

    let aligned = AlignedValue::from(maximum);
    assert_eq!(aligned.alignment, Uint248::alignment());
    let value = Value::from(maximum);
    assert_eq!(value.0.len(), 1);
    assert_eq!(value.0[0].0, vec![0xff; 31]);
    assert_eq!(Uint248::try_from(&value[..]).unwrap(), maximum);
    let mut overflow = [0_u8; 32];
    overflow[31] = 1;
    assert!(Uint248::from_le_bytes(&overflow).is_err());
}

#[test]
fn arbitrary_wide_maximum_rejects_values_above_its_bound() {
    let mut within = [0_u8; 17];
    within[0] = 10;
    within[16] = 1;
    assert!(CustomWide::from_le_bytes(&within).is_ok());
    within[0] = 11;
    assert!(CustomWide::from_le_bytes(&within).is_err());
    let small = CustomWide::from_le_bytes(&[42]).unwrap();
    assert_eq!(narrow_wide_uint::<255, 1, 10>(small).unwrap().value(), 42);
    assert!(
        narrow_wide_uint::<255, 1, 10>(
            CustomWide::from_le_bytes(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])
                .unwrap()
        )
        .is_err()
    );
}
