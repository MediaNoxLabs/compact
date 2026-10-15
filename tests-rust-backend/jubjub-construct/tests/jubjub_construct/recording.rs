// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_jubjub_construct_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Default)]
struct OrderedEcho(Rc<RefCell<Vec<(Field, u64)>>>);
impl c::TryWitnesses<u64> for OrderedEcho {
    fn echo(
        &self,
        ctx: WitnessContext<'_, u64, LedgerView<'_>>,
        value: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.0.borrow_mut().push((value, *ctx.private_state));
        Ok((*ctx.private_state + 1, value))
    }
}
fn setup() -> (ObservedContractState, VerifierKey) {
    // Installed empty key carrier reaches the empty-program preparation boundary;
    // this fixture does not claim a proof or a usable circuit verifier key.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"construct_echo".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    let ctx = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                ctx.query.state.get_ref().clone(),
                operations,
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}
#[test]
fn recorded_construct_echo_preserves_coordinate_order_and_checked_point_boundary() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/jubjub-construct.json"
    ))
    .unwrap();
    for (x, y, expected) in [
        (Field::from(0_u64), Field::from(1_u64), &oracle["identity"]),
        (
            field(&oracle["input"]["x"]),
            field(&oracle["input"]["y"]),
            &oracle["witness"]["result"],
        ),
    ] {
        let (state, key) = setup();
        let [a, b, w] = std::array::from_fn(|_| OrderedEcho::default());
        let native = c::construct_echo(state.circuit_context(7), &a, x, y).unwrap();
        let recorded = c::recorded::construct_echo(state.circuit_context(7), &b, x, y).unwrap();
        let contract = c::Contract::from(w.clone());
        let call = contract
            .recording()
            .construct_echo_call(&state, 7, x, y)
            .unwrap();
        assert_coords(native.result, expected);
        assert_eq!(native.context.private_state, 9);
        assert_eq!(
            native.private_transcript_outputs,
            vec![r::fab::AlignedValue::from(x), r::fab::AlignedValue::from(y)]
        );
        for candidate in [&recorded, call.recorded()] {
            assert_eq!(candidate.execution.result, native.result);
            assert_eq!(candidate.execution.context.private_state, 9);
            assert_eq!(
                candidate.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                candidate.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(
                candidate.execution.private_transcript_outputs,
                native.private_transcript_outputs
            );
            assert_eq!(candidate.execution.gas_cost, native.gas_cost);
            assert!(candidate.public.verify_ops().is_empty());
            let replay = candidate
                .public
                .initial()
                .query(
                    candidate.public.verify_ops(),
                    None,
                    &native.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
        }
        for witness in [&a, &b, &w] {
            assert_eq!(*witness.0.borrow(), vec![(x, 7), (y, 8)]);
        }
        assert!(matches!(
            call.prepare(key, Field::from(0_u64)),
            Err(ObservedCallError::Prepare(
                PrepareCallError::EmptyTranscript
            ))
        ));
    }
    // Existing Rust constructor boundary; TS merely stores these coordinates.
    // Both witnesses run exactly once before the typed constructor refusal.
    let (state, _) = setup();
    let [a, b, w] = std::array::from_fn(|_| OrderedEcho::default());
    let x = Field::from(1_u64);
    let y = Field::from(1_u64);
    let contract = c::Contract::from(w.clone());
    assert_eq!(
        c::construct_echo(state.circuit_context(7), &a, x, y)
            .err()
            .unwrap(),
        CompactError::InvalidJubjubPoint
    );
    assert_eq!(
        c::recorded::construct_echo(state.circuit_context(7), &b, x, y)
            .err()
            .unwrap(),
        CompactError::InvalidJubjubPoint
    );
    assert_eq!(
        contract
            .recording()
            .construct_echo_call(&state, 7, x, y)
            .err()
            .unwrap(),
        CompactError::InvalidJubjubPoint
    );
    for witness in [&a, &b, &w] {
        assert_eq!(*witness.0.borrow(), vec![(x, 7), (y, 8)]);
    }
    assert_eq!(state.circuit_context(7).private_state, 7);
}
