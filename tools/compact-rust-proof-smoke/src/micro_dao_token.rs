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
//! Original microDAO token query: real nonempty proof and ledger application.
use super::*;
use compact_rust_test_center_micro_dao_fixture::{ledger_contract as c, types};
use midnight_base_crypto::hash::HashOutput;
use midnight_compact_runtime::{BoundedUint, FixedBytes};
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let name = "dao_voting_token";
    let mut rng = StdRng::seed_from_u64(0x0190_0000);
    let initial = c::initial_state(
        ConstructorContext::new(vec![77u8]),
        FixedBytes::new([4; 32]),
        types::Costs {
            seed_dust: BoundedUint::new(10)?,
            buy_in_dust: BoundedUint::new(3)?,
        },
    )?;
    let expected = initial.ledger_state.get_ref().clone();
    let deploy = make_deploy(root, name, expected.clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let recorded = c::recorded::dao_voting_token(observed.circuit_context(vec![77u8]))?;
    if recorded.public.verify_ops().is_empty()
        || !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.private_state != [77]
    {
        return Err("token query transcript/private invariant".into());
    }
    let native = c::dao_voting_token(observed.circuit_context(vec![77u8]))?;
    let mut domain = [0; 32];
    domain[..16].copy_from_slice(b"dao_voting_token");
    if recorded.execution.result != native.result
        || native.result.0
            != deploy
                .address()
                .custom_shielded_token_type(HashOutput(domain))
                .0
                .0
    {
        return Err("token derivation differs at deployed address".into());
    }
    let manual = check_generated_trace(root, name, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{name}.verifier")),
    )?))?;
    let typed = c::recorded::Contract.dao_voting_token_call(&observed, vec![77u8])?;
    let prepared = typed.prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("token observed/direct mismatch".into());
    }
    check_transaction(root, name, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected {
            return Err("token query changed ledger state".into());
        }
        Ok(())
    })?;
    println!(
        "original microDAO.dao_voting_token proved, verified and ledger-applied at deployed address under shared unbalanced smoke policy; no funding claim"
    );
    Ok(())
}
