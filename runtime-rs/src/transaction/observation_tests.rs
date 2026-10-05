// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::ledger::{CoinInfo, CoinRecipient, QualifiedCoinInfo, StateValue};
use crate::recording::RecordingFrame;
use midnight_onchain_state::state::ContractMaintenanceAuthority;
use midnight_serialize::tagged_serialize;
use midnight_zswap::{Output, keys::SecretKeys};
use rand::{SeedableRng, rngs::StdRng};

struct Fixture {
    ledger: LedgerState<DefaultDB>,
    wallet: midnight_zswap::local::State<DefaultDB>,
    address: ContractAddress,
    contract_coin: QualifiedCoinInfo,
    wallet_input: Input<ProofPreimage, DefaultDB>,
    verifier: VerifierKey,
}
impl Fixture {
    fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(217);
        let keys = SecretKeys::from_rng_seed(&mut rng);
        let address = ContractAddress::default();
        let verifier: VerifierKey = rng.r#gen();
        let mut contract = ContractState::new(
            StateValue::Array(Default::default()),
            Default::default(),
            ContractMaintenanceAuthority::default(),
        );
        contract.operations = contract.operations.insert(
            EntryPointBuf(b"step".to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
        let mut ledger = LedgerState::new("local-test");
        ledger.contract = ledger.contract.insert(address, contract);
        let contract_coin = super::super::zswap_tests::coin(1);
        let wallet_coin = super::super::zswap_tests::coin(2);
        let contract_output =
            Output::new_contract_owned(&mut rng, &contract_coin, None, address).unwrap();
        let commitment = contract_output.coin_com;
        let seed = Offer::new(
            vec![],
            vec![
                contract_output,
                Output::new(
                    &mut rng,
                    &wallet_coin,
                    None,
                    &keys.coin_public_key(),
                    Some(keys.enc_public_key()),
                )
                .unwrap(),
                Output::new_contract_owned(
                    &mut rng,
                    &super::super::zswap_tests::coin(3),
                    None,
                    address,
                )
                .unwrap(),
            ],
            vec![],
        )
        .unwrap();
        let (state, indices) = ledger.zswap.try_apply(&seed, None).unwrap();
        ledger.zswap = Sp::new(state.post_block_update(Timestamp::from_secs(0)));
        let wallet = midnight_zswap::local::State::new().apply(&keys, &seed);
        let wallet_coin = *wallet.coins.iter().next().unwrap().1;
        let (_, wallet_input) = wallet.spend(&mut rng, &keys, &wallet_coin, None).unwrap();
        Self {
            ledger,
            wallet,
            address,
            contract_coin: contract_coin.qualify(*indices.get(&commitment).unwrap()),
            wallet_input,
            verifier,
        }
    }
    fn observation(&self) -> Observation {
        Observation {
            transaction_hash: [1; 32],
            block_hash: [2; 32],
            block_height: 3,
        }
    }
    fn observed(&self) -> ObservedContractState {
        ObservedContractState::new(
            self.address,
            self.ledger.contract.get(&self.address).unwrap().clone(),
            self.observation(),
        )
    }
    fn parts(&self) -> TrustedObservationParts {
        TrustedObservationParts {
            metadata: CheckpointMetadata {
                observation: self.observation(),
                address: self.address,
                network_id: "local-test".into(),
                ledger_version: "8.0.3".into(),
                zswap_root: self.ledger.zswap.coin_coms.rehash().root().unwrap(),
                first_free: self.ledger.zswap.first_free,
                final_zswap_event_id: 19,
            },
            node_contract: self.observed().contract.clone(),
            contract_tree: self.ledger.zswap.filter(&[self.address]),
            wallet: self.wallet.clone(),
            wallet_checkpoint: WalletCheckpointMetadata {
                block_hash: [2; 32],
                block_height: 3,
                network_id: "local-test".into(),
                ledger_version: "8.0.3".into(),
                applied_event_id: 19,
                wallet_state_sha256: midnight_base_crypto::hash::persistent_hash(&bytes(
                    &self.wallet,
                ))
                .0,
            },
        }
    }
    fn checkpoint(&self) -> TrustedObservationCheckpoint {
        TrustedObservationCheckpoint::from_trusted_sources(self.parts()).unwrap()
    }
    fn offer(&self, wallet: bool, outputs: &[CoinInfo]) -> Offer<ProofPreimage, DefaultDB> {
        let mut rng = StdRng::seed_from_u64(218);
        let input = if wallet {
            self.wallet_input.clone()
        } else {
            Input::new_contract_owned(
                &mut rng,
                &self.contract_coin,
                None,
                self.address,
                &self.ledger.zswap.coin_coms,
            )
            .unwrap()
        };
        Offer::new(
            vec![input],
            outputs
                .iter()
                .map(|coin| Output::new_contract_owned(&mut rng, coin, None, self.address).unwrap())
                .collect(),
            vec![],
        )
        .unwrap()
    }
    fn funding(&self, wallet: bool) -> Option<WalletFundingInputs> {
        wallet.then(|| WalletFundingInputs::from_inputs(vec![self.wallet_input.clone()]).unwrap())
    }
    fn record(
        &self,
        observed: &ObservedContractState,
        wallet: bool,
        outputs: &[CoinInfo],
    ) -> RecordedCircuitResult<(), ()> {
        let mut frame = RecordingFrame::new(observed.circuit_context(()));
        if !wallet {
            frame = frame.create_zswap_input(self.contract_coin);
        }
        for coin in outputs {
            frame = frame
                .create_zswap_output(*coin, CoinRecipient::Contract(self.address))
                .unwrap();
        }
        frame.kernel_self().unwrap().0.finish(())
    }
}
fn bytes<T: midnight_serialize::Serializable + midnight_serialize::Tagged>(value: &T) -> Vec<u8> {
    let mut bytes = vec![];
    tagged_serialize(value, &mut bytes).unwrap();
    bytes
}

#[test]
fn real_wallet_and_contract_inputs_have_equal_recorded_and_prepared_transactions() {
    let f = Fixture::new();
    assert_eq!(f.wallet.first_free, 3);
    for wallet in [false, true] {
        for reverse in [false, true] {
            let mut outputs = vec![
                super::super::zswap_tests::coin(4),
                super::super::zswap_tests::coin(5),
            ];
            if reverse {
                outputs.reverse();
            }
            let offer = f.offer(wallet, &outputs);
            let mut options = OfferBindingOptions::default()
                .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices);
            options.wallet_funding = f.funding(wallet);
            let full = OfferBackedObservedState::with_options(
                f.observed(),
                &f.ledger,
                offer.clone(),
                options,
            )
            .unwrap();
            let observed = ObservationalOfferBackedState::bind(
                f.observed(),
                f.checkpoint(),
                offer,
                f.funding(wallet),
            )
            .unwrap();
            assert_eq!(full.observed.com_indices, observed.observed.com_indices);
            assert_eq!(full.observed.allocation, observed.observed.allocation);
            let a = f.record(full.observed(), wallet, &outputs);
            let b = f.record(observed.observed(), wallet, &outputs);
            assert_eq!(a.public.verify_ops(), b.public.verify_ops());
            assert_eq!(a.execution.gas_cost, b.execution.gas_cost);
            assert_eq!(
                a.execution.private_transcript_outputs,
                b.execution.private_transcript_outputs
            );
            assert_eq!(
                a.execution.context.query.state,
                b.execution.context.query.state
            );
            assert_eq!(
                a.execution.context.query.effects,
                b.execution.context.query.effects
            );
            let a = full
                .prepare(
                    RecordedCall::new(full.observed(), a, "step", ()),
                    f.verifier.clone(),
                    Fr::from(7),
                )
                .unwrap();
            let b = observed
                .prepare(
                    RecordedCall::new(observed.observed(), b, "step", ()),
                    f.verifier.clone(),
                    Fr::from(7),
                )
                .unwrap();
            assert_eq!(a.admission(), &OfferAdmission::CompleteLedger);
            assert!(
                matches!(b.admission(),OfferAdmission::TrustedObservation(e) if e.metadata().first_free==3 && e.wallet().applied_event_id==19)
            );
            let a = a.into_transaction(
                &mut StdRng::seed_from_u64(5),
                "local-test",
                Timestamp::from_secs(100),
            );
            let b = b.into_transaction(&mut StdRng::seed_from_u64(5), Timestamp::from_secs(100));
            assert_eq!(bytes(&a), bytes(&b));
        }
    }
}

#[test]
fn checkpoint_mismatch_checks_do_not_claim_to_authenticate_network_provenance() {
    let f = Fixture::new();
    type CheckpointMutation = (ObservationBindingError, fn(&mut TrustedObservationParts));
    let mutations: Vec<CheckpointMutation> = vec![
        (ObservationBindingError::NetworkMismatch, |p| {
            p.wallet_checkpoint.network_id = "wrong".into()
        }),
        (ObservationBindingError::LedgerVersionMismatch, |p| {
            p.metadata.ledger_version = "8.1.0".into()
        }),
        (ObservationBindingError::BlockMismatch, |p| {
            p.wallet_checkpoint.block_hash = [3; 32]
        }),
        (ObservationBindingError::BlockMismatch, |p| {
            p.wallet_checkpoint.block_height += 1
        }),
        (ObservationBindingError::EventMismatch, |p| {
            p.wallet_checkpoint.applied_event_id += 1
        }),
        (ObservationBindingError::EventMismatch, |p| {
            p.wallet_checkpoint.applied_event_id -= 1
        }),
        (ObservationBindingError::RootMismatch, |p| {
            p.metadata.zswap_root = MerkleTreeDigest(Fr::from(12))
        }),
        (ObservationBindingError::RootMismatch, |p| {
            p.wallet.merkle_tree = MerkleTree::blank(midnight_zswap::ZSWAP_TREE_HEIGHT)
        }),
        (ObservationBindingError::FrontierMismatch, |p| {
            p.metadata.first_free += 1
        }),
        (ObservationBindingError::TreeHeightMismatch, |p| {
            p.contract_tree = MerkleTree::blank(4)
        }),
    ];
    for (expected, mutate) in mutations {
        let mut p = f.parts();
        mutate(&mut p);
        assert_eq!(
            TrustedObservationCheckpoint::from_trusted_sources(p).err(),
            Some(expected)
        );
    }
    // Equal endpoint claims can be dishonest; finality is an adapter obligation.
    let mut p = f.parts();
    p.metadata.observation.block_hash = [9; 32];
    p.wallet_checkpoint.block_hash = [9; 32];
    assert!(TrustedObservationCheckpoint::from_trusted_sources(p).is_ok());
}

#[test]
fn observation_offer_and_call_rejections_preserve_scope() {
    let f = Fixture::new();
    let outputs = vec![
        super::super::zswap_tests::coin(4),
        super::super::zswap_tests::coin(5),
    ];
    let original = f.offer(false, &outputs);
    let bind =
        |offer| ObservationalOfferBackedState::bind(f.observed(), f.checkpoint(), offer, None);
    let mut reversed = original.clone();
    let mut rows: Vec<_> = reversed.outputs.iter_deref().cloned().collect();
    rows.reverse();
    reversed.outputs = rows.into_iter().collect();
    assert_eq!(
        bind(reversed).err(),
        Some(ObservationBindingError::UnnormalizedOffer)
    );
    assert_eq!(
        bind(original.retarget_segment(1)).err(),
        Some(ObservationBindingError::SegmentMismatch)
    );
    let mut duplicate = original.clone();
    duplicate.outputs = vec![original.outputs.get(0).unwrap().clone(); 2]
        .into_iter()
        .collect();
    assert_eq!(
        bind(duplicate).err(),
        Some(ObservationBindingError::DuplicateCommitment)
    );
    let mut wrong_root = original.clone();
    let mut input = wrong_root.inputs.get(0).unwrap().clone();
    input.merkle_tree_root = MerkleTreeDigest(Fr::from(10));
    wrong_root.inputs = vec![input].into_iter().collect();
    assert_eq!(
        bind(wrong_root).err(),
        Some(ObservationBindingError::InputRootMismatch)
    );
    let mut dup_input = original.clone();
    dup_input.inputs = vec![original.inputs.get(0).unwrap().clone(); 2]
        .into_iter()
        .collect();
    assert_eq!(
        bind(dup_input).err(),
        Some(ObservationBindingError::DuplicateNullifier)
    );
    assert_eq!(
        bind(f.offer(true, &outputs)).err(),
        Some(ObservationBindingError::WalletFundingMismatch)
    );
    let mut changed = f.observed();
    changed.observation.transaction_hash = [7; 32];
    assert_eq!(
        ObservationalOfferBackedState::bind(changed, f.checkpoint(), original.clone(), None).err(),
        Some(ObservationBindingError::ContractMismatch)
    );
    let bound = bind(original.clone()).unwrap();
    let other = bind(original).unwrap();
    let call = RecordedCall::new(
        other.observed(),
        f.record(other.observed(), false, &outputs),
        "step",
        (),
    );
    assert!(matches!(
        bound.prepare(call, f.verifier.clone(), Fr::from(0)),
        Err(ObservedCallError::OfferMismatch)
    ));
    let empty = RecordingFrame::new(bound.observed().circuit_context(()))
        .kernel_self()
        .unwrap()
        .0
        .finish(());
    assert!(matches!(
        bound.prepare(
            RecordedCall::new(bound.observed(), empty, "step", ()),
            f.verifier.clone(),
            Fr::from(0)
        ),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::CanonicalEmptyPlan
        ))
    ));
    let mut record = f.record(bound.observed(), false, &outputs);
    record.execution.context.query.call_context.com_indices = Map::new();
    assert!(matches!(
        bound.prepare(
            RecordedCall::new(bound.observed(), record, "step", ()),
            f.verifier.clone(),
            Fr::from(0)
        ),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::AllocationMismatch
        ))
    ));
}

#[test]
fn missing_history_is_an_explicit_node_admission_obligation() {
    let f = Fixture::new();
    let offer = f.offer(false, &[super::super::zswap_tests::coin(4)]);
    let input = offer.inputs.get(0).unwrap();
    let mut spent = f.ledger.clone();
    let mut zswap = (*spent.zswap).clone();
    zswap.nullifiers = zswap.nullifiers.insert(input.nullifier, ());
    spent.zswap = Sp::new(zswap);
    assert!(
        OfferBackedObservedState::with_options(
            f.observed(),
            &spent,
            offer.clone(),
            OfferBindingOptions::default()
        )
        .is_err()
    );
    // The root and frontier did not change. The observation type deliberately
    // cannot know spent history: the real node/full ledger must reject replay.
    assert!(ObservationalOfferBackedState::bind(f.observed(), f.checkpoint(), offer, None).is_ok());
}

#[test]
fn output_only_bootstrap_is_supported_but_input_only_burns_are_not() {
    let f = Fixture::new();
    let output = super::super::zswap_tests::coin(4);
    let offered = f.offer(false, &[output]);
    let output_only = Offer::new(
        vec![],
        offered.outputs.iter_deref().cloned().collect(),
        vec![],
    )
    .unwrap();
    let full = OfferBackedObservedState::with_options(
        f.observed(),
        &f.ledger,
        output_only.clone(),
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices),
    )
    .unwrap();
    let observed =
        ObservationalOfferBackedState::bind(f.observed(), f.checkpoint(), output_only, None)
            .unwrap();
    let record = |observed: &ObservedContractState| {
        RecordingFrame::new(observed.circuit_context(()))
            .create_zswap_output(output, CoinRecipient::Contract(f.address))
            .unwrap()
            .kernel_self()
            .unwrap()
            .0
            .finish(())
    };
    let a = full
        .prepare(
            RecordedCall::new(full.observed(), record(full.observed()), "step", ()),
            f.verifier.clone(),
            Fr::from(0),
        )
        .unwrap();
    let b = observed
        .prepare(
            RecordedCall::new(observed.observed(), record(observed.observed()), "step", ()),
            f.verifier.clone(),
            Fr::from(0),
        )
        .unwrap();
    assert_eq!(
        bytes(&a.into_transaction(
            &mut StdRng::seed_from_u64(0),
            "local-test",
            Timestamp::from_secs(100)
        )),
        bytes(&b.into_transaction(&mut StdRng::seed_from_u64(0), Timestamp::from_secs(100)))
    );
    let input_only = Offer::new(
        offered.inputs.iter_deref().cloned().collect(),
        vec![],
        vec![],
    )
    .unwrap();
    assert_eq!(
        ObservationalOfferBackedState::bind(f.observed(), f.checkpoint(), input_only, None).err(),
        Some(ObservationBindingError::UnsupportedOffer)
    );
    // This is structural bootstrap preparation, not proof that arbitrary mint
    // deltas balance. Generated strict receive/spend proof tests are separate.
}

#[test]
fn funding_identity_owner_transient_and_capacity_boundaries() {
    let f = Fixture::new();
    let output = super::super::zswap_tests::coin(4);
    let offer = f.offer(true, &[output]);
    let mut selected = f.wallet_input.clone();
    std::sync::Arc::make_mut(&mut selected.proof)
        .private_transcript
        .push(Fr::from(1));
    assert_eq!(
        ObservationalOfferBackedState::bind(
            f.observed(),
            f.checkpoint(),
            offer.clone(),
            Some(WalletFundingInputs::from_inputs(vec![selected]).unwrap())
        )
        .err(),
        Some(ObservationBindingError::WalletFundingMismatch)
    );
    let mut selected = f.wallet_input.clone();
    selected.nullifier = crate::ledger::CoinNullifier(crate::ledger::HashOutput([99; 32]));
    assert_eq!(
        ObservationalOfferBackedState::bind(
            f.observed(),
            f.checkpoint(),
            offer.clone(),
            Some(WalletFundingInputs::from_inputs(vec![f.wallet_input.clone(), selected]).unwrap())
        )
        .err(),
        Some(ObservationBindingError::WalletFundingMismatch)
    );
    let mut parts = f.parts();
    parts.node_contract.data = midnight_onchain_state::state::ChargedState::new(StateValue::Null);
    assert_eq!(
        ObservationalOfferBackedState::bind(
            f.observed(),
            TrustedObservationCheckpoint::from_trusted_sources(parts).unwrap(),
            offer.clone(),
            f.funding(true)
        )
        .err(),
        Some(ObservationBindingError::ContractMismatch)
    );
    let mut foreign = f.offer(false, &[output]);
    let mut input = foreign.inputs.get(0).unwrap().clone();
    input.contract_address = Some(Sp::new(ContractAddress(crate::ledger::HashOutput(
        [11; 32],
    ))));
    foreign.inputs = vec![input].into_iter().collect();
    assert_eq!(
        ObservationalOfferBackedState::bind(f.observed(), f.checkpoint(), foreign, None).err(),
        Some(ObservationBindingError::InputOwnerMismatch)
    );
    let mut rng = StdRng::seed_from_u64(219);
    let transient_coin = super::super::zswap_tests::coin(6);
    let transient_output =
        Output::new_contract_owned(&mut rng, &transient_coin, None, f.address).unwrap();
    let transient = midnight_zswap::Transient::new_from_contract_owned_output(
        &mut rng,
        &transient_coin.qualify(0),
        None,
        transient_output,
    )
    .unwrap();
    let mut transient_offer = offer.clone();
    transient_offer.transient = vec![transient].into_iter().collect();
    assert_eq!(
        ObservationalOfferBackedState::bind(
            f.observed(),
            f.checkpoint(),
            transient_offer,
            f.funding(true)
        )
        .err(),
        Some(ObservationBindingError::UnsupportedOffer)
    );
    // Matching caller claims do not prove frontier truth; nevertheless bounded
    // allocation must fail before recording if the requested indices overflow.
    let mut parts = f.parts();
    parts.metadata.first_free = 1u64 << midnight_zswap::ZSWAP_TREE_HEIGHT;
    parts.wallet.first_free = parts.metadata.first_free;
    assert_eq!(
        ObservationalOfferBackedState::bind(
            f.observed(),
            TrustedObservationCheckpoint::from_trusted_sources(parts).unwrap(),
            offer,
            f.funding(true)
        )
        .err(),
        Some(ObservationBindingError::FrontierMismatch)
    );
    let mut parts = f.parts();
    parts.metadata.first_free = u64::MAX;
    parts.wallet.first_free = u64::MAX;
    assert_eq!(
        TrustedObservationCheckpoint::from_trusted_sources(parts).err(),
        Some(ObservationBindingError::FrontierMismatch)
    );
}
