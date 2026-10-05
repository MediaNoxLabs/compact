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

//! Prove a guarded pure arithmetic call followed by one Counter increment.

use super::*;
use compact_rust_guarded_assert_arith_oracle_fixture::ledger_contract as guarded;
use compact_rust_guarded_assert_arith_oracle_fixture::types::{
    Attestation, StatusProof, VerifierPolicy,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "recordFreshEnough";
    let mut rng = StdRng::seed_from_u64(0x0091_4755_4152_4433);
    let initial = guarded::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let policy = VerifierPolicy {
        enforceMaxAge: true,
        maxAge: BoundedUint::new(20)?,
    };
    let attestation = Attestation {
        proof: StatusProof {
            createdAt: BoundedUint::new(100)?,
            issuer: BoundedUint::new(1)?,
        },
        hasExpiration: false,
        expiresAt: BoundedUint::new(0)?,
    };
    let now = BoundedUint::new(110)?;
    let input = AlignedValue::concat(&[
        AlignedValue::from(policy.clone()),
        AlignedValue::from(attestation.clone()),
        AlignedValue::from(now),
    ]);
    let recorded =
        guarded::recorded::recordFreshEnough(context, policy.clone(), attestation.clone(), now)?;
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, CIRCUIT, recorded, input)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/recordFreshEnough.verifier"),
    )?))?;
    let prepared = guarded::recorded::Contract
        .recordFreshEnough_call(&observed, (), policy, attestation, now)?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("guarded arithmetic observed call differs from manual prototype".into());
    }
    check_transaction(root, CIRCUIT, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("guarded arithmetic proven state differs from recorded state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("guarded arithmetic state is not an array".into());
        };
        if read_counter(fields.get(0).ok_or("missing accepted Counter")?)? != 1 {
            return Err("guarded arithmetic proof did not increment accepted".into());
        }
        Ok(())
    })?;
    println!("guarded arithmetic proved, validated, and applied through ledger-8");
    Ok(())
}
