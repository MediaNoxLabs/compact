// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite composition controls; old-family grammar inclusion is a separate gate.
use compact_rust_effect_plan_jubjub_source_fixture::{
    ledger_contract as contract, ledger_slots, runtime as r,
};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{CircuitContext, CircuitResult, ConstructorContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, JubjubPoint};

type Execution = CircuitResult<u64, JubjubPoint>;
type Recorded = RecordedCircuitResult<u64, JubjubPoint>;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Compose,
    ReadBranch(bool),
    RawAfterWrite,
}

fn context() -> CircuitContext<u64> {
    contract::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}

fn native(
    scenario: Scenario,
    point: JubjubPoint,
    scalar: Field,
) -> Result<Execution, CompactError> {
    match scenario {
        Scenario::Compose => contract::compose(context(), point, scalar),
        Scenario::ReadBranch(selected) => contract::read_branch(context(), selected, point, scalar),
        Scenario::RawAfterWrite => contract::raw_after_write(context(), point, scalar),
    }
}

fn recorded(
    scenario: Scenario,
    point: JubjubPoint,
    scalar: Field,
) -> Result<Recorded, CompactError> {
    match scenario {
        Scenario::Compose => contract::recorded::compose(context(), point, scalar),
        Scenario::ReadBranch(selected) => {
            contract::recorded::read_branch(context(), selected, point, scalar)
        }
        Scenario::RawAfterWrite => contract::recorded::raw_after_write(context(), point, scalar),
    }
}

/// Independent source-order program using official typed slots and crypto adapters.
fn reference(
    scenario: Scenario,
    point: JubjubPoint,
    scalar: Field,
) -> Result<Recorded, CompactError> {
    let frame = RecordingFrame::new(context());
    match scenario {
        Scenario::Compose => {
            let reduced = r::jubjub_scalar_from_native(scalar);
            let combined = r::ec_add(r::ec_mul_generator(reduced)?, point);
            let value = r::ec_mul(combined, reduced)?;
            let frame = ledger_slots::first.record_write(frame, value)?;
            Ok(frame.finish(r::ec_add(value, r::ec_mul_generator(Field::from(1_u64))?)))
        }
        Scenario::ReadBranch(selected) => {
            let frame = ledger_slots::first.record_write(frame, point)?;
            let reduced = r::jubjub_scalar_from_native(scalar);
            // Source mitigation: the same point is read once before either branch.
            let (frame, supplied) = ledger_slots::first.record_read(frame)?;
            let result = if selected {
                r::ec_add(supplied, r::ec_mul_generator(reduced)?)
            } else {
                r::ec_mul(supplied, reduced)?
            };
            let frame = ledger_slots::second.record_write(frame, result)?;
            let (frame, result) = ledger_slots::second.record_read(frame)?;
            Ok(frame.finish(result))
        }
        Scenario::RawAfterWrite => {
            let frame = ledger_slots::first.record_write(frame, point)?;
            let value = r::ec_mul(point, scalar)?;
            let frame = ledger_slots::second.record_write(frame, value)?;
            Ok(frame.finish(value))
        }
    }
}

fn assert_execution(native: Execution, recorded: Recorded, manual: Recorded) {
    assert_eq!(native.result, manual.execution.result);
    assert_eq!(recorded.execution.result, native.result);
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
        assert_eq!(execution.context.private_state, 7);
        assert!(execution.private_transcript_outputs.is_empty());
    }
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    let reference_replay = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    // Whole-program replay gas has a different batching policy from execution.
    assert_eq!(replay.gas_cost, reference_replay.gas_cost);
}

fn boundaries() -> Vec<(String, Field)> {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../jubjub-scalar-cell/oracle/boundaries.json"
    ))
    .unwrap();
    data["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let bytes = hex::decode(row["hex_le"].as_str().unwrap()).unwrap();
            (
                row["name"].as_str().unwrap().to_owned(),
                Field::from_le_bytes(&bytes).unwrap(),
            )
        })
        .collect()
}

#[test]
fn ordered_point_compositions_match_official_programs_at_scalar_boundaries() {
    let mut accepted = 0;
    let mut refused = 0;
    for point in [
        JubjubPoint::identity(),
        r::ec_mul_generator(Field::from(7_u64)).unwrap(),
    ] {
        for (name, scalar) in boundaries() {
            for scenario in [
                Scenario::Compose,
                Scenario::ReadBranch(false),
                Scenario::ReadBranch(true),
                Scenario::RawAfterWrite,
            ] {
                let native = native(scenario, point, scalar);
                let recorded = recorded(scenario, point, scalar);
                match reference(scenario, point, scalar) {
                    Ok(manual) => {
                        accepted += 1;
                        assert_execution(native.unwrap(), recorded.unwrap(), manual);
                    }
                    Err(error) => {
                        refused += 1;
                        assert!(
                            matches!(scenario, Scenario::RawAfterWrite),
                            "{name}: {scenario:?}"
                        );
                        assert!(matches!(name.as_str(), "q" | "q_plus_one" | "native_max"));
                        assert_eq!(error, CompactError::InvalidJubjubScalar);
                        assert_eq!(native.err().unwrap(), error);
                        assert_eq!(recorded.err().unwrap(), error);
                    }
                }
            }
        }
    }
    assert_eq!((accepted, refused), (50, 6));
}

#[test]
fn returned_point_is_not_substituted_with_the_stored_point() {
    let execution = native(
        Scenario::Compose,
        JubjubPoint::identity(),
        Field::from(0_u64),
    )
    .unwrap();
    let stored = ledger_slots::first
        .inspect(execution.context.query.state.get_ref())
        .unwrap();
    assert_eq!(stored, JubjubPoint::identity());
    assert_eq!(
        execution.result,
        r::ec_mul_generator(Field::from(1_u64)).unwrap()
    );
    assert_ne!(execution.result, stored);
}

#[test]
fn noncanonical_scalar_after_first_write_rolls_back_lab_state() {
    let point = r::ec_mul_generator(Field::from(7_u64)).unwrap();
    let scalar = boundaries()
        .into_iter()
        .find(|(name, _)| name == "q")
        .unwrap()
        .1;
    // Establish that the source's prefix really changes state before multiplication fails.
    let prefix = ledger_slots::first
        .record_write(RecordingFrame::new(context()), point)
        .unwrap();
    assert_eq!(
        ledger_slots::first
            .inspect(prefix.context().query.state.get_ref())
            .unwrap(),
        point
    );
    assert_eq!(
        r::ec_mul(point, scalar),
        Err(CompactError::InvalidJubjubScalar)
    );
    let initial = contract::initial_state(ConstructorContext::new(7)).unwrap();
    // Rollback-only identity; source/proof pairing is enforced by the proof adapter.
    let identity = ArtifactIdentity {
        source_sha256: [0; 32],
        generated_sha256: [0; 32],
    };
    let mut lab = ContractLab::from_constructor(
        identity,
        Environment::new(Default::default(), Default::default(), [0; 32]),
        initial,
    )
    .unwrap();
    let before = lab.snapshot().public_state().clone();
    let result = lab.recorded(|ctx| contract::recorded::raw_after_write(ctx, point, scalar));
    assert!(matches!(
        result,
        Err(LabError::Execution(CompactError::InvalidJubjubScalar))
    ));
    assert_eq!(lab.snapshot().public_state(), &before);
    assert_eq!(*lab.private_state(), 7);
}
