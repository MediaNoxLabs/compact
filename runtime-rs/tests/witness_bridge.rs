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

use midnight_compact_runtime as runtime;

#[runtime::compact_witness_bridge]
pub trait Witnesses<Private> {
    fn secret(&self, context: u64, seed: u16) -> (Private, u64);
    fn observe(&self, context: u64) -> (Private, ());
}

struct Infallible;

impl Witnesses<u64> for Infallible {
    fn secret(&self, context: u64, seed: u16) -> (u64, u64) {
        (context + 1, u64::from(seed))
    }

    fn observe(&self, context: u64) -> (u64, ()) {
        (context, ())
    }
}

struct Fallible;

impl TryWitnesses<u64> for Fallible {
    fn secret(&self, context: u64, seed: u16) -> Result<(u64, u64), runtime::CompactError> {
        if seed == 0 {
            Err(runtime::CompactError::AssertionFailed("rejected".into()))
        } else {
            Ok((context + 1, u64::from(seed)))
        }
    }

    fn observe(&self, context: u64) -> Result<(u64, ()), runtime::CompactError> {
        Ok((context, ()))
    }
}

#[test]
fn infallible_witnesses_adapt_and_fallible_witnesses_keep_errors() {
    assert_eq!(TryWitnesses::secret(&Infallible, 5, 7).unwrap(), (6, 7));
    assert_eq!(TryWitnesses::observe(&Infallible, 5).unwrap(), (5, ()));
    assert_eq!(TryWitnesses::secret(&Fallible, 5, 7).unwrap(), (6, 7));
    assert_eq!(
        TryWitnesses::secret(&Fallible, 5, 0),
        Err(runtime::CompactError::AssertionFailed("rejected".into()))
    );
}

mod empty {
    use midnight_compact_runtime as runtime;

    #[runtime::compact_witness_bridge]
    pub trait Witnesses<Private> {}

    pub struct Empty;
    impl Witnesses<()> for Empty {}

    pub fn accepts_fallible<T: TryWitnesses<()>>() {}
}

#[test]
fn empty_witness_trait_still_has_its_fallible_companion() {
    empty::accepts_fallible::<empty::Empty>();
}
