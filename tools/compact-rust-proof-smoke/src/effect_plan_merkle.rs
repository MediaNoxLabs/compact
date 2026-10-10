// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound composition of plain/historic insertions and root observations.
use super::*;
use compact_rust_effect_plan_merkle_source_fixture as source;
use source::{ledger_contract as contract, runtime as r, types};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        source::RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0545_4d45_524b_4c45);
    for selected in [false, true] {
        let initial = contract::initial_state(ConstructorContext::new(()))?;
        let blank = merkle_tree_view_at_path(initial.ledger_state.get_ref(), &[0])?
            .root()
            .ok_or("missing initial plain Merkle root")?;
        let digest = types::MerkleTreeDigest { field: blank.0 };
        let leaf = r::BoundedUint::<255>::new(7)?;
        let deploy = make_deploy(
            root,
            "run",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let recorded = contract::recorded::run(
            initial.into_circuit_context(deploy.address()),
            leaf,
            digest.clone(),
            selected,
        )?;
        let native = contract::run(
            contract::initial_state(ConstructorContext::new(()))?
                .into_circuit_context(deploy.address()),
            leaf,
            digest.clone(),
            selected,
        )?;
        if recorded.execution.result != native.result
            || recorded.execution.context.query.state != native.context.query.state
            || recorded.execution.context.query.effects != native.context.query.effects
            || recorded.execution.gas_cost != native.gas_cost
            || !recorded.execution.private_transcript_outputs.is_empty()
        {
            return Err("Merkle composition native/recorded observations differ".into());
        }
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual =
            check_generated_trace(root, "run", recorded, (leaf, digest.clone(), selected))?;
        let observed = pure_field_arguments::observed_state(&deploy);
        let prepared = contract::recorded::Contract
            .run_call(&observed, (), leaf, digest, selected)?
            .prepare(
                pure_field_arguments::verifier(root, "run")?,
                Fr::from(0_u64),
            )?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("Merkle observed/direct preparation differs".into());
        }
        check_transaction(root, "run", deploy, prepared, &mut rng, |state| {
            let plain = merkle_tree_view_at_path(state.data.get_ref(), &[0])?;
            let historic = historic_merkle_tree_view_at_path(state.data.get_ref(), &[1])?;
            if state.data.get_ref() != &expected_state
                || plain.first_free()?.value() != 1
                || plain.find_path_for_leaf(leaf).is_none()
                || historic.first_free()?.value() != u128::from(selected)
                || (selected && historic.find_path_for_leaf(leaf).is_none())
            {
                return Err("Merkle proof applied unexpected tree state".into());
            }
            Ok(())
        })?;
    }
    println!(
        "plain/historic Merkle selected/skipped insertion branches proved and applied through ledger8"
    );
    Ok(())
}
