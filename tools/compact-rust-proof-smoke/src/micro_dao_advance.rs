// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original advance branches from explicit prior states with separate Dust fees.
use super::micro_dao_advance_support as support;
use super::*;
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (i, phase) in [
        types::LedgerState::commit,
        types::LedgerState::reveal,
        types::LedgerState::r#final,
    ]
    .into_iter()
    .enumerate()
    {
        let seeded = support::seeded(phase, 2, 3, 7, false, true)?;
        let private = seeded.private_state.clone();
        let mut rng = StdRng::seed_from_u64(0x0194_0000 + i as u64);
        let deploy = make_deploy(
            root,
            "advance",
            seeded.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let address = deploy.address();
        // Explicit fixture state, not proof of prior DAO setup/deposit/vote calls.
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
        let native = c::advance(
            observed.circuit_context(private.clone()),
            &support::Witness::new(support::Mode::Normal),
        )?;
        let witness = support::Witness::new(support::Mode::Normal);
        let recorded = c::recorded::advance(observed.circuit_context(private.clone()), &witness)?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || native.context.query.state != recorded.execution.context.query.state
            || native.context.query.effects != recorded.execution.context.query.effects
            || recorded.execution.private_transcript_outputs.len() != 1
            || *witness.calls.borrow() != ["secret"]
        {
            return Err("original advance native/recorded mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "advance", recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/advance.verifier"),
        )?))?;
        let generated = c::Contract::from(support::Witness::new(support::Mode::Normal));
        let prepared = generated
            .recording()
            .advance_call(&observed, private)?
            .prepare(verifier.clone(), Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("advance typed/direct preparation differs".into());
        }
        super::kernel_shielded_effects::prove_and_verify_call(
            root, "advance", &prepared, &verifier,
        )?;
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
                .add_call::<ProofPreimage>(prepared);
        let tx = Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
        let resolver = super::qualified_coin_funding::fee_resolver(root, "advance")?;
        let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x0194_5052 + i as u64),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed = proven.seal(StdRng::seed_from_u64(0x0194_5345 + i as u64));
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
            return Err(format!("advance {phase:?} application failed: {result:?}").into());
        }
        let actual = updated
            .contract
            .get(&address)
            .ok_or("advance contract missing")?;
        if actual.data.get_ref() != &expected
            || slots::pot.inspect(actual.data.get_ref())? != support::pot()
            || !slots::pot_has_coin.inspect(actual.data.get_ref())?
        {
            return Err("advance applied state/pot differs".into());
        }
        println!(
            "original microDAO.advance {phase:?}: cryptographic proof verified, changed binding rejected, separate Dust default-strict ledger apply passed; explicit prior state, no funded DAO lifecycle claim"
        );
    }
    Ok(())
}
