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

//! Checked construction accepts endpoints and rejects the first out-of-range value.
use compact_contract_counter::runtime::{BoundedUint, CompactError};

#[test]
fn checked_constructor_enforces_both_boundaries() {
    assert_eq!(BoundedUint::<255>::new(0).unwrap().value(), 0);
    assert_eq!(BoundedUint::<255>::new(255).unwrap().value(), 255);
    assert!(matches!(
        BoundedUint::<255>::new(256),
        Err(CompactError::UnsignedOutOfRange {
            value: 256,
            max: 255
        })
    ));
}
