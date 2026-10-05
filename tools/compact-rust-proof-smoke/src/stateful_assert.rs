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

//! Prove typed short-circuit assertions with Unit and Uint64 results.
use super::*;
use compact_rust_stateful_assert_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime::BoundedUint;
struct Witness;
impl c::Witnesses<Vec<u8>> for Witness {
    fn next_gate(
        &self,
        context: WitnessContext<'_, Vec<u8>, c::LedgerView<'_>>,
        tag: BoundedUint<255>,
    ) -> (Vec<u8>, bool) {
        let mut state = context.private_state.clone();
        state.push(tag.value() as u8);
        (state, true)
    }
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, name) in ["checked", "unit_result"].into_iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(0x0185_0000 + index as u64);
        let initial = c::initial_state(
            ConstructorContext::new(Vec::<u8>::new()),
            true,
            BoundedUint::new(3)?,
            BoundedUint::new(7)?,
        )?;
        let expected_state = initial.ledger_state.get_ref().clone();
        let deploy = make_deploy(root, name, expected_state.clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let contract = c::Contract::from(Witness);
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        let (manual, prepared) = if name == "checked" {
            let recorded = c::recorded::checked(observed.circuit_context(vec![]), &Witness, true)?;
            if recorded.execution.result.value() != 7
                || recorded.execution.context.private_state != [1, 2, 3]
            {
                return Err("checked assertion result or witness order changed".into());
            }
            if !matches!(
                c::recorded::checked(observed.circuit_context(vec![]), &Witness, false),
                Err(midnight_compact_runtime::CompactError::AssertionFailed(_))
            ) {
                return Err("checked false guard did not reject".into());
            }
            let manual = check_generated_trace(root, name, recorded, true)?;
            let typed = contract.recording().checked_call(&observed, vec![], true)?;
            (manual, typed.prepare(verifier, Fr::from(0_u64))?)
        } else {
            let recorded =
                c::recorded::unit_result(observed.circuit_context(vec![]), &Witness, true)?;
            if recorded.execution.context.private_state != [1] {
                return Err("unit assertion witness order changed".into());
            }
            if !matches!(
                c::recorded::unit_result(observed.circuit_context(vec![]), &Witness, false),
                Err(midnight_compact_runtime::CompactError::AssertionFailed(_))
            ) {
                return Err("unit false guard did not reject".into());
            }
            let manual = check_generated_trace(root, name, recorded, true)?;
            let typed = contract
                .recording()
                .unit_result_call(&observed, vec![], true)?;
            (manual, typed.prepare(verifier, Fr::from(0_u64))?)
        };
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("assertion observed call differs from direct recording".into());
        }
        check_transaction(root, name, deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("read-only assertion changed ledger state".into());
            }
            Ok(())
        })?;
        println!(
            "{name} assertion proved, verified and ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}
