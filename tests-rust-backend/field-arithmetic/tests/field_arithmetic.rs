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

#[test]
fn generated_field_arithmetic_reduces_modulus_boundary_values() {
    // midnight-transient-crypto 2.0.1 uses midnight-curves 0.2.0 Fq with modulus
    // p = 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
    // These canonical little-endian bytes were calculated independently from
    // that constant, not with the runtime arithmetic being checked. This is a
    // Compact source-semantic control, not an additional TypeScript capture.
    const P_MINUS_ONE: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0x02, 0xa4, 0xbd,
        0x53, 0x05, 0xd8, 0xa1, 0x09, 0x08, 0xd8, 0x39, 0x33, 0x48, 0x7d, 0x9d, 0x29, 0x53, 0xa7,
        0xed, 0x73,
    ];
    const P_MINUS_TWO: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xfe, 0xff, 0xff, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0x02, 0xa4, 0xbd,
        0x53, 0x05, 0xd8, 0xa1, 0x09, 0x08, 0xd8, 0x39, 0x33, 0x48, 0x7d, 0x9d, 0x29, 0x53, 0xa7,
        0xed, 0x73,
    ];
    let mut one = [0; 32];
    one[0] = 1;
    let near_modulus = Field::from_le_bytes(&P_MINUS_ONE).unwrap();

    assert_eq!(
        multiply(near_modulus, near_modulus).unwrap().as_le_bytes(),
        one,
        "(p - 1) squared reduces to 1"
    );
    assert_eq!(
        multiply(near_modulus, Field::from(2_u64))
            .unwrap()
            .as_le_bytes(),
        P_MINUS_TWO,
        "2 * (p - 1) reduces to p - 2 without narrowing the result"
    );
    assert_eq!(
        subtract(Field::from(0_u64), Field::from(1_u64))
            .unwrap()
            .as_le_bytes(),
        P_MINUS_ONE,
        "0 - 1 wraps to the canonical p - 1 bytes"
    );
}
