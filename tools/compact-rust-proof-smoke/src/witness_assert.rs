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

//! Proves an assertion guarded by an ordered private Boolean witness result.

use super::*;
use compact_rust_assert_witness_fixture::ledger_contract as contract;

struct Echo;

impl contract::Witnesses<u64> for Echo {
    fn echo(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
        flag: bool,
    ) -> (u64, bool) {
        (*context.private_state + 1, flag)
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0092_4153_5345_5254);
    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        "checked_write",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let generated = contract::Contract::from(Echo);

    let failed = generated.recording().checked_write(
        contract::initial_state(ConstructorContext::new(7_u64))?
            .into_circuit_context(deploy.address()),
        false,
        Field::from(42_u64),
    );
    if failed.err().map(|error| error.to_string()) != Some("failed assert: write denied".to_owned())
    {
        return Err("failed witness assertion did not stop recording".into());
    }

    let recorded = generated.recording().checked_write(
        initial.into_circuit_context(deploy.address()),
        true,
        Field::from(42_u64),
    )?;
    if recorded.execution.context.private_state != 8
        || recorded.execution.private_transcript_outputs.len() != 1
    {
        return Err("recorded witness assertion lost its private effects".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual =
        check_generated_trace(root, "checked_write", recorded, (true, Field::from(42_u64)))?;

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
        root.join("keys/checked_write.verifier"),
    )?))?;
    let typed =
        generated
            .recording()
            .checked_write_call(&observed, 7_u64, true, Field::from(42_u64))?;
    let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("observed witness assertion differs from direct recorded call".into());
    }

    check_transaction(root, "checked_write", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven assertion changed the expected Cell state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("assertion contract state is not an array".into());
        };
        let value: Field = read_cell(fields.get(0).ok_or("Cell missing")?)?;
        if value != Field::from(42_u64) {
            return Err("proven assertion did not write the expected Field".into());
        }
        Ok(())
    })?;
    println!("Boolean witness assertion proved and applied through ledger-8");
    Ok(())
}
