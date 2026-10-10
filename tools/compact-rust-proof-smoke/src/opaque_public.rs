// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Exact-source opaque interfaces: checked preparation and funded strict application.
use super::*;
use compact_rust_effect_plan_opaque_public_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime::{self as runtime, context::CircuitResult};
use std::{cell::Cell, rc::Rc};

const TEXT: &str = "é文🦀\0tail";
const INPUT_BYTES: &[u8] = &[1, 0, 255];
const REPLACEMENT_BYTES: &[u8] = &[9, 0, 8];

#[derive(Clone, Default)]
struct Witness(Rc<Cell<usize>>);
impl c::TryWitnesses<u64> for Witness {
    fn replacement(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, runtime::OpaqueBytes), runtime::CompactError> {
        assert_eq!(*context.private_state, 5);
        assert_eq!(context.ledger.calls()?.value(), 1);
        self.0.set(self.0.get() + 1);
        Ok((6, REPLACEMENT_BYTES.to_vec().into()))
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Complete input identity before key loading, state initialization or funding.
    // This guard is not authentication of proving-key provenance.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    for (index, name) in ["run", "text_roundtrip"].into_iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(0x0555_0000 + index as u64);
        let initial = c::initial_state(ConstructorContext::new(5_u64))?;
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
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        let (prepared, expected_state) = if name == "run" {
            let input = types::Envelope {
                message: types::Message {
                    text: TEXT.into(),
                    bytes: INPUT_BYTES.to_vec().into(),
                },
            };
            let (native_witness, recorded_witness, observed_witness) =
                (Witness::default(), Witness::default(), Witness::default());
            let native = c::run(
                observed.circuit_context(5),
                &native_witness,
                input.clone(),
                true,
            )?;
            let recorded = c::recorded::run(
                observed.circuit_context(5),
                &recorded_witness,
                input.clone(),
                true,
            )?;
            let generated = c::Contract::from(observed_witness.clone());
            let call = generated
                .recording()
                .run_call(&observed, 5, input.clone(), true)?;
            if native.result.original != input
                || native.result.chosen.message.text.0 != TEXT
                || native.result.chosen.message.bytes.0 != REPLACEMENT_BYTES
                || call.recorded().execution.result != native.result
            {
                return Err("opaque nested result differs from raw expected values".into());
            }
            // The witness checks source order and its private transition itself.
            // Each independently executed native/recorded call invokes it once.
            if native_witness.0.get() != 1
                || recorded_witness.0.get() != 1
                || observed_witness.0.get() != 1
            {
                return Err("opaque witness was not evaluated exactly once".into());
            }
            let prepared = call.prepare(verifier.clone(), Fr::from(0u64))?;
            require_atoms(&prepared.input, &[TEXT.as_bytes(), INPUT_BYTES, &[1]])?;
            require_atoms(
                &prepared.output,
                &[
                    TEXT.as_bytes(),
                    REPLACEMENT_BYTES,
                    TEXT.as_bytes(),
                    INPUT_BYTES,
                ],
            )?;
            compare_preparation(root, name, native, recorded, (input, true), prepared, 6)?
        } else {
            let native = c::text_roundtrip(observed.circuit_context(5), TEXT.into())?;
            let recorded = c::recorded::text_roundtrip(observed.circuit_context(5), TEXT.into())?;
            let call = c::recorded::Contract.text_roundtrip_call(&observed, 5, TEXT.into())?;
            if native.result.value.0 != TEXT || call.recorded().execution.result != native.result {
                return Err(
                    "opaque text result differs from raw Unicode/internal-zero value".into(),
                );
            }
            let prepared = call.prepare(verifier.clone(), Fr::from(0u64))?;
            require_atoms(&prepared.input, &[TEXT.as_bytes()])?;
            require_atoms(&prepared.output, &[TEXT.as_bytes()])?;
            compare_preparation(
                root,
                name,
                native,
                recorded,
                runtime::OpaqueString::from(TEXT),
                prepared,
                5,
            )?
        };
        kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
        apply_funded(
            root,
            name,
            prepared,
            deploy,
            &expected_state,
            rng,
            index as u64,
        )?;
        println!(
            "{name}: exact opaque atoms, native/recorded state, call proof and changed-binding refusal, funded default-strict application passed"
        );
    }
    Ok(())
}

fn require_atoms(value: &AlignedValue, expected: &[&[u8]]) -> Result<(), Box<dyn Error>> {
    let actual = value
        .value
        .0
        .iter()
        .map(|atom| atom.0.as_slice())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err("opaque public atoms differ from raw canonical expected bytes".into());
    }
    Ok(())
}

fn compare_preparation<Output: Into<AlignedValue> + PartialEq, Input: Into<AlignedValue>>(
    root: &Path,
    name: &'static str,
    native: CircuitResult<u64, Output>,
    recorded: RecordedCircuitResult<u64, Output>,
    input: Input,
    prepared: ContractCallPrototype<DefaultDB>,
    expected_private: u64,
) -> Result<(ContractCallPrototype<DefaultDB>, StateValue<DefaultDB>), Box<dyn Error>> {
    if native.result != recorded.execution.result
        || native.context.query.state != recorded.execution.context.query.state
        || native.context.query.effects != recorded.execution.context.query.effects
        || native.gas_cost != recorded.execution.gas_cost
        || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        || native.context.private_state != expected_private
        || recorded.execution.context.private_state != expected_private
        || slots::calls.inspect(native.context.query.state.get_ref())? != 1
        || recorded.public.verify_ops().is_empty()
    {
        return Err("opaque native/recorded execution mismatch".into());
    }
    let state = native.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, name, recorded, input)?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("opaque manual/observed preparation mismatch".into());
    }
    Ok((prepared, state))
}

fn apply_funded(
    root: &Path,
    name: &'static str,
    prepared: ContractCallPrototype<DefaultDB>,
    deploy: ContractDeploy<DefaultDB>,
    expected_state: &StateValue<DefaultDB>,
    mut rng: StdRng,
    index: u64,
) -> Result<(), Box<dyn Error>> {
    let address = deploy.address();
    let mut funded = qualified_coin_funding::fee_funded_state(&mut rng)?;
    funded.ledger.contract = funded.ledger.contract.insert(address, deploy.initial_state);
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(funded.time.to_secs() + 3600))
            .add_call::<ProofPreimage>(prepared);
    let tx = Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
    let resolver = qualified_coin_funding::fee_resolver(root, name)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0555_5052 + index),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0555_5345 + index));
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
        .ok_or("opaque contract missing after apply")?;
    if actual.data.get_ref() != expected_state {
        return Err("opaque applied state differs from native/recorded state".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_ir_change_is_rejected() {
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["contractName"] = serde_json::json!("changed-independent-guard-control");
        let error =
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .unwrap_err();
        assert!(error.to_string().contains("IR differs"));
    }

    #[test]
    fn absent_ir_refuses_before_keys_or_state_work() {
        let root = std::env::temp_dir().join(format!(
            "compact-opaque-missing-ir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        assert!(!root.exists());
        let error = run(&root).unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
