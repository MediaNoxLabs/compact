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

//! Prove each original identity-map write while preserving its constructor.
use super::*;

pub(super) fn run(identity: &Path, arithmetic_constructor: &Path) -> Result<(), Box<dyn Error>> {
    macro_rules! prove_map {
        ($fixture:ident, $root:expr, $seed:expr) => {{
            use $fixture::ledger_contract as contract;
            let root = $root;
            let mut rng = StdRng::seed_from_u64($seed);
            let initial = contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                root,
                "ping",
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let native = contract::ping(observed.circuit_context(()))?;
            let recorded = contract::recorded::ping(observed.circuit_context(()))?;
            if recorded.execution.context.query.state.get_ref()
                != native.context.query.state.get_ref()
                || !recorded.execution.private_transcript_outputs.is_empty()
            {
                return Err("identity map recording differs from native execution".into());
            }
            let manual = check_generated_trace(root, "ping", recorded, ())?;
            let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
                root.join("keys/ping.verifier"),
            )?))?;
            let typed = contract::recorded::Contract
                .ping_call(&observed, ())?
                .prepare(verifier, Fr::from(0u64))?;
            if format!("{manual:?}") != format!("{typed:?}") {
                return Err("identity map observed call differs from recording".into());
            }
            let expected = native.context.query.state.get_ref().clone();
            check_transaction(root, "ping", deploy, typed, &mut rng, |contract| {
                if contract.data.get_ref() != &expected {
                    return Err("identity map proof differs from native final state".into());
                }
                Ok(())
            })?;
        }};
    }
    prove_map!(compact_rust_map_fn_oracle_fixture, identity, 0x0149_0001);
    prove_map!(
        compact_rust_map_lambda_oracle_fixture,
        arithmetic_constructor,
        0x0149_0002
    );
    println!("both original identity-map calls proved and applied through ledger-8");
    Ok(())
}
