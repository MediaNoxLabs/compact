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
use compact_rust_did_alias_digest_reducer_fixture::{ledger_contract as ad, types as dt};
use compact_rust_did_alias_guard_reducer_fixture::{ledger_contract as ag, types as gt};
use compact_rust_did_alias_set_reducer_fixture::{ledger_contract as aset, types as st};
use r::context::{ConstructorContext, WitnessContext};
struct Witness;
impl ad::TryWitnesses<u64> for Witness {
    fn authorize(
        &self,
        context: WitnessContext<'_, u64, ad::LedgerView<'_>>,
        _message: r::Field,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((*context.private_state + 1, true))
    }
}
fn set_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(aset::initial_state(ConstructorContext::new(0))?)
}
fn digest_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(ad::initial_state(ConstructorContext::new(0))?)
}
fn guard_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(ag::initial_state(ConstructorContext::new(0))?)
}
fn set_call(case: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let m = match case.id {
        "insert-unicode" => st::Mutation::Insert,
        "remove-unicode" => st::Mutation::Remove,
        _ => return Err("unknown Set case".into()),
    };
    let v = text(ID);
    pair(
        aset::mutate(o.circuit_context(0), v.clone(), m)?,
        aset::recorded::mutate(o.circuit_context(0), v.clone(), m)?,
        AlignedValue::from((v, m)),
    )
}
fn digest_call(_: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let m = dt::Mutation::Insert;
    let v = text(ID);
    pair(
        ad::run(o.circuit_context(0), &Witness, v.clone(), m)?,
        ad::recorded::run(o.circuit_context(0), &Witness, v.clone(), m)?,
        AlignedValue::from((v, m)),
    )
}
fn guard_call(case: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let m = gt::Mutation::Insert;
    let (native, recorded) = match case.operation {
        "run" => (
            ag::run(o.circuit_context(0), m)?,
            ag::recorded::run(o.circuit_context(0), m)?,
        ),
        "local_control" => (
            ag::local_control(o.circuit_context(0), m)?,
            ag::recorded::local_control(o.circuit_context(0), m)?,
        ),
        "transitive_control" => (
            ag::transitive_control(o.circuit_context(0), m)?,
            ag::recorded::transitive_control(o.circuit_context(0), m)?,
        ),
        _ => return Err("unknown pure guard case".into()),
    };
    pair(native, recorded, AlignedValue::from((m,)))
}
pub(super) fn fixture(kind: &str) -> Result<Fixture, Failure> {
    match kind {
        "alias-set" => Ok(Fixture {
            operations: &["mutate"],
            cases: &[
                Case {
                    id: "insert-unicode",
                    operation: "mutate",
                },
                Case {
                    id: "remove-unicode",
                    operation: "mutate",
                },
            ],
            initial: set_initial,
            call: set_call,
        }),
        "alias-digest" => Ok(Fixture {
            operations: &["run"],
            cases: &[Case {
                id: "unicode-insert",
                operation: "run",
            }],
            initial: digest_initial,
            call: digest_call,
        }),
        "alias-guard" => Ok(Fixture {
            operations: &["run", "local_control", "transitive_control"],
            cases: &[
                Case {
                    id: "run-insert",
                    operation: "run",
                },
                Case {
                    id: "local_control-insert",
                    operation: "local_control",
                },
                Case {
                    id: "transitive_control-insert",
                    operation: "transitive_control",
                },
            ],
            initial: guard_initial,
            call: guard_call,
        }),
        _ => Err("unknown alias reducer".into()),
    }
}
