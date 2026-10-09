// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Exact-source ordinary arithmetic, with selected and skipped witness effects.
use super::*;
use compact_rust_effect_plan_unsigned_arithmetic_source_fixture as source;
use source::{
    ledger_contract as contract,
    support::{U8, Witness},
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        source::RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0541_4152_4954_484d);
    for selected in [false, true] {
        let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
        let deploy = make_deploy(
            root,
            "run",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let (left, right) = (U8::new(255)?, U8::new(254)?);
        let witness = Witness::default();
        let recorded = contract::recorded::run(
            initial.into_circuit_context(deploy.address()),
            &witness,
            left,
            right,
            selected,
        )?;
        let expected_product = if selected { 255 * 254 } else { 0 };
        let expected_witnesses = if selected { 4 } else { 2 };
        if recorded.execution.result.difference.value() != 1
            || recorded.execution.result.product.value() != expected_product
            || recorded.execution.private_transcript_outputs.len() != expected_witnesses
            || recorded.execution.context.private_state != 7 + expected_witnesses as u64
        {
            return Err("arithmetic result or selected witness effects changed".into());
        }
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "run", recorded, (left, right, selected))?;
        let observed = pure_field_arguments::observed_state(&deploy);
        let generated = contract::Contract::from(Witness::default());
        let typed = generated
            .recording()
            .run_call(&observed, 7_u64, left, right, selected)?;
        let prepared = typed.prepare(
            pure_field_arguments::verifier(root, "run")?,
            Fr::from(0_u64),
        )?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("arithmetic direct/observed preparation differs".into());
        }
        check_transaction(root, "run", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("arithmetic proof changed ledger state".into());
            }
            if read_cell_at_path::<U8, _>(state.data.get_ref(), &[0])?.value() != 1
                || read_cell_at_path::<BoundedUint<65535>, _>(state.data.get_ref(), &[1])?.value()
                    != expected_product
            {
                return Err("arithmetic proof applied unexpected numeric values".into());
            }
            Ok(())
        })?;
    }
    println!(
        "ordinary arithmetic: selected/skipped witness branches proved and applied through ledger8"
    );
    Ok(())
}
