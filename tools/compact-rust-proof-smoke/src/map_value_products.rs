// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound guarded Map read preserving vector, tuple and wide stored fields.
use super::*;
use compact_rust_effect_plan_map_value_products_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, support,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "readFresh";
    let mut rng = StdRng::seed_from_u64(595);
    let key: r::OpaqueString = "map-λ".into();
    let value = support::proof_value();
    let initial = c::initial_state(ConstructorContext::new(7_u64), key.clone(), value.clone())?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let observed = pure_field_arguments::observed_state(&deploy);
    let as_of = r::BoundedUint::new(100)?;
    let native = c::readFresh(observed.circuit_context(7), key.clone(), as_of)?;
    let recorded = c::recorded::readFresh(observed.circuit_context(7), key.clone(), as_of)?;
    let call = c::recorded::Contract.readFresh_call(&observed, 7, key.clone(), as_of)?;
    let manual = support::manual_read(observed.circuit_context(7), key.clone(), as_of)?;
    if native.context.private_state != 7 || !native.private_transcript_outputs.is_empty() {
        return Err("Map product read changed native private state/output".into());
    }
    let expected_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != 7
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("Map product native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != expected_replay.gas_cost
        {
            return Err("Map product verifier replay differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if expected_state != *observed.contract().data.get_ref()
        || slots::records
            .inspect(&expected_state)?
            .lookup(key.clone())?
            != value
    {
        return Err("Map product read changed original vector/tuple/wide state".into());
    }
    let verifier = pure_field_arguments::verifier(root, name)?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from((key.clone(), as_of))
        || prepared.output != AlignedValue::from(())
    {
        return Err("Map product read declared arguments/Unit output differ".into());
    }
    let direct = check_generated_trace(root, name, recorded, (key, as_of))?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("Map product direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 595)?;
    println!(
        "guarded Map product read: exact vector/tuple/Uint129 stored value, native/recorded/observed/manual/replay, verified proof and changed-binding refusal, default-strict funded ledger-8 application; constructor seed supplied, deployment not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_map_product_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-map-product-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        assert_eq!(
            run(&root)
                .unwrap_err()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
