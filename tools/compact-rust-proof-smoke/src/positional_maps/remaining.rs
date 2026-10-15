// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound proofs for Map metadata, default overwrite and populated reset.
use crate::*;
use compact_rust_map_boolean_field_fixture::{ledger_contract as c, ledger_slots as slots};
use midnight_compact_runtime::{
    context::CircuitResult, ledger::ContractAddress, recording::RecordingFrame,
    transaction::RecordedCall,
};

const RETAINED_IR: &str =
    include_str!("../../../compact-rust-backend/tests/map-boolean-field-schema20-ir.json");

struct Scenario<'a> {
    root: &'a Path,
    name: &'static str,
    deploy: ContractDeploy<DefaultDB>,
    rng: StdRng,
    verifier: VerifierKey,
}

impl<'a> Scenario<'a> {
    fn new(
        root: &'a Path,
        name: &'static str,
    ) -> Result<(Self, ObservedContractState), Box<dyn Error>> {
        let mut rng = StdRng::seed_from_u64(0x0609_4d41_5053);
        let initial = c::initial_state(ConstructorContext::new(7_u64))?;
        let seed = initial.into_circuit_context(ContractAddress::default());
        let seed = slots::table
            .insert(seed, true, Field::from(41_u64))?
            .context;
        let seed = slots::table
            .insert(seed, false, Field::from(17_u64))?
            .context;
        let deploy = make_deploy(root, name, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        Ok((
            Self {
                root,
                name,
                deploy,
                rng,
                verifier,
            },
            observed,
        ))
    }

    fn finish<T: PartialEq + Clone + Into<AlignedValue>>(
        self,
        native: CircuitResult<u64, T>,
        recorded: RecordedCircuitResult<u64, T>,
        manual: RecordedCircuitResult<u64, T>,
        call: RecordedCall<'_, u64, T>,
        input: AlignedValue,
        expected: T,
    ) -> Result<(), Box<dyn Error>> {
        if native.result != expected
            || native.context.private_state != 7
            || !native.private_transcript_outputs.is_empty()
        {
            return Err("Map result/private data differs from the supplied scenario".into());
        }
        let manual_replay = manual.public.initial().query(
            manual.public.verify_ops(),
            None,
            &native.context.cost_model,
        )?;
        for actual in [&recorded, call.recorded(), &manual] {
            if actual.execution.result != expected
                || actual.execution.context.query.state != native.context.query.state
                || actual.execution.context.query.effects != native.context.query.effects
                || actual.execution.gas_cost != native.gas_cost
                || actual.execution.context.private_state != 7
                || !actual.execution.private_transcript_outputs.is_empty()
                || actual.public.verify_ops() != manual.public.verify_ops()
            {
                return Err("Map native/recorded/observed/manual execution differs".into());
            }
            let replay = actual.public.initial().query(
                actual.public.verify_ops(),
                None,
                &native.context.cost_model,
            )?;
            if replay.context.state != native.context.query.state
                || replay.context.effects != native.context.query.effects
                || replay.gas_cost != manual_replay.gas_cost
            {
                return Err("Map public replay differs from the manual slot program".into());
            }
        }
        let expected_state = native.context.query.state.get_ref().clone();
        check_state(self.name, &expected_state)?;
        let prepared = call.prepare(self.verifier.clone(), Fr::from(0_u64))?;
        if prepared.input != input || prepared.output != expected.into() {
            return Err("Map prepared public input/output differs".into());
        }
        let direct = check_generated_trace(self.root, self.name, recorded, input)?;
        if format!("{direct:?}") != format!("{prepared:?}") {
            return Err("Map direct/observed preparation differs".into());
        }
        kernel_shielded_effects::prove_and_verify_call(
            self.root,
            self.name,
            &prepared,
            &self.verifier,
        )?;
        opaque_public::apply_funded(
            self.root,
            self.name,
            prepared,
            self.deploy,
            &expected_state,
            self.rng,
            609,
        )?;
        println!(
            "{}: native/recorded/observed/manual agreement, proof and changed-binding refusal, strict funded ledger8 apply; constructor and populated seed supplied, not proved",
            self.name
        );
        Ok(())
    }
}

fn check_state(name: &str, state: &StateValue<DefaultDB>) -> Result<(), Box<dyn Error>> {
    let map = slots::table.inspect(state)?;
    if name == "reset_table" {
        if !map.is_empty() || map.size()?.value() != 0 {
            return Err("Map reset did not clear the populated map".into());
        }
    } else {
        let expected_true = if name == "put_default" { 0 } else { 41 };
        if map.size()?.value() != 2
            || map.lookup(true)? != Field::from(expected_true as u64)
            || map.lookup(false)? != Field::from(17_u64)
        {
            return Err("Map default overwrite/metadata changed unexpected entries".into());
        }
    }
    Ok(())
}

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let generated = c::Contract::default();
    for name in ["put_default", "table_size", "table_is_empty", "reset_table"] {
        let (scenario, observed) = Scenario::new(root, name)?;
        let frame = RecordingFrame::new(observed.circuit_context(7_u64));
        match name {
            "put_default" => scenario.finish(
                c::put_default(observed.circuit_context(7), true)?,
                c::recorded::put_default(observed.circuit_context(7), true)?,
                slots::table.record_insert_default(frame, true)?.finish(()),
                generated.recording().put_default_call(&observed, 7, true)?,
                true.into(),
                (),
            )?,
            "table_size" => {
                let (frame, size) = slots::table.record_size(frame)?;
                scenario.finish(
                    c::table_size(observed.circuit_context(7))?,
                    c::recorded::table_size(observed.circuit_context(7))?,
                    frame.finish(BoundedUint::<18446744073709551615>::new(size as u128)?),
                    generated.recording().table_size_call(&observed, 7)?,
                    ().into(),
                    BoundedUint::<18446744073709551615>::new(2)?,
                )?;
            }
            "table_is_empty" => {
                let (frame, empty) = slots::table.record_is_empty(frame)?;
                scenario.finish(
                    c::table_is_empty(observed.circuit_context(7))?,
                    c::recorded::table_is_empty(observed.circuit_context(7))?,
                    frame.finish(empty),
                    generated.recording().table_is_empty_call(&observed, 7)?,
                    ().into(),
                    false,
                )?;
            }
            "reset_table" => scenario.finish(
                c::reset_table(observed.circuit_context(7))?,
                c::recorded::reset_table(observed.circuit_context(7))?,
                slots::table.record_reset(frame)?.finish(()),
                generated.recording().reset_table_call(&observed, 7)?,
                ().into(),
                (),
            )?,
            _ => unreachable!(),
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "remaining_tests.rs"]
mod tests;
