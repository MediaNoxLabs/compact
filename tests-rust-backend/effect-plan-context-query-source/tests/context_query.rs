// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_effect_plan_context_query_source_fixture::{ledger_contract as c, runtime as r};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::RecordingFrame;
use r::{CompactError, FixedBytes};
use std::cell::RefCell;
fn padded(s: &str) -> FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[..s.len()].copy_from_slice(s.as_bytes());
    FixedBytes::new(bytes)
}
fn context(n: u8) -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(r::ledger::ContractAddress(
            midnight_base_crypto::hash::HashOutput([n; 32]),
        ))
}
#[test]
fn direct_outer_commit_preserves_two_context_queries_and_official_commitment() {
    let ir: serde_json::Value =
        serde_json::from_str(compact_rust_effect_plan_context_query_source_fixture::RETAINED_IR)
            .unwrap();
    let value = &ir["stateful_circuits"][0]["return_value"]["value"];
    assert_eq!(value["kind"], "persistent_commit");
    assert_eq!(
        value["value"]["elements"][0]["value"]["kind"],
        "kernel_self"
    );
    assert_eq!(value["opening"]["value"]["kind"], "kernel_self");
    let mut results = std::collections::HashSet::new();
    for n in [0, 1, 7, 255] {
        let initial = context(n);
        let native = c::direct_commit(context(n)).unwrap();
        let recorded = c::recorded::direct_commit(context(n)).unwrap();
        let (frame, first) = RecordingFrame::new(context(n)).kernel_self().unwrap();
        let first = r::ledger::contract_address_bytes(&first);
        let (frame, opening) = frame.kernel_self().unwrap();
        let opening = r::ledger::contract_address_bytes(&opening);
        let expected = r::persistent_commit((first, padded("context-query")), opening);
        let manual = frame.finish(expected);
        assert!(results.insert(expected.0));
        for execution in [&native, &recorded.execution, &manual.execution] {
            assert_eq!(execution.result, expected);
            assert_eq!(execution.context.query.state, initial.query.state);
            assert_eq!(
                execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(execution.gas_cost, native.gas_cost);
            assert_eq!(execution.context.private_state, 7);
            assert!(execution.private_transcript_outputs.is_empty());
        }
        assert!(!recorded.public.verify_ops().is_empty());
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
        let reference = manual
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
        assert_eq!(replay.gas_cost, reference.gas_cost);
    }
}
#[derive(Default)]
struct Witness {
    calls: RefCell<Vec<u64>>,
    refuse: bool,
}
impl c::TryWitnesses<u64> for Witness {
    fn opening(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, FixedBytes<32>), CompactError> {
        self.calls.borrow_mut().push(*ctx.private_state);
        if self.refuse {
            return Err(CompactError::AssertionFailed("opening refused".into()));
        }
        Ok((*ctx.private_state + 1, padded("opening")))
    }
}
#[test]
fn witness_only_commit_keeps_private_output_without_synthetic_public_queries() {
    let a = Witness::default();
    let b = Witness::default();
    let native = c::no_context(context(3), &a).unwrap();
    let recorded = c::recorded::no_context(context(3), &b).unwrap();
    assert_eq!(
        native.result,
        r::persistent_commit((padded("left"), padded("right")), padded("opening"))
    );
    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(
        recorded.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(native.context.private_state, 8);
    assert_eq!(recorded.execution.context.private_state, 8);
    assert_eq!(
        native.private_transcript_outputs,
        vec![r::fab::AlignedValue::from(padded("opening"))]
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(*a.calls.borrow(), [7]);
    assert_eq!(*b.calls.borrow(), [7]);
    assert!(recorded.public.verify_ops().is_empty());
}
#[test]
fn opening_refusal_matches_native_once() {
    let a = Witness {
        refuse: true,
        ..Default::default()
    };
    let b = Witness {
        refuse: true,
        ..Default::default()
    };
    let expected = CompactError::AssertionFailed("opening refused".into());
    assert_eq!(c::no_context(context(0), &a).err().unwrap(), expected);
    assert_eq!(
        c::recorded::no_context(context(0), &b).err().unwrap(),
        expected
    );
    assert_eq!(*a.calls.borrow(), [7]);
    assert_eq!(*b.calls.borrow(), [7]);
}
#[cfg(feature = "ledger-transaction")]
#[test]
fn no_context_is_recordable_but_cannot_prepare_an_empty_public_transcript() {
    use midnight_serialize::{Deserializable, Serializable};
    use r::transaction::{CallSpec, PrepareCallError, VerifierKey, prepare_call};
    let recorded = c::recorded::no_context(context(0), &Witness::default()).unwrap();
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    // Early-refusal carrier only; this is not a cryptographic proof key.
    let carrier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    assert!(matches!(
        prepare_call(
            recorded,
            CallSpec::new("no_context", carrier, (), r::Field::from(0_u64))
        ),
        Err(PrepareCallError::EmptyTranscript)
    ));
}
