// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Real-source expansions: nested physical layout, no write, no helper, empty public program.
use compact_rust_effect_plan_unit_actions_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::RecordingFrame;
use r::{CompactError, Field};
use std::cell::RefCell;
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
#[derive(Default)]
struct Witness {
    calls: RefCell<Vec<u64>>,
    refuse: bool,
}
impl Witness {
    fn invoke(&self, private: u64) -> Result<(u64, ()), CompactError> {
        self.calls.borrow_mut().push(private);
        if self.refuse {
            return Err(CompactError::AssertionFailed("unit witness refused".into()));
        }
        Ok((private + 1, ()))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn notifyOnce(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, ()), CompactError> {
        self.invoke(*ctx.private_state)
    }
}
#[test]
fn read_only_helper_uses_actual_nested_slot_without_a_write() {
    let zero = Field::from(0_u64);
    let native = c::verify(context(), zero).unwrap();
    let recorded = c::recorded::verify(context(), zero).unwrap();
    let (frame, value) = ledger_slots::stored
        .record_read(RecordingFrame::new(context()))
        .unwrap();
    assert_eq!(value, zero);
    let manual = frame.finish(());
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
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    assert!(!recorded.public.verify_ops().is_empty());
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
    // Pin the compiler's real 16-field chunk layout, not a forged one-field path.
    let ir: serde_json::Value =
        serde_json::from_str(compact_rust_effect_plan_unit_actions_source_fixture::RETAINED_IR)
            .unwrap();
    assert_eq!(ir["ledger_fields"].as_array().unwrap().len(), 16);
    assert_eq!(ir["ledger_fields"][0]["path"], serde_json::json!([0, 0]));
    assert_eq!(ir["ledger_fields"][15]["path"], serde_json::json!([1, 14]));
}
#[test]
fn read_only_helper_keeps_native_assertion_failure() {
    let error = CompactError::AssertionFailed("stored value differs".into());
    assert_eq!(
        c::verify(context(), Field::from(1_u64)).err().unwrap(),
        error
    );
    assert_eq!(
        c::recorded::verify(context(), Field::from(1_u64))
            .err()
            .unwrap(),
        error
    );
}
#[test]
fn direct_unit_witness_has_no_helper_or_synthetic_public_operation() {
    let a = Witness::default();
    let b = Witness::default();
    let w = Witness::default();
    let native = c::no_helper(context(), &a).unwrap();
    let recorded = c::recorded::no_helper(context(), &b).unwrap();
    let (frame, ()) = RecordingFrame::new(context())
        .try_witness_metered(|ctx, _| w.invoke(ctx.private_state))
        .unwrap();
    let manual = frame.finish(());
    assert_eq!(*a.calls.borrow(), vec![7]);
    assert_eq!(*b.calls.borrow(), *a.calls.borrow());
    assert_eq!(*w.calls.borrow(), *a.calls.borrow());
    assert_eq!(native.context.private_state, 8);
    assert_eq!(
        native.private_transcript_outputs,
        vec![r::fab::AlignedValue::from(())]
    );
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
        assert_eq!(execution.context.private_state, 8);
        assert_eq!(
            execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
    assert!(recorded.public.verify_ops().is_empty());
    assert!(manual.public.verify_ops().is_empty());
}
#[test]
fn direct_unit_witness_failure_is_preserved() {
    let a = Witness {
        refuse: true,
        ..Witness::default()
    };
    let b = Witness {
        refuse: true,
        ..Witness::default()
    };
    let error = CompactError::AssertionFailed("unit witness refused".into());
    assert_eq!(c::no_helper(context(), &a).err().unwrap(), error);
    assert_eq!(c::recorded::no_helper(context(), &b).err().unwrap(), error);
    assert_eq!(*a.calls.borrow(), vec![7]);
    assert_eq!(*b.calls.borrow(), vec![7]);
}
#[cfg(feature = "ledger-transaction")]
#[test]
fn direct_unit_witness_is_recordable_but_empty_transcript_cannot_be_prepared() {
    use midnight_serialize::{Deserializable, Serializable};
    use r::transaction::{CallSpec, PrepareCallError, VerifierKey, prepare_call};
    let recorded = c::recorded::no_helper(context(), &Witness::default()).unwrap();
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    // Early refusal carrier only, not a cryptographic proof key.
    let carrier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    assert!(matches!(
        prepare_call(
            recorded,
            CallSpec::new("no_helper", carrier, (), Field::from(0_u64))
        ),
        Err(PrepareCallError::EmptyTranscript)
    ));
}
