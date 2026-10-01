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

use compact_rust_uint_arithmetic_fixture::pure_circuits::{multiply_uint, subtract_uint};
use midnight_compact_runtime::{BoundedUint, CompactError};

#[test]
fn generated_uint_arithmetic_checks_bounds_and_underflow() {
    let seven = BoundedUint::<255>::new(7).unwrap();
    let five = BoundedUint::<255>::new(5).unwrap();
    assert_eq!(subtract_uint(seven, five).unwrap().value(), 2);
    assert_eq!(
        subtract_uint(five, seven),
        Err(CompactError::UnsignedUnderflow)
    );
    assert_eq!(multiply_uint(seven, five).unwrap().value(), 35);
    let max = BoundedUint::<255>::new(255).unwrap();
    assert_eq!(multiply_uint(max, max).unwrap().value(), 65_025);
}
