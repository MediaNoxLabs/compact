// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use compact_rust_did_point_digest_reducer_fixture::ledger_contract as pd;
use compact_rust_did_point_guard_reducer_fixture::ledger_contract as pg;
use r::context::{ConstructorContext, WitnessContext};
struct Witness;
impl pd::TryWitnesses<u64> for Witness {
    fn authorize(
        &self,
        context: WitnessContext<'_, u64, pd::LedgerView<'_>>,
        _message: r::Field,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((*context.private_state + 1, true))
    }
}
fn digest_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(pd::initial_state(ConstructorContext::new(0), point(1)?)?)
}
fn guard_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(pg::initial_state(
        ConstructorContext::new(0),
        point(1)?,
        point(2)?,
    )?)
}
fn digest_call(_: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let p = point(3)?;
    pair(
        pd::update(o.circuit_context(0), &Witness, p)?,
        pd::recorded::update(o.circuit_context(0), &Witness, p)?,
        AlignedValue::from((p,)),
    )
}
fn guard_call(_: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let p = point(3)?;
    pair(
        pg::check(o.circuit_context(0), p)?,
        pg::recorded::check(o.circuit_context(0), p)?,
        AlignedValue::from((p,)),
    )
}
pub(super) fn fixture(kind: &str) -> Result<Fixture, Failure> {
    match kind {
        "point-digest" => Ok(Fixture {
            operations: &["update"],
            cases: &[Case {
                id: "update",
                operation: "update",
            }],
            initial: digest_initial,
            call: digest_call,
        }),
        "point-guard" => Ok(Fixture {
            operations: &["check"],
            cases: &[Case {
                id: "different-both",
                operation: "check",
            }],
            initial: guard_initial,
            call: guard_call,
        }),
        _ => Err("unknown Point reducer".into()),
    }
}
