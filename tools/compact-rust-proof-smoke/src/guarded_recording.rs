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

//! Prove a guarded pure arithmetic call followed by one Counter increment.

use super::*;
use compact_rust_guarded_assert_arith_oracle_fixture::ledger_contract as guarded;
use compact_rust_guarded_assert_arith_oracle_fixture::ledger_slots as slots;
use compact_rust_guarded_assert_arith_oracle_fixture::types::{
    Attestation, StatusProof, VerifierPolicy,
};
use midnight_compact_runtime as runtime;
use runtime::recording::RecordingFrame;

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/guarded-assert-arith-schema20-ir.json");

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    const CIRCUIT: &str = "recordFreshEnough";
    let mut rng = StdRng::seed_from_u64(0x0091_4755_4152_4433);
    // Constructor state is supplied. This scenario proves the call, not deployment.
    let initial = guarded::initial_state(ConstructorContext::new(()))?;
    if slots::accepted.inspect(initial.ledger_state.get_ref())? != 0 {
        return Err("guarded arithmetic constructor Counter is not zero".into());
    }
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let policy = VerifierPolicy {
        enforceMaxAge: true,
        maxAge: BoundedUint::new(20)?,
    };
    let attestation = Attestation {
        proof: StatusProof {
            createdAt: BoundedUint::new(100)?,
            issuer: BoundedUint::new(1)?,
        },
        hasExpiration: false,
        expiresAt: BoundedUint::new(0)?,
    };
    let now = BoundedUint::new(110)?;
    let native = guarded::recordFreshEnough(
        observed.circuit_context(()),
        policy.clone(),
        attestation.clone(),
        now,
    )?;
    let recorded = guarded::recorded::recordFreshEnough(
        observed.circuit_context(()),
        policy.clone(),
        attestation.clone(),
        now,
    )?;
    let call = guarded::recorded::Contract.recordFreshEnough_call(
        &observed,
        (),
        policy.clone(),
        attestation.clone(),
        now,
    )?;
    // Independent source predicates execute before the sole public operation.
    // Do not call the generated pure function to construct this reference trace.
    let created = attestation.proof.createdAt.value();
    if now.value() < created {
        return Err("manual freshness creation time is in the future".into());
    }
    if policy.enforceMaxAge && now.value() - created > policy.maxAge.value() {
        return Err("manual freshness max-age guard failed".into());
    }
    let manual = slots::accepted
        .record_increment(RecordingFrame::new(observed.circuit_context(())), 1)?
        .finish(());
    if !native.private_transcript_outputs.is_empty() {
        return Err("guarded arithmetic unexpectedly requested a witness".into());
    }
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err(
                "guarded arithmetic native/recorded/observed/manual execution differs".into(),
            );
        }
        // Private state and result are Unit in all four lanes by their Rust types.
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != manual_replay.gas_cost
        {
            return Err("guarded arithmetic verifier replay differs from manual replay".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::accepted.inspect(&expected_state)? != 1 {
        return Err("guarded arithmetic did not increment accepted exactly once".into());
    }
    let input = AlignedValue::from((policy, attestation, now));
    let direct = check_generated_trace(root, CIRCUIT, recorded, input.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/recordFreshEnough.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != input || prepared.output != AlignedValue::from(()) {
        return Err("guarded arithmetic declared argument order/Unit output differs".into());
    }
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("guarded arithmetic observed/direct preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 600)?;
    println!(
        "guarded arithmetic: source-bound native/recorded/observed/manual/replay, verified proof and changed-binding refusal, default-strict funded ledger-8 application; constructor state supplied, deployment not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_guarded_source_control"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn guarded_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-guarded-proof-absent-{}-{}",
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
