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

//! Prove the original qualified Set lifecycle. The separate insertCoin export
//! hardcodes two distinct commitments at index zero and is only a source-level
//! mock-context parity case, not a valid transaction.

use super::*;
use compact_rust_adt_set_qualified_coin_info_fixture::ledger_contract as contract;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let name = "test_QualifiedShieldedCoinInfo";
    let mut rng = StdRng::seed_from_u64(0x0170_5153);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let native = contract::test_QualifiedShieldedCoinInfo(observed.circuit_context(()))?;
    let recorded =
        contract::recorded::test_QualifiedShieldedCoinInfo(observed.circuit_context(()))?;
    if native.context.query.state.get_ref() != recorded.execution.context.query.state.get_ref()
        || native.context.query.effects != recorded.execution.context.query.effects
        || native.gas_cost != recorded.execution.gas_cost
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("qualified Set lifecycle native and recorded differ".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, name, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{name}.verifier")),
    )?))?;
    let typed = contract::Contract::default()
        .recording()
        .test_QualifiedShieldedCoinInfo_call(&observed, ())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("qualified Set observed call differs from manual trace".into());
    }
    check_transaction(root, name, deploy, typed, &mut rng, |applied| {
        if applied.data.get_ref() != &expected_state {
            return Err("qualified Set applied state differs".into());
        }
        Ok(())
    })?;
    println!("original qualified Set lifecycle proof verified and ledger-8 applied");
    Ok(())
}
