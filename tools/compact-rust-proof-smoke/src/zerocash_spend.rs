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

use super::*;
use compact_rust_zerocash_oracle_fixture::ledger_contract as contract;
#[path = "../../../tests-rust-backend/zerocash-oracle/support.rs"]
mod support;
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for historical in [false, true] {
        let (seeded, path) = support::seeded(historical)?;
        let private = seeded.private_state.clone();
        let mut rng = StdRng::seed_from_u64(0x167 + u64::from(historical));
        let deploy = make_deploy(
            root,
            "spend",
            seeded.query.state.get_ref().clone(),
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
        let make_witness = || support::Witness {
            path: historical.then_some(path.clone()),
            ..Default::default()
        };
        let native = contract::spend(
            observed.circuit_context(private.clone()),
            &make_witness(),
            support::destination(),
            support::coin(3),
        )?;
        let witness = make_witness();
        let recorded = contract::recorded::spend(
            observed.circuit_context(private.clone()),
            &witness,
            support::destination(),
            support::coin(3),
        )?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || recorded.execution.private_transcript_outputs.len() != 5
            || native.context.private_state != recorded.execution.context.private_state
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
            || *witness.calls.borrow() != ["secret", "path", "new_coin", "encrypt", "remove_coin"]
        {
            return Err("spend native/recorded mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(
            root,
            "spend",
            recorded,
            (support::destination(), support::coin(3)),
        )?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/spend.verifier"),
        )?))?;
        let generated = contract::Contract::from(make_witness());
        let prepared = generated
            .recording()
            .spend_call(&observed, private, support::destination(), support::coin(3))?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("spend observed-call mismatch".into());
        }
        check_transaction(root, "spend", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected {
                return Err("applied spend state mismatch".into());
            }
            let view = contract::PublicStateView::from(applied);
            if view.ciphertexts()?
                != support::ciphertext(
                    &support::destination().encryption,
                    &support::coin(if historical { 7 } else { 5 }),
                )
                || view.commitments()?.first_free()?.value() != if historical { 3 } else { 2 }
            {
                return Err("recipient ciphertext/commitment mismatch".into());
            }
            Ok(())
        })?;
        println!("Zerocash spend historical={historical} verified and ledger-applied");
    }
    Ok(())
}
