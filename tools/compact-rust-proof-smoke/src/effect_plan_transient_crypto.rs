// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound official crypto with a conditional private opening.
use super::*;
use compact_rust_effect_plan_transient_crypto_source_fixture as source;
use source::{ledger_contract as contract, runtime as r, support::Witness};
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        source::RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0543_4352_5950_544f);
    for selected in [false, true] {
        let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
        let deploy = make_deploy(
            root,
            "run",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let input = r::FixedBytes::new([255; 32]);
        let seed = Field::from(19_u64);
        let witness = Witness::default();
        let recorded = contract::recorded::run(
            initial.into_circuit_context(deploy.address()),
            &witness,
            input,
            seed,
            selected,
        )?;
        let hash = r::transient_hash((r::degrade_to_transient(input), seed));
        let expected = if selected {
            r::transient_commit(hash, seed + Field::from(1_u64))
        } else {
            hash
        };
        if recorded.execution.result.hash != hash
            || recorded.execution.result.commitment != expected
            || recorded.execution.result.digest != r::upgrade_from_transient(expected)
            || recorded.execution.private_transcript_outputs.len() != usize::from(selected)
            || recorded.execution.context.private_state != 7 + u64::from(selected)
        {
            return Err("transient crypto values or selected witness effects changed".into());
        }
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "run", recorded, (input, seed, selected))?;
        let observed = pure_field_arguments::observed_state(&deploy);
        let generated = contract::Contract::from(Witness::default());
        let typed = generated
            .recording()
            .run_call(&observed, 7_u64, input, seed, selected)?;
        let prepared = typed.prepare(
            pure_field_arguments::verifier(root, "run")?,
            Fr::from(0_u64),
        )?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("transient crypto observed/direct preparation differs".into());
        }
        check_transaction(root, "run", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state
                || read_cell_at_path::<r::FixedBytes<32>, _>(state.data.get_ref(), &[0])? != input
                || read_cell_at_path::<Field, _>(state.data.get_ref(), &[1])? != expected
            {
                return Err("transient crypto proof applied unexpected state".into());
            }
            Ok(())
        })?;
    }
    println!(
        "transient crypto selected/skipped witness branches proved and applied through ledger8"
    );
    Ok(())
}
