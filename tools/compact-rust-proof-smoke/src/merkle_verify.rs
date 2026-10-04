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

//! Focused source-to-proof-to-ledger check for witnessed Merkle root reads.

use super::*;
use compact_rust_merkle_path_verify_fixture::ledger_contract as merkle_verify_contract;
use compact_rust_merkle_path_verify_fixture::types::MerkleTreePath;
use midnight_compact_runtime::ledger::ContractAddress;

struct PathWitness;

impl merkle_verify_contract::Witnesses<()> for PathWitness {
    fn leaf_path(
        &self,
        context: WitnessContext<'_, (), merkle_verify_contract::LedgerView<'_>>,
    ) -> ((), MerkleTreePath) {
        let path = context
            .ledger
            .t()
            .expect("Merkle view")
            .path_for_leaf(0, BoundedUint::<255>::new(7).expect("Uint<8>"))
            .expect("Merkle path");
        (
            (),
            MerkleTreePath::from_ledger_path(path).expect("typed Merkle path"),
        )
    }
}

fn source_state(valid: bool) -> Result<StateValue<DefaultDB>, Box<dyn Error>> {
    let initial = merkle_verify_contract::initial_state(ConstructorContext::new(()))?;
    let context = initial.into_circuit_context(ContractAddress::default());
    let after_append = merkle_verify_contract::append(context, BoundedUint::<255>::new(7)?)?;
    let context = if valid {
        after_append.context
    } else {
        merkle_verify_contract::replace(after_append.context, BoundedUint::<255>::new(8)?)?.context
    };
    Ok(context.query.state.get_ref().clone())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for valid in [true, false] {
        let expected_state = source_state(valid)?;
        let mut rng = StdRng::seed_from_u64(if valid { 0x0083_1001 } else { 0x0083_1000 });
        let deploy = make_deploy(root, "verify", expected_state.clone(), &mut rng)?;
        let observed_state = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let witness = PathWitness;
        let recorded =
            merkle_verify_contract::recorded::verify(observed_state.circuit_context(()), &witness)?;
        if recorded.execution.result != valid {
            return Err(format!("Merkle verify result differs from expected {valid}").into());
        }
        let manual = check_generated_trace(root, "verify", recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/verify.verifier"),
        )?))?;
        let contract = merkle_verify_contract::Contract::from(PathWitness);
        let observed_call = contract.recording().verify_call(&observed_state, ())?;
        let observed = observed_call.prepare(verifier, Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{observed:?}") {
            return Err("Merkle observed call differs from recorded prototype".into());
        }
        check_transaction(root, "verify", deploy, observed, &mut rng, |contract| {
            if contract.data.get_ref() != &expected_state {
                return Err("read-only Merkle proof changed ledger state".into());
            }
            Ok(())
        })?;
        println!("witnessed Merkle checkRoot proved and applied (result={valid})");
    }
    Ok(())
}
