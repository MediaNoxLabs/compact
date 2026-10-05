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
//! Read-only composites: real nonempty proofs and exact empty-call refusal.
use super::*;
use compact_rust_field_observation_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime as runtime;
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, (name, selected)) in [
        ("direct", false),
        ("snapshot", false),
        ("via_field_helper", false),
        ("via_snapshot_helper", false),
        ("pair", false),
        ("selected", false),
        ("selected", true),
        ("optional", true),
    ]
    .into_iter()
    .enumerate()
    {
        let mut rng = StdRng::seed_from_u64(0x0201_0000 + index as u64);
        let initial = c::initial_state(
            ConstructorContext::new(Vec::<u8>::new()),
            runtime::Field::from(7u64),
            runtime::Field::from(19u64),
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
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        macro_rules! calls { ($circuit:ident, $call:ident, $input:expr $(,$arg:expr)*) => {{
            let recorded = c::recorded::$circuit(observed.circuit_context(Vec::<u8>::new()) $(,$arg)*)?;
            let manual = check_generated_trace(root, name, recorded, $input)?;
            let typed = c::Contract::default().recording.$call(&observed, Vec::<u8>::new() $(,$arg)*)?;
            (manual, typed.prepare(verifier.clone(), Fr::from(0u64))?)
        }}; }
        let (manual, prepared) = match name {
            "direct" => calls!(direct, direct_call, ()),
            "snapshot" => calls!(snapshot, snapshot_call, ()),
            "via_field_helper" => calls!(via_field_helper, via_field_helper_call, ()),
            "via_snapshot_helper" => calls!(via_snapshot_helper, via_snapshot_helper_call, ()),
            "pair" => calls!(pair, pair_call, ()),
            "selected" => calls!(selected, selected_call, selected, selected),
            "optional" => {
                let empty = c::Contract::default().recording.optional_call(
                    &observed,
                    Vec::<u8>::new(),
                    false,
                )?;
                if !matches!(
                    empty.prepare(verifier.clone(), Fr::from(0u64)),
                    Err(ObservedCallError::Prepare(
                        runtime::transaction::PrepareCallError::EmptyTranscript
                    ))
                ) {
                    return Err("optional(false) must retain exact empty-transcript refusal".into());
                }
                println!("optional(false): exact EmptyTranscript, no fabricated operation");
                calls!(optional, optional_call, true, true)
            }
            _ => unreachable!(),
        };
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("manual/observed preparation differs".into());
        }
        super::kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
        // Explicit prior contract state; fee funding is separate and does not
        // fabricate a ledger query or an application-level shielded offer.
        let address = deploy.address();
        let mut funded = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
        funded.ledger.contract = funded.ledger.contract.insert(address, deploy.initial_state);
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut rng, Timestamp::from_secs(funded.time.to_secs() + 3600))
                .add_call::<ProofPreimage>(prepared);
        let tx = Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
        let resolver = super::qualified_coin_funding::fee_resolver(root, name)?;
        let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x0201_5052 + index as u64),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed = proven.seal(StdRng::seed_from_u64(0x0201_5345 + index as u64));
        let sealed = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(funded.balance_tx(rng, sealed, &resolver))?;
        let verified =
            sealed.well_formed(&funded.ledger, WellFormedStrictness::default(), funded.time)?;
        let context = TransactionContext {
            ref_state: funded.ledger.clone(),
            block_context: BlockContext {
                tblock: funded.time,
                last_block_time: funded.time,
                ..BlockContext::default()
            },
            whitelist: None,
        };
        let (updated, result) = funded.ledger.apply(&verified, &context);
        if !matches!(result, TransactionResult::Success(_)) {
            return Err(format!("{name} application failed: {result:?}").into());
        }
        let actual = updated
            .contract
            .get(&address)
            .ok_or("observation contract missing")?;
        if actual.data.get_ref() != &expected_state {
            return Err("read-only observation changed ledger state".into());
        }
        println!(
            "{name}({selected}): proof verified, changed binding rejected, separate Dust default-strict ledger apply passed"
        );
    }
    Ok(())
}
