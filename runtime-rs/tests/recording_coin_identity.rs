// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{
    CircuitContext, ConstructorContext, ConstructorResult, RunningCost,
};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, StateValue, constructor_cell,
};
use midnight_compact_runtime::recording::RecordingFrame;

fn context() -> CircuitContext<u64, DefaultDB> {
    let state = StateValue::Array(vec![constructor_cell::<_, DefaultDB>(false)].into());
    ConstructorResult::new(ConstructorContext::new(7), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default())
}

#[test]
fn own_key_preserves_native_encoding_order_state_and_query_cost() {
    let key = [11; 32];
    let execution = context().with_coin_public_key_bytes(key);
    let (frame, first) =
        RecordingFrame::new(execution).witness(|context| (context.private_state + 1, [7_u8; 32]));
    assert_eq!(first, [7; 32]);
    let (frame, own) = frame.own_coin_public_key().unwrap();
    assert_eq!(own, key);
    let (frame, _) = frame.kernel_self().unwrap();
    let (frame, own_again) = frame.own_coin_public_key().unwrap();
    assert_eq!(own_again, key);
    let (frame, last) = frame.witness(|context| (context.private_state + 1, true));
    assert!(last);
    let recorded = frame.finish(());
    let (control, _) = RecordingFrame::new(context().with_coin_public_key_bytes(key))
        .kernel_self()
        .unwrap();
    let control = control.finish(());
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        vec![
            AlignedValue::from([7_u8; 32]),
            AlignedValue::from(key),
            AlignedValue::from(key),
            AlignedValue::from(true),
        ]
    );
    assert_eq!(recorded.execution.context.private_state, 9);
    assert_eq!(recorded.execution.gas_cost, control.execution.gas_cost);
    assert_eq!(recorded.public.verify_ops(), control.public.verify_ops());
    assert_eq!(
        recorded.execution.context.query.state,
        control.execution.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        control.execution.context.query.effects
    );
    assert_eq!(recorded.public.initial_coin_public_key(), Some(key));
    assert_eq!(recorded.public.final_coin_public_key(), Some(key));
}

#[test]
fn missing_identity_uses_native_error_and_key_only_has_no_public_transcript() {
    assert!(matches!(
        context().own_coin_public_key(),
        Err(CompactError::MissingCoinPublicKey)
    ));
    assert!(matches!(
        RecordingFrame::new(context()).own_coin_public_key(),
        Err(CompactError::MissingCoinPublicKey)
    ));
    let (frame, key) = RecordingFrame::new(context().with_coin_public_key_bytes([0; 32]))
        .own_coin_public_key()
        .unwrap();
    let recorded = frame.finish(key);
    assert!(recorded.public.verify_ops().is_empty());
    assert_eq!(recorded.execution.gas_cost, RunningCost::ZERO);
    assert_eq!(recorded.execution.context.private_state, 7);
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        vec![AlignedValue::from([0_u8; 32])]
    );
}

#[test]
fn execution_context_replacement_cannot_change_sealed_identity() {
    let (frame, _) = RecordingFrame::new(context().with_coin_public_key_bytes([11; 32]))
        .own_coin_public_key()
        .unwrap();
    let mut recorded = frame.finish(());
    recorded.execution.context = recorded
        .execution
        .context
        .with_coin_public_key_bytes([23; 32]);
    assert_eq!(recorded.public.initial_coin_public_key(), Some([11; 32]));
    assert_eq!(recorded.public.final_coin_public_key(), Some([11; 32]));
    assert_eq!(
        recorded.execution.context.own_coin_public_key().unwrap(),
        [23; 32]
    );
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn successful_key_only_recording_still_refuses_empty_public_preparation() {
    use midnight_compact_runtime::transaction::{CallSpec, PrepareCallError, prepare_call};
    use midnight_transient_crypto::curve::Fr;
    use rand::{Rng, SeedableRng, rngs::StdRng};
    let (frame, key) = RecordingFrame::new(context().with_coin_public_key_bytes([11; 32]))
        .own_coin_public_key()
        .unwrap();
    let recorded = frame.finish(key);
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 1);
    let spec = CallSpec::new(
        "own_key",
        StdRng::seed_from_u64(206).r#gen(),
        (),
        Fr::from(0),
    );
    assert!(matches!(
        prepare_call(recorded, spec),
        Err(PrepareCallError::EmptyTranscript)
    ));
}

#[cfg(feature = "ledger-transaction")]
mod preparation {
    use super::*;
    use midnight_compact_runtime::ledger::{CoinPublicKey, ContractState, HashOutput};
    use midnight_compact_runtime::transaction::{
        CallSpec, Observation, ObservedCallError, ObservedContractState, OfferBackedObservedState,
        PrepareCallError, RecordedCall, VerifierKey, prepare_call,
    };
    use midnight_onchain_state::state::{
        ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
    };
    use midnight_transient_crypto::curve::Fr;
    use rand::{Rng, SeedableRng, rngs::StdRng};

    fn observed(key: Option<u8>, verifier: &VerifierKey) -> ObservedContractState {
        let operations = midnight_storage::storage::HashMap::new().insert(
            EntryPointBuf(b"identity".to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
        let contract = ContractState::new(
            context().query.state.get_ref().clone(),
            operations,
            ContractMaintenanceAuthority::default(),
        );
        let observed = ObservedContractState::new(
            ContractAddress::default(),
            contract,
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        match key {
            Some(key) => observed.with_coin_public_key(CoinPublicKey(HashOutput([key; 32]))),
            None => observed,
        }
    }

    #[test]
    fn generic_prepare_rejects_replaced_execution_identity() {
        for replacement in [Some(23), None] {
            let (frame, _) = RecordingFrame::new(context().with_coin_public_key_bytes([11; 32]))
                .own_coin_public_key()
                .unwrap();
            let (frame, _) = frame.kernel_self().unwrap();
            let mut recorded = frame.finish(());
            recorded.execution.context = match replacement {
                Some(key) => recorded
                    .execution
                    .context
                    .with_coin_public_key_bytes([key; 32]),
                None => context(),
            };
            let spec = CallSpec::new(
                "identity",
                StdRng::seed_from_u64(206).r#gen(),
                (),
                Fr::from(0),
            );
            assert!(matches!(
                prepare_call(recorded, spec),
                Err(PrepareCallError::CoinPublicKeyMismatch)
            ));
        }
    }

    #[test]
    fn public_recorded_call_construction_cannot_rebind_identity() {
        let verifier: VerifierKey = StdRng::seed_from_u64(206).r#gen();
        let source = observed(Some(11), &verifier);
        for target in [Some(11), Some(23), None] {
            let target = observed(target, &verifier);
            let (frame, _) = RecordingFrame::new(source.circuit_context(7_u64))
                .own_coin_public_key()
                .unwrap();
            let (frame, _) = frame.kernel_self().unwrap();
            let call = RecordedCall::new(&target, frame.finish(()), "identity", ());
            let result = call.prepare(verifier.clone(), Fr::from(0));
            if target.circuit_context(()).own_coin_public_key().ok() == Some([11; 32]) {
                assert!(result.is_ok());
            } else {
                assert!(matches!(
                    result,
                    Err(ObservedCallError::CoinPublicKeyMismatch)
                ));
            }
        }
        // Replacing the public context with the target's key cannot rewrite sealed entry metadata.
        let target = observed(Some(23), &verifier);
        let (frame, _) = RecordingFrame::new(source.circuit_context(7_u64))
            .own_coin_public_key()
            .unwrap();
        let (frame, _) = frame.kernel_self().unwrap();
        let mut recorded = frame.finish(());
        recorded.execution.context = recorded
            .execution
            .context
            .with_coin_public_key_bytes([23; 32]);
        let call = RecordedCall::new(&target, recorded, "identity", ());
        assert!(matches!(
            call.prepare(verifier, Fr::from(0)),
            Err(ObservedCallError::CoinPublicKeyMismatch)
        ));
    }

    #[test]
    fn default_offer_preparation_enforces_final_identity_without_changing_empty_plan_policy() {
        use midnight_ledger::structure::LedgerState;
        use midnight_transient_crypto::proofs::ProofPreimage;
        use midnight_zswap::{Offer, Output};
        let mut rng = StdRng::seed_from_u64(206);
        let verifier: VerifierKey = rng.r#gen();
        let observed = observed(Some(11), &verifier);
        let mut ledger = LedgerState::new("local-test");
        ledger.contract = ledger
            .contract
            .insert(observed.address(), observed.contract().clone());
        let coin = midnight_compact_runtime::ledger::coin_info_from_compact(
            midnight_compact_runtime::FixedBytes::new([3; 32]),
            midnight_compact_runtime::FixedBytes::new([2; 32]),
            42,
        );
        let output = Output::new_contract_owned(&mut rng, &coin, None, observed.address()).unwrap();
        let offer: Offer<ProofPreimage, DefaultDB> =
            Offer::new(vec![], vec![output], vec![]).unwrap();
        let bound = OfferBackedObservedState::new(observed, &ledger, offer).unwrap();
        for changed in [false, true] {
            let (frame, _) = RecordingFrame::new(bound.observed().circuit_context(7_u64))
                .own_coin_public_key()
                .unwrap();
            let (frame, _) = frame.kernel_self().unwrap();
            let mut recorded = frame.finish(());
            if changed {
                recorded.execution.context = recorded
                    .execution
                    .context
                    .with_coin_public_key_bytes([23; 32]);
            }
            let call = RecordedCall::new(bound.observed(), recorded, "identity", ());
            let result = bound.prepare(call, verifier.clone(), Fr::from(0));
            if changed {
                assert!(matches!(
                    result,
                    Err(ObservedCallError::Prepare(
                        PrepareCallError::CoinPublicKeyMismatch
                    ))
                ));
            } else {
                assert!(result.is_ok());
            }
        }
    }
}
