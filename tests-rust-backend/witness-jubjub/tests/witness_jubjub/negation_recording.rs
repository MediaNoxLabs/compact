// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_witness_jubjub_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
struct PointWitness {
    point: JubjubPoint,
    fail: bool,
    events: Rc<RefCell<Vec<(Field, u64)>>>,
}
impl PointWitness {
    fn new(point: JubjubPoint, fail: bool) -> Self {
        Self {
            point,
            fail,
            events: Rc::default(),
        }
    }
}
fn failure() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("negation point provider refused".into())
}
impl c::TryWitnesses<u64> for PointWitness {
    fn secret_point(
        &self,
        ctx: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> Result<(u64, JubjubPoint), r::CompactError> {
        self.events.borrow_mut().push((seed, *ctx.private_state));
        if self.fail {
            return Err(failure());
        }
        Ok((*ctx.private_state + 1, self.point))
    }
    fn secret_scalar(
        &self,
        _: WitnessContext<'_, u64, LedgerView<'_>>,
        _: Field,
    ) -> Result<(u64, Field), r::CompactError> {
        panic!("negate must not request a scalar witness")
    }
}
fn frozen_point(row: &serde_json::Value) -> JubjubPoint {
    let coordinate =
        |name| Field::from_le_bytes(&hex::decode(row[name].as_str().unwrap()).unwrap()).unwrap();
    let value = r::fab::Value::from((coordinate("x"), coordinate("y")));
    JubjubPoint::try_from(&*value).unwrap()
}
fn observed_state() -> (ObservedContractState, VerifierKey) {
    // A serialized empty installed carrier reaches the intended empty-program
    // boundary; it is not a circuit proof key and no proof is attempted.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"negate".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                context().query.state.get_ref().clone(),
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
fn retained_negation_records_point_once_and_preserves_empty_preparation_boundary() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/witness-jubjub.json"
    ))
    .unwrap();
    let seed = Field::from(2_u64);
    // The retained TS reveal/negate rows share seed2/private7. Identity is an
    // additional official curve boundary, not a newly captured TS scenario.
    for (point, expected, ts) in [
        (JubjubPoint::identity(), JubjubPoint::identity(), false),
        (
            frozen_point(&oracle["reveal"]["result"]),
            frozen_point(&oracle["negate"]["result"]),
            true,
        ),
    ] {
        let (state, key) = observed_state();
        let [a, b, w] = std::array::from_fn(|_| PointWitness::new(point, false));
        let native = c::negate(state.circuit_context(7), &a, seed).unwrap();
        let recorded = c::recorded::negate(state.circuit_context(7), &b, seed).unwrap();
        let contract = c::Contract::from(w.clone());
        let call = contract.recording().negate_call(&state, 7, seed).unwrap();
        assert_eq!(native.result, expected);
        assert_eq!(native.context.private_state, 8);
        assert_eq!(
            native.private_transcript_outputs,
            vec![r::fab::AlignedValue::from(point)]
        );
        for actual in [&recorded, call.recorded()] {
            assert_eq!(actual.execution.result, expected);
            assert_eq!(
                actual.execution.context.private_state,
                native.context.private_state
            );
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(
                actual.execution.private_transcript_outputs,
                native.private_transcript_outputs
            );
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert!(actual.public.verify_ops().is_empty());
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            if ts {
                assert_oracle(
                    &actual.execution,
                    point_json(actual.execution.result),
                    &oracle["negate"],
                );
            }
        }
        if ts {
            assert_oracle(&native, point_json(native.result), &oracle["negate"]);
        }
        for witness in [&a, &b, &w] {
            assert_eq!(*witness.events.borrow(), vec![(seed, 7)]);
        }
        assert!(matches!(
            call.prepare(key, Field::from(0_u64)),
            Err(ObservedCallError::Prepare(
                PrepareCallError::EmptyTranscript
            ))
        ));
    }
    let (state, _) = observed_state();
    let [a, b, w] = std::array::from_fn(|_| PointWitness::new(JubjubPoint::identity(), true));
    let contract = c::Contract::from(w.clone());
    assert_eq!(
        c::negate(state.circuit_context(7), &a, seed).err().unwrap(),
        failure()
    );
    assert_eq!(
        c::recorded::negate(state.circuit_context(7), &b, seed)
            .err()
            .unwrap(),
        failure()
    );
    assert_eq!(
        contract
            .recording()
            .negate_call(&state, 7, seed)
            .err()
            .unwrap(),
        failure()
    );
    for witness in [&a, &b, &w] {
        assert_eq!(*witness.events.borrow(), vec![(seed, 7)]);
    }
    assert_eq!(state.circuit_context(7).private_state, 7);
}
