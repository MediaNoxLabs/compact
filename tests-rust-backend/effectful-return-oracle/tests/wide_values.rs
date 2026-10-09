// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Wide-value execution controls: one retained source circuit plus finite IR-only
//! probes. This combined generated contract is not a source-matched proof artifact.
//! Native and recorded code are emitted independently; a small manual public
//! program supplies a third transcript/replay oracle.
include!(concat!(env!("OUT_DIR"), "/wide_contract.rs"));

use runtime::context::{CircuitContext, CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::ContractAddress;
use runtime::recording::{RecordedCircuitResult, RecordingFrame};
use runtime::{CompactError, Field};
use std::cell::RefCell;

type Observation = (Field, Field, Field, u64);

struct Witness {
    fail: bool,
    observations: RefCell<Vec<Observation>>,
}

impl Witness {
    fn new(fail: bool) -> Self {
        Self {
            fail,
            observations: RefCell::new(Vec::new()),
        }
    }
}

impl ledger_contract::TryWitnesses<u64> for Witness {
    fn mark(
        &self,
        _: WitnessContext<'_, u64, ledger_contract::LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        panic!("wide probes must call observe, not the unchanged original mark witness")
    }

    fn observe(
        &self,
        context: WitnessContext<'_, u64, ledger_contract::LedgerView<'_>>,
        value: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.observations.borrow_mut().push((
            value,
            context.ledger.state()?,
            context.ledger.unused()?,
            *context.private_state,
        ));
        if self.fail {
            Err(CompactError::AssertionFailed(
                "wide probe witness refusal".into(),
            ))
        } else {
            Ok((*context.private_state + 1, value + Field::from(77_u64)))
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Probe {
    Equal,
    Unequal,
    Maximum,
}

impl Probe {
    fn operands(self) -> (Field, Field) {
        // Independent byte constants: 2^128+1, 2^128+2, and 2^248-1.
        let mut left = [0_u8; 32];
        left[0] = 1;
        left[16] = 1;
        let mut right = left;
        match self {
            Self::Equal => (),
            Self::Unequal => right[0] = 2,
            Self::Maximum => {
                left[..31].fill(255);
                right = left;
            }
        }
        (
            Field::from_le_bytes(&left).unwrap(),
            Field::from_le_bytes(&right).unwrap(),
        )
    }

    fn calls_witness(self, next: u64) -> bool {
        !matches!(self, Self::Unequal) || next != 0
    }

    fn native(
        self,
        context: CircuitContext<u64>,
        witness: &Witness,
        next: u64,
    ) -> Result<CircuitResult<u64, Field>, CompactError> {
        match self {
            Self::Equal => ledger_contract::wide_equal(context, witness, Field::from(next)),
            Self::Unequal => ledger_contract::wide_unequal(context, witness, Field::from(next)),
            Self::Maximum => ledger_contract::wide_maximum(context, witness, Field::from(next)),
        }
    }

    fn recorded(
        self,
        context: CircuitContext<u64>,
        witness: &Witness,
        next: u64,
    ) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
        match self {
            Self::Equal => {
                ledger_contract::recorded::wide_equal(context, witness, Field::from(next))
            }
            Self::Unequal => {
                ledger_contract::recorded::wide_unequal(context, witness, Field::from(next))
            }
            Self::Maximum => {
                ledger_contract::recorded::wide_maximum(context, witness, Field::from(next))
            }
        }
    }
}

fn context(seed: Field, private: u64) -> CircuitContext<u64> {
    let initial = ledger_contract::initial_state(ConstructorContext::new(private)).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let context = ledger_slots::state.write(context, seed).unwrap().context;
    ledger_slots::unused
        .write(context, Field::from(23_u64))
        .unwrap()
        .context
}

fn expected_observations(probe: Probe, next: u64, private: u64) -> Vec<Observation> {
    let (left, right) = probe.operands();
    if probe.calls_witness(next) {
        vec![(right, left, Field::from(23_u64), private)]
    } else {
        vec![]
    }
}

fn public_reference(
    probe: Probe,
    seed: Field,
    next: u64,
    private: u64,
) -> RecordedCircuitResult<u64, Field> {
    let (left, right) = probe.operands();
    let (frame, before) = ledger_slots::state
        .record_read(RecordingFrame::new(context(seed, private)))
        .unwrap();
    let frame = ledger_slots::state.record_write(frame, left).unwrap();
    let (frame, result) = if probe.calls_witness(next) {
        let frame = ledger_slots::unused
            .record_write(frame, right + Field::from(77_u64))
            .unwrap();
        (frame, before + right)
    } else {
        (frame, before)
    };
    frame.finish(result)
}

fn compare_success(probe: Probe, seed: Field, next: u64, private: u64) {
    let native_witness = Witness::new(false);
    let recorded_witness = Witness::new(false);
    let native = probe
        .native(context(seed, private), &native_witness, next)
        .unwrap();
    let recorded = probe
        .recorded(context(seed, private), &recorded_witness, next)
        .unwrap();
    let expected = public_reference(probe, seed, next, private);
    assert_eq!(native.result, expected.execution.result);
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.query.state,
        expected.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        expected.execution.context.query.effects
    );
    assert_eq!(
        native.context.private_state,
        private + u64::from(probe.calls_witness(next))
    );
    assert_eq!(
        native.context.private_state,
        recorded.execution.context.private_state
    );
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(
        native.private_transcript_outputs.len(),
        usize::from(probe.calls_witness(next))
    );
    // Equality compares all four runtime gas dimensions, including witness reads.
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let observations = expected_observations(probe, next, private);
    assert_eq!(*native_witness.observations.borrow(), observations);
    assert_eq!(*recorded_witness.observations.borrow(), observations);
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(expected.public.verify_ops()).unwrap()
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
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    // Compare two batched public replays. Per-query execution also charges
    // intermediate writes and query overhead, so its gas is a different metric.
    let expected_replay = expected
        .public
        .initial()
        .query(
            expected.public.verify_ops(),
            None,
            &expected.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.gas_cost, expected_replay.gas_cost);
}

#[test]
fn wide_values_preserve_native_recorded_and_public_replay_semantics() {
    let mut cases = 0;
    for probe in [Probe::Equal, Probe::Unequal, Probe::Maximum] {
        for seed in [Field::from(0_u64), Field::from(9_u64), -Field::from(1_u64)] {
            for next in [0_u64, 1] {
                for private in [0_u64, 7] {
                    compare_success(probe, seed, next, private);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 36);
}

#[test]
fn wide_witness_failure_preserves_observation_prefix_and_skipped_branch() {
    let mut cases = 0;
    for probe in [Probe::Equal, Probe::Unequal, Probe::Maximum] {
        for seed in [Field::from(0_u64), Field::from(9_u64)] {
            for next in [0_u64, 1] {
                let native_witness = Witness::new(true);
                let recorded_witness = Witness::new(true);
                let native = probe.native(context(seed, 7), &native_witness, next);
                let recorded = probe
                    .recorded(context(seed, 7), &recorded_witness, next)
                    .map(|result| result.execution);
                for result in [native, recorded] {
                    if probe.calls_witness(next) {
                        assert_eq!(
                            result.err().expect("selected witness fails"),
                            CompactError::AssertionFailed("wide probe witness refusal".into())
                        );
                    } else {
                        let result = result.unwrap();
                        assert_eq!(result.result, seed);
                        assert_eq!(result.context.private_state, 7);
                        assert!(result.private_transcript_outputs.is_empty());
                    }
                }
                let expected = expected_observations(probe, next, 7);
                assert_eq!(*native_witness.observations.borrow(), expected);
                assert_eq!(*recorded_witness.observations.borrow(), expected);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 12);
}

struct SourceWitness;

impl ledger_contract::TryWitnesses<u64> for SourceWitness {
    fn mark(
        &self,
        context: WitnessContext<'_, u64, ledger_contract::LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        Ok((*context.private_state + 1, Field::from(77_u64)))
    }

    fn observe(
        &self,
        _: WitnessContext<'_, u64, ledger_contract::LedgerView<'_>>,
        _: Field,
    ) -> Result<(u64, Field), CompactError> {
        panic!("retained source must not invoke the IR-only probe witness")
    }
}

fn source_public_reference(seed: Field) -> RecordedCircuitResult<u64, Field> {
    // The retained source's wide equality is true, so the public program reads
    // the branch condition after the first write and records only its chosen arm.
    let (frame, before) = ledger_slots::state
        .record_read(RecordingFrame::new(context(seed, 7)))
        .unwrap();
    let frame = ledger_slots::state
        .record_write(frame, before + Field::from(1_u64))
        .unwrap();
    let (frame, observed) = ledger_slots::state.record_read(frame).unwrap();
    let (frame, result) = if observed == Field::from(1_u64) {
        let frame = ledger_slots::state
            .record_write(frame, Field::from(8_u64))
            .unwrap();
        (frame, before)
    } else {
        let frame = ledger_slots::unused
            .record_write(frame, Field::from(77_u64))
            .unwrap();
        (frame, before + Field::from(100_u64))
    };
    frame.finish(result)
}

#[test]
fn retained_wide_source_body_executes_both_branches_and_replays() {
    // choose is copied unchanged from effect-plan-wide-schema20-ir.json.
    // Other exports in this integration crate are explicitly IR-only probes.
    for seed in [0_u64, 9] {
        let seed = Field::from(seed);
        let native =
            ledger_contract::choose(context(seed, 7), &SourceWitness, Field::from(8_u64)).unwrap();
        let recorded =
            ledger_contract::recorded::choose(context(seed, 7), &SourceWitness, Field::from(8_u64))
                .unwrap();
        let selected = seed == Field::from(0_u64);
        let expected = if selected {
            seed
        } else {
            seed + Field::from(100_u64)
        };
        assert_eq!(native.result, expected);
        assert_eq!(recorded.execution.result, expected);
        assert_eq!(
            native.context.query.state,
            recorded.execution.context.query.state
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(native.context.private_state, 7 + u64::from(!selected));
        assert_eq!(
            native.context.private_state,
            recorded.execution.context.private_state
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
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
        let reference = source_public_reference(seed);
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            serde_json::to_value(reference.public.verify_ops()).unwrap()
        );
        let reference_replay = reference
            .public
            .initial()
            .query(
                reference.public.verify_ops(),
                None,
                &reference.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(reference_replay.context.state, native.context.query.state);
        assert_eq!(
            reference_replay.context.effects,
            native.context.query.effects
        );
        // Replay-to-replay comparison preserves every gas dimension without
        // conflating batched VM replay with per-query circuit execution.
        assert_eq!(replay.gas_cost, reference_replay.gas_cost);
    }
}
