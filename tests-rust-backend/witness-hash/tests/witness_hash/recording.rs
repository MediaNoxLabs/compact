// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_witness_hash_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};
#[derive(Clone, Default)]
struct OrderedEcho(std::rc::Rc<std::cell::RefCell<Vec<(Field, u64)>>>);
impl Witnesses<u64> for OrderedEcho {
    fn echo(&self, ctx: WitnessContext<'_, u64, LedgerView<'_>>, value: Field) -> (u64, Field) {
        self.0.borrow_mut().push((value, *ctx.private_state));
        Echo.echo(ctx, value)
    }
    fn echo_bytes(
        &self,
        _ctx: WitnessContext<'_, u64, LedgerView<'_>>,
        _value: FixedBytes<32>,
    ) -> (u64, FixedBytes<32>) {
        panic!("degrade_hash_echo must only call the Field witness")
    }
}
fn observed_state() -> (ObservedContractState, VerifierKey) {
    // A serialized empty carrier is sufficient to reach EmptyTranscript; it is
    // not a circuit proof key and no proof is attempted here.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"degrade_hash_echo".to_vec()),
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
fn degraded_persistent_hash_records_one_witness_and_refuses_empty_preparation() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/witness-hash.json"
    ))
    .unwrap();
    let (state, key) = observed_state();
    let value = Field::from(2_u64);
    let a = OrderedEcho::default();
    let b = OrderedEcho::default();
    let w = OrderedEcho::default();
    let native = c::degrade_hash_echo(state.circuit_context(7), &a, value).unwrap();
    let recorded = c::recorded::degrade_hash_echo(state.circuit_context(7), &b, value).unwrap();
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .degrade_hash_echo_call(&state, 7, value)
        .unwrap();
    for actual in [&recorded, call.recorded()] {
        assert_oracle(
            &actual.execution,
            hex::encode(actual.execution.result.as_le_bytes()),
            &oracle["degradeHash"],
        );
        assert_eq!(actual.execution.result, native.result);
        assert_eq!(
            actual.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            actual.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            actual.execution.context.private_state,
            native.context.private_state
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
    }
    assert_oracle(
        &native,
        hex::encode(native.result.as_le_bytes()),
        &oracle["degradeHash"],
    );
    for witness in [&a, &b, &w] {
        assert_eq!(*witness.0.borrow(), vec![(value, 7)]);
    }
    assert!(matches!(
        call.prepare(key, Field::from(0_u64)),
        Err(ObservedCallError::Prepare(
            PrepareCallError::EmptyTranscript
        ))
    ));
}
