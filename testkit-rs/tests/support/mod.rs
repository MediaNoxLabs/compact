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

use compact_rust_counter_parameter_fixture::ledger_contract as counter;
use midnight_compact_testkit::runtime::{context::ConstructorContext, ledger::ContractAddress};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment};

pub fn environment() -> Environment {
    Environment::new(ContractAddress::default(), Default::default(), [7; 32])
}
pub fn identity_for(source: &[u8], generated: &[u8]) -> ArtifactIdentity {
    // Fixture callers hash their actual source and generated crate; ContractLab
    // only compares these declarations and does not authenticate their origin.
    ArtifactIdentity {
        source_sha256: midnight_base_crypto::hash::persistent_hash(source).0,
        generated_sha256: midnight_base_crypto::hash::persistent_hash(generated).0,
    }
}
pub fn identity() -> ArtifactIdentity {
    identity_for(
        include_bytes!("../../../examples/rust_backend/counter_parameter.compact"),
        include_bytes!("../../../tests-rust-backend/counter-parameter/lib.rs"),
    )
}
pub fn counter() -> ContractLab<Vec<String>> {
    ContractLab::from_constructor(
        identity(),
        environment(),
        counter::initial_state(ConstructorContext::new(vec![])).unwrap(),
    )
    .unwrap()
}
