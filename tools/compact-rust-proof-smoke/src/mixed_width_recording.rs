// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Prove the exact mixed-width guard before the Counter-one VM continuation.

use super::*;
use compact_rust_mixed_width_operand_oracle_fixture::ledger_contract as mixed;
use midnight_compact_runtime::CompactError;
use pure_field_arguments::{observed_state, verifier};

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/mixed-width-operand-schema20-ir.json");

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0134_4d49_5845_4457);
    let initial = mixed::initial_state(
        ConstructorContext::new(()),
        BoundedUint::<4294967295>::new(20)?,
        BoundedUint::<4294967295>::new(4)?,
    )?;
    let deploy = make_deploy(
        root,
        "recordMatching",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = observed_state(&deploy);
    let small = BoundedUint::<255>::new(7)?;
    let matching = BoundedUint::<4294967295>::new(7)?;
    let different = BoundedUint::<4294967295>::new(8)?;
    let failure = mixed::recorded::Contract
        .recordMatching_call(&observed, (), small, different)
        .err()
        .ok_or("recordMatching(7,8) unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("recordMatching(7,8) failed with wrong error: {failure}").into());
    }
    let recorded = mixed::recorded::recordMatching(
        initial.into_circuit_context(deploy.address()),
        small,
        matching,
    )?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("recordMatching private output changed".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "recordMatching", recorded, (small, matching))?;
    let typed = mixed::recorded::Contract.recordMatching_call(&observed, (), small, matching)?;
    let prepared = typed.prepare(verifier(root, "recordMatching")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("recordMatching observed call differs from direct recording".into());
    }
    check_transaction(
        root,
        "recordMatching",
        deploy,
        prepared,
        &mut rng,
        |state| {
            if state.data.get_ref() != &expected_state {
                return Err("proven recordMatching changed unexpected ledger state".into());
            }
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err("recordMatching state is not an array".into());
            };
            if read_counter(fields.get(1).ok_or("recordMatching Counter missing")?)? != 1 {
                return Err("proven recordMatching incremented Counter incorrectly".into());
            }
            Ok(())
        },
    )?;
    pinned_call(root, &mut rng)
}

/// Prove the product guard only; recordMatching key material is not required.
pub(super) fn run_pinned(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0591_4d49_5845_4457);
    pinned_call(root, &mut rng)
}

fn pinned_call(root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let initial = mixed::initial_state(
        ConstructorContext::new(()),
        BoundedUint::<4294967295>::new(20)?,
        BoundedUint::<4294967295>::new(4)?,
    )?;
    let deploy = make_deploy(
        root,
        "recordPinned",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let observed = observed_state(&deploy);
    let q = BoundedUint::<4294967295>::new(4)?;
    let y = BoundedUint::<4294967295>::new(20)?;
    let rejected_q = BoundedUint::<4294967295>::new(1_073_741_824)?;
    let rejected_y = BoundedUint::<4294967295>::new(4_294_967_295)?;
    let failure = mixed::recorded::Contract
        .recordPinned_call(&observed, (), rejected_q, rejected_y)
        .err()
        .ok_or("recordPinned over bound unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("recordPinned over bound failed with wrong error: {failure}").into());
    }
    let native = mixed::recordPinned(observed.circuit_context(()), q, y)?;
    let recorded = mixed::recorded::recordPinned(observed.circuit_context(()), q, y)?;
    let typed = mixed::recorded::Contract.recordPinned_call(&observed, (), q, y)?;
    if !native.private_transcript_outputs.is_empty() {
        return Err("recordPinned native private output changed".into());
    }
    for candidate in [&recorded, typed.recorded()] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != recorded.public.verify_ops()
        {
            return Err("recordPinned native/recorded/observed execution differs".into());
        }
        // Both the private state and circuit result are statically Unit.
        let _: () = candidate.execution.context.private_state;
        let _: () = candidate.execution.result;
    }
    let _: () = native.context.private_state;
    let _: () = native.result;
    let expected_state = native.context.query.state.get_ref().clone();
    let StateValue::Array(fields) = &expected_state else {
        return Err("recordPinned state is not an array".into());
    };
    if read_counter(fields.get(1).ok_or("recordPinned Counter missing")?)? != 1 {
        return Err("recordPinned incremented Counter incorrectly".into());
    }
    let manual = check_generated_trace(root, "recordPinned", recorded, (q, y))?;
    let verifier = verifier(root, "recordPinned")?;
    let prepared = typed.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from((q, y)) || prepared.output != AlignedValue::from(()) {
        return Err("recordPinned changed original argument order or Unit output".into());
    }
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("recordPinned observed call differs from direct recording".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, "recordPinned", &prepared, &verifier)?;
    opaque_public::apply_funded(
        root,
        "recordPinned",
        prepared,
        deploy,
        &expected_state,
        rng.clone(),
        591,
    )?;
    println!(
        "recordPinned: mixed-width product guard, exact declared bindings, verified proof and changed-binding refusal, default-strict funded ledger-8 application; generated constructor state supplied, deployment not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_product_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn pinned_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-pinned-proof-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        assert_eq!(
            run_pinned(&root)
                .unwrap_err()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
