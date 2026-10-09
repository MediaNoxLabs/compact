// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_effect_plan_stateful_calls_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field};
use std::cell::RefCell;

#[derive(Default)]
struct Witness {
    calls: RefCell<Vec<(Field, u64)>>,
    fail_at: Option<usize>,
}
impl Witness {
    fn value(&self, private: u64, seed: Field) -> Result<(u64, Field), CompactError> {
        self.calls.borrow_mut().push((seed, private));
        if self.fail_at == Some(self.calls.borrow().len()) {
            return Err(CompactError::AssertionFailed("witness refused".into()));
        }
        Ok((private + 1, seed + Field::from(private)))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn nextValue(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.value(*context.private_state, seed)
    }
}
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn witness(
    frame: RecordingFrame<u64>,
    witness: &Witness,
    seed: Field,
) -> Result<(RecordingFrame<u64>, Field), CompactError> {
    frame.try_witness_metered(|ctx, _| witness.value(ctx.private_state, seed))
}
fn update(
    frame: RecordingFrame<u64>,
    w: &Witness,
    left: Field,
    right: Field,
    permit: bool,
) -> Result<(RecordingFrame<u64>, Field), CompactError> {
    let (frame, before) = ledger_slots::stored.record_read(frame)?;
    let (frame, value) = witness(frame, w, left + right + before)?;
    let frame = ledger_slots::stored.record_write(frame, value)?;
    if !permit {
        return Err(CompactError::AssertionFailed(
            "helper failed after write".into(),
        ));
    }
    ledger_slots::stored.record_read(frame)
}
fn reference(
    selected: bool,
    first: Field,
    second: Field,
    permit: bool,
    w: &Witness,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
    let (frame, left) = witness(RecordingFrame::new(context()), w, first)?;
    let (frame, right) = witness(frame, w, second)?;
    let (frame, pair) = update(frame, w, left, right, true)?;
    let (frame, inside) = update(frame, w, pair, first, true)?;
    let (frame, nested) = update(frame, w, inside, second, true)?;
    let (frame, value) = if selected {
        update(frame, w, nested, first, permit)?
    } else {
        update(frame, w, second, nested, permit)?
    };
    let frame = ledger_slots::receipt.record_write(frame, value)?;
    let (frame, receipt) = ledger_slots::receipt.record_read(frame)?;
    Ok(frame.finish(receipt + first))
}

#[test]
fn repeated_nested_calls_preserve_values_scopes_and_all_effect_order() {
    for selected in [false, true] {
        for first in [0, 1, 19] {
            for second in [0, 2, 23] {
                let (first, second) = (Field::from(first as u64), Field::from(second as u64));
                let native_witness = Witness::default();
                let recorded_witness = Witness::default();
                let manual_witness = Witness::default();
                let native =
                    c::run(context(), &native_witness, selected, first, second, true).unwrap();
                let recorded =
                    c::recorded::run(context(), &recorded_witness, selected, first, second, true)
                        .unwrap();
                let manual = reference(selected, first, second, true, &manual_witness).unwrap();
                assert_eq!(
                    *native_witness.calls.borrow(),
                    *manual_witness.calls.borrow()
                );
                assert_eq!(
                    *recorded_witness.calls.borrow(),
                    *manual_witness.calls.borrow()
                );
                assert_eq!(manual_witness.calls.borrow().len(), 6);
                assert_eq!(manual_witness.calls.borrow()[0], (first, 7));
                assert_eq!(manual_witness.calls.borrow()[1], (second, 8));
                for execution in [&recorded.execution, &manual.execution] {
                    assert_eq!(execution.result, native.result);
                    assert_eq!(execution.context.query.state, native.context.query.state);
                    assert_eq!(
                        execution.context.query.effects,
                        native.context.query.effects
                    );
                    assert_eq!(execution.gas_cost, native.gas_cost);
                    assert_eq!(execution.context.private_state, 13);
                    assert_eq!(
                        execution.private_transcript_outputs,
                        native.private_transcript_outputs
                    );
                }
                assert_eq!(native.context.private_state, 13);
                assert_eq!(native.private_transcript_outputs.len(), 6);
                assert_eq!(
                    native.result,
                    ledger_slots::receipt
                        .inspect(native.context.query.state.get_ref())
                        .unwrap()
                        + first
                );
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
                assert_eq!(replay.gas_cost, reference_replay.gas_cost);
            }
        }
    }
}

#[test]
fn each_argument_and_helper_witness_failure_stops_the_remaining_calls() {
    for selected in [false, true] {
        for fail_at in 1..=6 {
            let make = || Witness {
                fail_at: Some(fail_at),
                ..Witness::default()
            };
            let a = make();
            let b = make();
            let manual = make();
            let expected = reference(
                selected,
                Field::from(3_u64),
                Field::from(5_u64),
                true,
                &manual,
            )
            .err()
            .unwrap();
            assert_eq!(
                expected,
                CompactError::AssertionFailed("witness refused".into())
            );
            assert_eq!(
                c::run(
                    context(),
                    &a,
                    selected,
                    Field::from(3_u64),
                    Field::from(5_u64),
                    true
                )
                .err()
                .unwrap(),
                expected
            );
            assert_eq!(
                c::recorded::run(
                    context(),
                    &b,
                    selected,
                    Field::from(3_u64),
                    Field::from(5_u64),
                    true
                )
                .err()
                .unwrap(),
                expected
            );
            assert_eq!(*a.calls.borrow(), *manual.calls.borrow());
            assert_eq!(*b.calls.borrow(), *manual.calls.borrow());
            assert_eq!(b.calls.borrow().len(), fail_at);
        }
    }
}

#[test]
fn assertion_after_helper_write_rolls_back_public_and_private_lab_state() {
    for selected in [false, true] {
        let a = Witness::default();
        let b = Witness::default();
        let manual = Witness::default();
        let expected = reference(
            selected,
            Field::from(3_u64),
            Field::from(5_u64),
            false,
            &manual,
        )
        .err()
        .unwrap();
        assert_eq!(
            expected,
            CompactError::AssertionFailed("helper failed after write".into())
        );
        assert_eq!(
            c::run(
                context(),
                &a,
                selected,
                Field::from(3_u64),
                Field::from(5_u64),
                false
            )
            .err()
            .unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::run(
                context(),
                &b,
                selected,
                Field::from(3_u64),
                Field::from(5_u64),
                false
            )
            .err()
            .unwrap(),
            expected
        );
        assert_eq!(*a.calls.borrow(), *manual.calls.borrow());
        assert_eq!(*b.calls.borrow(), *manual.calls.borrow());
        assert_eq!(b.calls.borrow().len(), 6);
        let initial = c::initial_state(ConstructorContext::new(7)).unwrap();
        let mut lab = ContractLab::from_constructor(
            ArtifactIdentity {
                source_sha256: [0; 32],
                generated_sha256: [0; 32],
            },
            Environment::new(Default::default(), Default::default(), [0; 32]),
            initial,
        )
        .unwrap();
        let before = lab.snapshot().public_state().clone();
        let witness = Witness::default();
        let result = lab.recorded(|ctx| {
            c::recorded::run(
                ctx,
                &witness,
                selected,
                Field::from(3_u64),
                Field::from(5_u64),
                false,
            )
        });
        assert!(matches!(result, Err(LabError::Execution(ref error)) if *error == expected));
        assert_eq!(lab.snapshot().public_state(), &before);
        assert_eq!(*lab.private_state(), 7);
    }
}
