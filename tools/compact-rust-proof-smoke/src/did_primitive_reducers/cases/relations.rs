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
use compact_rust_did_relation_four_reducer_fixture::{ledger_contract as four, types as ft};
use compact_rust_did_relation_nested_reducer_fixture::{ledger_contract as nested, types as nt};
use compact_rust_did_relation_two_reducer_fixture::{ledger_contract as two, types as tt};
use r::context::ConstructorContext;

fn two_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(two::initial_state(ConstructorContext::new(0))?)
}
fn four_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(four::initial_state(ConstructorContext::new(0))?)
}
fn nested_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(nested::initial_state(
        ConstructorContext::new(0),
        nt::Method {
            id: text("x"),
            publicKeyJwk: nt::Jwk {
                curve: nt::Curve::X25519,
                x: text("x"),
            },
        },
    )?)
}
fn two_call(
    case: &Case,
    observed: &ObservedContractState,
) -> Result<(Recorded, AlignedValue), Failure> {
    let add = match case.id {
        "authentication-insert" => true,
        "authentication-remove" => false,
        _ => return Err("unreviewed two-set proof case".into()),
    };
    let relation = tt::Relation::Authentication;
    let id = text(ID);
    pair(
        two::update(observed.circuit_context(0), relation, id.clone(), add)?,
        two::recorded::update(observed.circuit_context(0), relation, id.clone(), add)?,
        AlignedValue::from((relation, id, add)),
    )
}
fn four_call(
    case: &Case,
    observed: &ObservedContractState,
) -> Result<(Recorded, AlignedValue), Failure> {
    let add = match case.id {
        "delegation-insert" => true,
        "delegation-remove" => false,
        _ => return Err("unreviewed four-set proof case".into()),
    };
    let relation = ft::Relation::Delegation;
    let id = text(ID);
    pair(
        four::update(observed.circuit_context(0), relation, id.clone(), add)?,
        four::recorded::update(observed.circuit_context(0), relation, id.clone(), add)?,
        AlignedValue::from((relation, id, add)),
    )
}
fn nested_call(
    case: &Case,
    observed: &ObservedContractState,
) -> Result<(Recorded, AlignedValue), Failure> {
    match case.id {
        "missing-signing-short-circuit" => {
            let id = text("missing");
            pair(
                nested::check(observed.circuit_context(0), id.clone(), false)?,
                nested::recorded::check(observed.circuit_context(0), id.clone(), false)?,
                AlignedValue::from((id, false)),
            )
        }
        "x25519-check" => {
            let id = text("x");
            pair(
                nested::check(observed.circuit_context(0), id.clone(), true)?,
                nested::recorded::check(observed.circuit_context(0), id.clone(), true)?,
                AlignedValue::from((id, true)),
            )
        }
        _ => Err("unreviewed nested relation proof case".into()),
    }
}
pub(super) fn fixture(kind: &str) -> Result<Fixture, Failure> {
    Ok(match kind {
        "relation-two" => Fixture {
            operations: &["update"],
            cases: &[
                Case {
                    id: "authentication-insert",
                    operation: "update",
                },
                Case {
                    id: "authentication-remove",
                    operation: "update",
                },
            ],
            initial: two_initial,
            call: two_call,
        },
        "relation-four" => Fixture {
            operations: &["update"],
            cases: &[
                Case {
                    id: "delegation-insert",
                    operation: "update",
                },
                Case {
                    id: "delegation-remove",
                    operation: "update",
                },
            ],
            initial: four_initial,
            call: four_call,
        },
        "relation-nested" => Fixture {
            operations: &["check"],
            cases: &[
                Case {
                    id: "missing-signing-short-circuit",
                    operation: "check",
                },
                Case {
                    id: "x25519-check",
                    operation: "check",
                },
            ],
            initial: nested_initial,
            call: nested_call,
        },
        _ => return Err("unreviewed relation reducer".into()),
    })
}
