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
//! Extracted lexical results with strict ledger application and separate Dust.
use super::terminal_lexical_return_support as support;
use super::*;
use compact_rust_terminal_lexical_return_oracle_fixture::{
    ledger_contract as c, ledger_slots as slots,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (i, name) in ["two", "three", "nested", "observed"]
        .into_iter()
        .enumerate()
    {
        // Native echo calls seed the source's Cell. These are prior local state,
        // not a claim of proved or funded earlier ledger transactions.
        let seeded = support::seeded(4);
        let private = seeded.private_state.clone();
        let mut rng = StdRng::seed_from_u64(0x0202_0000 + i as u64);
        let deploy = make_deploy(root, name, seeded.query.state.get_ref().clone(), &mut rng)?;
        let address = deploy.address();
        let mut state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
        state.ledger.contract = state
            .ledger
            .contract
            .insert(address, deploy.initial_state.clone());
        let observed = ObservedContractState::new(
            address,
            deploy.initial_state,
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native_witness = support::Witnesses::new(4, 2);
        let ctx = observed.circuit_context(private.clone());
        let native = match name {
            "two" => c::two(ctx),
            "three" => c::three(ctx),
            "nested" => c::nested(ctx),
            "observed" => c::observed(ctx, &native_witness, false),
            _ => unreachable!(),
        }?;
        let witness = support::Witnesses::new(4, 2);
        let ctx = observed.circuit_context(private.clone());
        let recorded = match name {
            "two" => c::recorded::two(ctx),
            "three" => c::recorded::three(ctx),
            "nested" => c::recorded::nested(ctx),
            "observed" => c::recorded::observed(ctx, &witness, false),
            _ => unreachable!(),
        }?;
        let expected_result = Field::from(match name {
            "three" => 7u64,
            "observed" => 6,
            _ => 5,
        });
        if native.result != expected_result
            || recorded.execution.result != expected_result
            || native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || native.context.query.state != recorded.execution.context.query.state
            || native.context.query.effects != recorded.execution.context.query.effects
            || *native_witness.trace.borrow() != *witness.trace.borrow()
            || witness.trace.borrow().len() != usize::from(name == "observed")
        {
            return Err(format!("{name}: native/recorded lexical result or effects differ").into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let input = if name == "observed" {
            AlignedValue::from(false)
        } else {
            AlignedValue::from(())
        };
        let manual = check_generated_trace(root, name, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        let generated = c::Contract::from(support::Witnesses::new(4, 2));
        let call = match name {
            "two" => generated.recording().two_call(&observed, private)?,
            "three" => generated.recording().three_call(&observed, private)?,
            "nested" => generated.recording().nested_call(&observed, private)?,
            "observed" => generated
                .recording()
                .observed_call(&observed, private, false)?,
            _ => unreachable!(),
        };
        let prepared = call.prepare(verifier.clone(), Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{prepared:?}")
            || prepared.output != AlignedValue::from(expected_result)
        {
            return Err(
                format!("{name}: typed/direct preparation or returned output differs").into(),
            );
        }
        super::kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
                .add_call::<ProofPreimage>(prepared);
        let tx = Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
        let resolver = super::qualified_coin_funding::fee_resolver(root, name)?;
        let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x0202_5052 + i as u64),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed = proven.seal(StdRng::seed_from_u64(0x0202_5345 + i as u64));
        let sealed = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(state.balance_tx(rng, sealed, &resolver))?;
        let verified =
            sealed.well_formed(&state.ledger, WellFormedStrictness::default(), state.time)?;
        let context = TransactionContext {
            ref_state: state.ledger.clone(),
            block_context: BlockContext {
                tblock: state.time,
                last_block_time: state.time,
                ..BlockContext::default()
            },
            whitelist: None,
        };
        let (updated, result) = state.ledger.apply(&verified, &context);
        if !matches!(result, TransactionResult::Success(_)) {
            return Err(format!("{name} application failed: {result:?}").into());
        }
        let actual = updated
            .contract
            .get(&address)
            .ok_or("terminal return contract missing")?;
        if actual.data.get_ref() != &expected
            || slots::stored.inspect(actual.data.get_ref())? != expected_result
        {
            return Err(format!("{name}: applied state or Field value differs").into());
        }
        println!(
            "terminal {name}: exact returned Field proved, changed binding rejected, separate Dust default-strict ledger apply passed; native-seeded prior state"
        );
    }
    Ok(())
}
