// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Prove direct plain-tree root checks for both Boolean outcomes.

use super::*;
use compact_rust_merkle_tree_oracle_fixture::types::MerkleTreeDigest;
use midnight_compact_runtime as runtime;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for expected in [true, false] {
        let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
        let blank = merkle_tree_view_at_path(initial.ledger_state.get_ref(), &[0])?
            .root()
            .ok_or("missing initial root")?;
        let seeded = merkle_contract::append(
            initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
            BoundedUint::<255>::new(7)?,
        )?;
        let state = seeded.context.query.state.get_ref().clone();
        let digest = MerkleTreeDigest {
            field: if expected {
                merkle_tree_view_at_path(&state, &[0])?
                    .root()
                    .ok_or("missing populated root")?
                    .0
            } else {
                blank.0
            },
        };
        let mut rng = StdRng::seed_from_u64(if expected { 0x0143_1001 } else { 0x0143_1000 });
        let deploy = make_deploy(root, "known", state.clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let recorded =
            merkle_contract::recorded::known(observed.circuit_context(()), digest.clone())?;
        if recorded.execution.result != expected
            || !recorded.execution.private_transcript_outputs.is_empty()
        {
            return Err("direct Merkle root check has unexpected result or private output".into());
        }
        let manual = check_generated_trace(root, "known", recorded, digest.clone())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/known.verifier"),
        )?))?;
        let typed = merkle_contract::recorded::Contract
            .known_call(&observed, (), digest)?
            .prepare(verifier, Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err("direct Merkle root observed call differs from recording".into());
        }
        check_transaction(root, "known", deploy, typed, &mut rng, |contract| {
            if contract.data.get_ref() != &state {
                return Err("direct root read changed ledger state".into());
            }
            Ok(())
        })?;
        println!("direct Merkle root check proved and applied (result={expected})");
    }
    Ok(())
}
