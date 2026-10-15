// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots::items};
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::context::{ConstructorContext, WitnessContext};
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use std::{cell::RefCell, rc::Rc};
type Events = Rc<RefCell<Vec<(u64, Option<r::Field>)>>>;
#[derive(Clone, Default)]
struct Witness {
    fail: bool,
    events: Events,
}
impl Witness {
    fn observe(
        &self,
        private: u64,
        head: Option<r::Field>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.events.borrow_mut().push((private, head));
        if self.fail {
            return Err(r::CompactError::InvalidLedgerCell(
                "push provider refused".into(),
            ));
        }
        Ok((private + 1, head == Some(r::Field::from(42_u64))))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn first_is_42(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.observe(*ctx.private_state, ctx.ledger.items()?.head()?)
    }
}
fn setup(populated: bool) -> (ObservedContractState, VerifierKey) {
    let initial = c::initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    let state = if populated {
        items
            .push_front(initial, r::Field::from(42_u64))
            .unwrap()
            .context
            .query
            .state
            .get_ref()
            .clone()
    } else {
        initial.query.state.get_ref().clone()
    };
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"prepend_witnessed".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(state, operations, Default::default()),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}
fn manual(
    state: &ObservedContractState,
    w: &Witness,
    selected: bool,
    value: r::Field,
) -> Result<r::recording::RecordedCircuitResult<u64, ()>, r::CompactError> {
    let mut frame = RecordingFrame::new(state.circuit_context(7));
    let value = if selected {
        let (next, first) = frame.try_witness_metered(|ctx, meter| {
            w.observe(ctx.private_state, items.witness_view(meter)?.head()?)
        })?;
        frame = next;
        if first { value } else { r::Field::from(0_u64) }
    } else {
        r::Field::from(0_u64)
    };
    Ok(items.record_push_front(frame, value)?.finish(()))
}
#[test]
fn lazy_witness_push_matches_manual_order_private_fab_and_preparation() {
    for (selected, populated) in [(true, true), (true, false), (false, true)] {
        let (state, key) = setup(populated);
        let value = r::Field::from(7_u64);
        let a = Witness::default();
        let b = Witness::default();
        let w = Witness::default();
        let m = Witness::default();
        let native = c::prepend_witnessed(state.circuit_context(7), &a, selected, value).unwrap();
        let recorded =
            c::recorded::prepend_witnessed(state.circuit_context(7), &b, selected, value).unwrap();
        let contract = c::Contract::from(w.clone());
        let call = contract
            .recording()
            .prepend_witnessed_call(&state, 7, selected, value)
            .unwrap();
        let manual = manual(&state, &m, selected, value).unwrap();
        let expected_outputs = if selected {
            vec![r::fab::AlignedValue::from(populated)]
        } else {
            vec![]
        };
        let expected_events = if selected {
            vec![(7, populated.then(|| r::Field::from(42_u64)))]
        } else {
            vec![]
        };
        assert_eq!(native.private_transcript_outputs, expected_outputs);
        assert_eq!(native.context.private_state, 7 + u64::from(selected));
        let expected_replay = manual
            .public
            .initial()
            .query(manual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        for actual in [&recorded, call.recorded(), &manual] {
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert_eq!(
                actual.execution.context.private_state,
                native.context.private_state
            );
            assert_eq!(
                actual.execution.private_transcript_outputs,
                expected_outputs
            );
            assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, expected_replay.gas_cost);
        }
        for provider in [&a, &b, &w, &m] {
            assert_eq!(*provider.events.borrow(), expected_events);
        }
        let list = items.inspect(native.context.query.state.get_ref()).unwrap();
        assert_eq!(list.length().unwrap().value(), 1 + u128::from(populated));
        assert_eq!(
            list.head().unwrap(),
            Some(if !selected || !populated {
                r::Field::from(0_u64)
            } else {
                value
            })
        );
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(
            prepared.input,
            r::fab::AlignedValue::from((selected, value))
        );
        assert_eq!(prepared.output, r::fab::AlignedValue::from(()));
    }
}
#[test]
fn provider_failure_precedes_push_and_unselected_operand_never_calls_it() {
    for selected in [true, false] {
        for lane in 0..4 {
            let (state, _) = setup(true);
            let value = r::Field::from(7_u64);
            let w = Witness {
                fail: true,
                ..Default::default()
            };
            let contract = c::Contract::from(w.clone());
            let result = match lane {
                0 => {
                    c::prepend_witnessed(state.circuit_context(7), &w, selected, value).map(|_| ())
                }
                1 => c::recorded::prepend_witnessed(state.circuit_context(7), &w, selected, value)
                    .map(|_| ()),
                2 => contract
                    .recording()
                    .prepend_witnessed_call(&state, 7, selected, value)
                    .map(|_| ()),
                _ => manual(&state, &w, selected, value).map(|_| ()),
            };
            if selected {
                assert!(
                    matches!(result,Err(r::CompactError::InvalidLedgerCell(ref detail)) if detail=="push provider refused")
                );
                assert_eq!(*w.events.borrow(), vec![(7, Some(r::Field::from(42_u64)))]);
            } else {
                assert!(result.is_ok());
                assert!(w.events.borrow().is_empty());
            }
            // The callback read the supplied old head; on refusal no result/private
            // output or updated context is committed by any lane.
            assert_eq!(
                items
                    .inspect(state.contract().data.get_ref())
                    .unwrap()
                    .head()
                    .unwrap(),
                Some(r::Field::from(42_u64))
            );
        }
    }
}
