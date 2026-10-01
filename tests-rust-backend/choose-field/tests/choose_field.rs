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

use compact_rust_choose_field_fixture::pure_circuits::{
    answer, call_sum, choose, sum_with_local, wide_constant,
};
use midnight_compact_runtime::Field;

#[test]
fn generated_conditional_selects_each_field_branch() {
    let left = Field::from(7_u64);
    let right = Field::from(19_u64);
    assert_eq!(choose(true, left, right).unwrap(), left);
    assert_eq!(choose(false, left, right).unwrap(), right);
    assert_eq!(
        sum_with_local(left, right).unwrap(),
        left + right + left + right
    );
    assert_eq!(call_sum(left, right).unwrap(), left + right + left + right);
    assert_eq!(answer().unwrap(), Field::from(42_u64));
    assert_eq!(wide_constant().unwrap(), Field::from(1_u128 << 64));
}
