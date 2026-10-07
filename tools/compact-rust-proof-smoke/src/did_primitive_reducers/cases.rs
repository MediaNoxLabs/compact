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

use midnight_compact_runtime as r;
use r::{
    context::{CircuitResult, ConstructorResult},
    fab::AlignedValue,
    recording::RecordedCircuitResult,
    transaction::ObservedContractState,
};
use std::error::Error;
#[path = "cases/aliases.rs"]
mod aliases;
#[path = "cases/maps.rs"]
mod maps;
#[path = "cases/points.rs"]
mod points;
pub type Failure = Box<dyn Error>;
pub type Recorded = RecordedCircuitResult<u64, ()>;
pub struct Case {
    pub id: &'static str,
    pub operation: &'static str,
}
type RunCase = fn(&Case, &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure>;

pub struct Fixture {
    pub operations: &'static [&'static str],
    pub cases: &'static [Case],
    pub initial: fn() -> Result<ConstructorResult<u64>, Failure>,
    pub call: RunCase,
}
pub fn fixture(kind: &str) -> Result<Fixture, Failure> {
    match kind {
        "point-digest" | "point-guard" => points::fixture(kind),
        "alias-set" | "alias-digest" | "alias-guard" => aliases::fixture(kind),
        "service-map" | "point-map" | "nested-map" | "enum-map" => maps::fixture(kind),
        _ => Err("unknown reducer".into()),
    }
}
fn pair(
    native: CircuitResult<u64, ()>,
    recorded: Recorded,
    input: AlignedValue,
) -> Result<(Recorded, AlignedValue), Failure> {
    if native.context.query.state != recorded.execution.context.query.state
        || native.context.query.effects != recorded.execution.context.query.effects
        || native.context.private_state != recorded.execution.context.private_state
        || native.gas_cost != recorded.execution.gas_cost
        || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
    {
        return Err("native recorded mismatch".into());
    }
    Ok((recorded, input))
}
fn point(n: u64) -> Result<r::JubjubPoint, Failure> {
    Ok(r::ec_mul_generator(r::Field::from(n))?)
}
fn text(s: &str) -> r::OpaqueString {
    r::OpaqueString(s.to_owned())
}
const ID: &str = "référence-東京";
