// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_effect_plan_pure_calls_source_fixture::{
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
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.value(*ctx.private_state, seed)
    }
}
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
// Independent official primitive composition. The two-field tuple has the same
// declared aligned encoding as Pair; no generated pure function is called here.
fn digest(seed: Field, selected: bool) -> Field {
    let folded = r::transient_commit(
        r::transient_commit(seed, Field::from(0_u64)),
        Field::from(0_u64),
    );
    r::transient_hash(if selected {
        (seed, folded)
    } else {
        (folded, seed)
    })
}
fn witness(
    frame: RecordingFrame<u64>,
    w: &Witness,
    seed: Field,
) -> Result<(RecordingFrame<u64>, Field), CompactError> {
    frame.try_witness_metered(|ctx, _| w.value(ctx.private_state, seed))
}
fn update(
    frame: RecordingFrame<u64>,
    seed: Field,
    selected: bool,
    permit: bool,
) -> Result<(RecordingFrame<u64>, Field), CompactError> {
    let (frame, previous) = ledger_slots::stored.record_read(frame)?;
    if !permit {
        return Err(CompactError::AssertionFailed("pure helper refused".into()));
    }
    let frame = ledger_slots::stored.record_write(frame, digest(seed + previous, selected))?;
    ledger_slots::stored.record_read(frame)
}
fn reference(
    selected: bool,
    seed: Field,
    permit: bool,
    w: &Witness,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
    let (frame, left_arg) = witness(RecordingFrame::new(context()), w, seed)?;
    let (frame, right_arg) = witness(frame, w, seed + Field::from(1_u64))?;
    let first = r::transient_hash((left_arg, right_arg));
    let (frame, left) = update(frame, first, selected, true)?;
    let (frame, next) = witness(frame, w, if selected { left } else { seed })?;
    let (frame, next) = update(frame, next, selected, permit)?;
    let frame = ledger_slots::stored.record_write(frame, next)?;
    let (frame, stored) = ledger_slots::stored.record_read(frame)?;
    Ok(frame.finish(digest(stored, !selected)))
}
#[test]
fn aggregate_hash_fold_helpers_preserve_native_recorded_and_manual_execution() {
    for selected in [false, true] {
        for seed in [0_u64, 1, 19] {
            let seed = Field::from(seed);
            let a = Witness::default();
            let b = Witness::default();
            let w = Witness::default();
            let native = c::run(context(), &a, selected, seed, true).unwrap();
            let recorded = c::recorded::run(context(), &b, selected, seed, true).unwrap();
            let manual = reference(selected, seed, true, &w).unwrap();
            assert_eq!(*a.calls.borrow(), *w.calls.borrow());
            assert_eq!(*b.calls.borrow(), *w.calls.borrow());
            assert_eq!(w.calls.borrow().len(), 3);
            assert_eq!(w.calls.borrow()[0], (seed, 7));
            assert_eq!(w.calls.borrow()[1], (seed + Field::from(1_u64), 8));
            for execution in [&recorded.execution, &manual.execution] {
                assert_eq!(execution.result, native.result);
                assert_eq!(execution.context.query.state, native.context.query.state);
                assert_eq!(
                    execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(execution.gas_cost, native.gas_cost);
                assert_eq!(execution.context.private_state, 10);
                assert_eq!(
                    execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
            }
            assert_eq!(native.context.private_state, 10);
            assert_eq!(native.private_transcript_outputs.len(), 3);
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
            let expected = manual
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
            assert_eq!(replay.gas_cost, expected.gas_cost);
        }
    }
}
#[test]
fn witness_failures_preserve_exact_prefix() {
    for selected in [false, true] {
        for fail_at in 1..=3 {
            let make = || Witness {
                fail_at: Some(fail_at),
                ..Witness::default()
            };
            let a = make();
            let b = make();
            let w = make();
            let seed = Field::from(3_u64);
            let error = reference(selected, seed, true, &w).err().unwrap();
            assert_eq!(
                error,
                CompactError::AssertionFailed("witness refused".into())
            );
            assert_eq!(
                c::run(context(), &a, selected, seed, true).err().unwrap(),
                error
            );
            assert_eq!(
                c::recorded::run(context(), &b, selected, seed, true)
                    .err()
                    .unwrap(),
                error
            );
            assert_eq!(*a.calls.borrow(), *w.calls.borrow());
            assert_eq!(*b.calls.borrow(), *w.calls.borrow());
            assert_eq!(w.calls.borrow().len(), fail_at);
        }
    }
}
#[test]
fn pure_assertion_failure_after_first_update_rolls_back_lab_state() {
    for selected in [false, true] {
        let a = Witness::default();
        let b = Witness::default();
        let w = Witness::default();
        let seed = Field::from(3_u64);
        let error = reference(selected, seed, false, &w).err().unwrap();
        assert_eq!(
            error,
            CompactError::AssertionFailed("pure helper refused".into())
        );
        assert_eq!(
            c::run(context(), &a, selected, seed, false).err().unwrap(),
            error
        );
        assert_eq!(
            c::recorded::run(context(), &b, selected, seed, false)
                .err()
                .unwrap(),
            error
        );
        assert_eq!(*a.calls.borrow(), *w.calls.borrow());
        assert_eq!(*b.calls.borrow(), *w.calls.borrow());
        assert_eq!(w.calls.borrow().len(), 3);
        // This identity is rollback-only; source/key binding has a separate proof gate.
        let mut lab = ContractLab::from_constructor(
            ArtifactIdentity {
                source_sha256: [0; 32],
                generated_sha256: [0; 32],
            },
            Environment::new(Default::default(), Default::default(), [0; 32]),
            c::initial_state(ConstructorContext::new(7)).unwrap(),
        )
        .unwrap();
        let before = lab.snapshot().public_state().clone();
        let witness = Witness::default();
        let result = lab.recorded(|ctx| c::recorded::run(ctx, &witness, selected, seed, false));
        assert!(matches!(result, Err(LabError::Execution(ref e)) if *e == error));
        assert_eq!(lab.snapshot().public_state(), &before);
        assert_eq!(*lab.private_state(), 7);
    }
}
