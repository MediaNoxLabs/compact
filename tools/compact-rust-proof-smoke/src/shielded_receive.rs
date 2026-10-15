// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Prove the unchanged receiveShielded wrapper and apply its exact zero output.
use super::*;
use compact_rust_shielded_receive_oracle_fixture::{ledger_contract as contract, types};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Offer, Output};
use runtime::transaction::{OfferBackedObservedState, ZswapIntentError};

pub(super) fn observed(
    address: runtime::ledger::ContractAddress,
    state: ContractState<DefaultDB>,
) -> ObservedContractState {
    ObservedContractState::new(
        address,
        state,
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}
pub(super) fn coin(nonce: u8, value: u128) -> types::ShieldedCoinInfo {
    let mut bytes = [0; 32];
    bytes[0] = nonce;
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new(bytes),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(value).unwrap(),
    }
}

fn validate_source_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let expected: serde_json::Value =
        serde_json::from_str(compact_rust_shielded_receive_oracle_fixture::RETAINED_IR)?;
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    if actual != expected {
        return Err("shielded receive source/proof IR mismatch".into());
    }
    Ok(())
}

/// Bind the complete source, including the renamed export and helper bodies,
/// before any of the receive scenarios reads deployment or proving artifacts.
pub(super) fn ensure_source_artifacts(root: &Path) -> Result<(), Box<dyn Error>> {
    validate_source_ir(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    ensure_source_artifacts(root)?;
    let mut rng = StdRng::seed_from_u64(0x0195_0000);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "accept",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let address = deploy.address();
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/accept.verifier"),
    )?))?;
    let mut state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    state.ledger.contract = state
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let generated = contract::Contract::default();
    let zero = coin(1, 0);
    let zero_info = runtime::ledger::coin_info_from_compact(zero.nonce, zero.color, 0);
    let output = Output::new_contract_owned(&mut rng, &zero_info, None, address)?;
    let commitment = output.coin_com;
    let offer = Offer::new(vec![], vec![output], vec![]).ok_or("zero receive offer missing")?;
    let bound = OfferBackedObservedState::new(
        observed(address, deploy.initial_state.clone()),
        &state.ledger,
        offer,
    )?;
    let call = generated
        .recording()
        .accept_call(bound.observed(), (), zero.clone())?;
    assert_eq!(call.recorded().execution.result, ());
    assert_eq!(call.recorded().public.verify_ops().len(), 9);
    assert_eq!(
        call.recorded().execution.private_transcript_outputs.len(),
        1
    );
    assert_eq!(
        call.recorded()
            .execution
            .context
            .circuit_zswap()
            .outputs()
            .len(),
        1
    );
    assert_eq!(
        call.recorded()
            .execution
            .context
            .query
            .call_context
            .com_indices
            .get(&commitment),
        Some(&0)
    );
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
    // A valid Compact/ledger u128 value beyond TypeScript's current b8
    // output descriptor still has a complete generated Rust call proof.
    // This output-only offer is not asserted to be balance-valid.
    let wide = coin(4, 1u128 << 64);
    let wide_info =
        runtime::ledger::coin_info_from_compact(wide.nonce, wide.color, wide.value.value());
    let wide_output = Output::new_contract_owned(&mut rng, &wide_info, None, address)?;
    let wide_offer = Offer::new(vec![], vec![wide_output], vec![]).ok_or("wide offer missing")?;
    let wide_bound = OfferBackedObservedState::new(
        observed(address, deploy.initial_state.clone()),
        &state.ledger,
        wide_offer,
    )?;
    let wide_call = generated
        .recording()
        .accept_call(wide_bound.observed(), (), wide)?;
    let wide_prepared = wide_bound.prepare(wide_call, verifier.clone(), Fr::from(0))?;
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        "accept",
        wide_prepared.prototype(),
        &verifier,
    )?;
    let _updated = super::composite_zswap::apply_bound(
        root,
        "accept",
        prepared,
        state,
        rng.clone(),
        &verifier,
        commitment,
        0,
        address,
    )?;

    // A wallet-funded receive has an external input. Exact contract-owned
    // intent policy must reject it, even when its output is the right contract
    // coin and the wallet can furnish a valid input proof.
    let mut wallet_ledger = super::qualified_coin_funding::fee_funded_state(&mut rng)?.ledger;
    let positive = coin(3, 42);
    let positive_info = runtime::ledger::coin_info_from_compact(positive.nonce, positive.color, 42);
    let funded = super::qualified_coin_funding::seed_and_fund(
        &mut rng,
        &mut wallet_ledger,
        positive_info,
        runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([7; 32])),
        Timestamp::from_secs(0),
    )?;
    wallet_ledger.contract = wallet_ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let wallet_input = funded
        .offer
        .inputs
        .iter_deref()
        .next()
        .cloned()
        .ok_or("wallet input missing")?;
    let contract_output = Output::new_contract_owned(&mut rng, &positive_info, None, address)?;
    let wallet_offer = Offer::new(vec![wallet_input], vec![contract_output], vec![])
        .ok_or("wallet funded contract offer missing")?;
    let wallet_bound = OfferBackedObservedState::new(
        observed(address, deploy.initial_state),
        &wallet_ledger,
        wallet_offer,
    )?;
    let call = generated
        .recording()
        .accept_call(wallet_bound.observed(), (), positive)?;
    assert!(matches!(
        wallet_bound.prepare(call, verifier, Fr::from(0)),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::InputMismatch
        ))
    ));
    println!(
        "receiveShielded: zero and wide call proofs verified; zero contract output applied with separate Dust; external wallet input rejected by exact policy"
    );
    Ok(())
}

#[cfg(test)]
mod source_pairing_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    const SWAPPED_IR: &[u8] =
        include_bytes!("../../compact-rust-backend/tests/shielded-send-schema20-ir.json");
    fn altered_renamed_export() -> Vec<u8> {
        let mut ir: serde_json::Value =
            serde_json::from_str(compact_rust_shielded_receive_oracle_fixture::RETAINED_IR)
                .unwrap();
        let renamed = ir["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|circuit| circuit["name"] == "accept_renamed")
            .unwrap();
        renamed["name"] = "changed_renamed_export".into();
        serde_json::to_vec(&ir).unwrap()
    }
    #[test]
    fn full_receive_source_pairing_rejects_swapped_or_mutated_ir() {
        let source = compact_rust_shielded_receive_oracle_fixture::RETAINED_IR;
        validate_source_ir(source.as_bytes()).unwrap();
        let formatted: serde_json::Value = serde_json::from_str(source).unwrap();
        validate_source_ir(&serde_json::to_vec(&formatted).unwrap()).unwrap();
        let altered = altered_renamed_export();
        for invalid in [SWAPPED_IR, altered.as_slice(), b"{}", b"not JSON"] {
            assert!(validate_source_ir(invalid).is_err());
        }
    }

    struct ArtifactRoot(std::path::PathBuf);
    impl ArtifactRoot {
        fn new(ir: &[u8]) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "compact-receive-source-pairing-{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            std::fs::create_dir(root.join("contract")).unwrap();
            std::fs::write(root.join("contract/compact-rust-ir.json"), ir).unwrap();
            Self(root)
        }
    }
    impl Drop for ArtifactRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn every_receive_entry_rejects_source_mismatch_before_deployment_or_keys() {
        type Entry = fn(&Path) -> Result<(), Box<dyn Error>>;
        let entries: [Entry; 3] = [
            run,
            crate::wallet_funded_receive::run,
            crate::wallet_funded_receive::run_observational,
        ];
        let altered = altered_renamed_export();
        for invalid in [SWAPPED_IR, altered.as_slice()] {
            let root = ArtifactRoot::new(invalid);
            assert!(!root.0.join("keys").exists());
            for entry in entries {
                let error = entry(&root.0).unwrap_err();
                assert_eq!(
                    error.to_string(),
                    "shielded receive source/proof IR mismatch"
                );
            }
        }
    }
}
