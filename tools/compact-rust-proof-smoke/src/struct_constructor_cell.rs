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

//! Prove both module-disambiguated struct constructor calls and their Field Cell writes.

use super::*;
use compact_rust_struct_collision_oracle_fixture::ledger_contract as contract;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0135_5354_5255_4354);
    for (circuit, value, path) in [
        ("runAlpha", Field::from(5_u64), &[0_u8][..]),
        ("runBeta", Field::from(7_u64), &[1_u8][..]),
    ] {
        let initial = contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let recorded = match circuit {
            "runAlpha" => {
                contract::recorded::runAlpha(initial.into_circuit_context(deploy.address()), value)?
            }
            "runBeta" => {
                contract::recorded::runBeta(initial.into_circuit_context(deploy.address()), value)?
            }
            _ => unreachable!(),
        };
        if !recorded.execution.private_transcript_outputs.is_empty() {
            return Err(format!("{circuit} unexpectedly emitted a private output").into());
        }
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, value)?;
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
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let typed = match circuit {
            "runAlpha" => contract::recorded::Contract
                .runAlpha_call(&observed, (), value)?
                .prepare(verifier, Fr::from(0_u64))?,
            "runBeta" => contract::recorded::Contract
                .runBeta_call(&observed, (), value)?
                .prepare(verifier, Fr::from(0_u64))?,
            _ => unreachable!(),
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed call differs from direct recording").into());
        }
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err(format!("{circuit} proven state differs from recording").into());
            }
            if read_cell_at_path::<Field, _>(state.data.get_ref(), path)? != value {
                return Err(format!("{circuit} stored a different Field").into());
            }
            Ok(())
        })?;
        println!("typed {circuit} proved and applied through ledger-8");
    }
    Ok(())
}
