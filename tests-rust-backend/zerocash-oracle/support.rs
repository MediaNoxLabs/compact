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

#![allow(
    dead_code,
    reason = "shared proof tests use successful variants and execution tests use rejection modes"
)]
use compact_rust_zerocash_oracle_fixture::{ledger_contract as contract, types::*};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::{CompactError, FixedBytes, OpaqueBytes};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Private {
    pub calls: u64,
    pub next: u8,
    pub owned: u64,
}
impl Default for Private {
    fn default() -> Self {
        Self {
            calls: 0,
            next: 3,
            owned: 0,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Normal,
    WrongRoot,
    WrongLeaf,
    Malformed,
    EncryptionFailure,
}
#[derive(Default)]
pub struct Witness {
    pub mode: Mode,
    pub path: Option<MerkleTreePath>,
    pub calls: RefCell<Vec<&'static str>>,
}
pub fn coin(n: u8) -> coin_info {
    coin_info {
        nonce: Nonce {
            bytes: FixedBytes::new([n; 32]),
        },
        opening: opening {
            bytes: FixedBytes::new([n + 1; 32]),
        },
    }
}
pub fn public_key() -> zk_public_key {
    zk_public_key {
        bytes: runtime::persistent_hash(FixedBytes::new([1; 32])),
    }
}
pub fn commitment_of(coin: &coin_info, key: zk_public_key) -> commitment {
    let mut prefix = [0; 32];
    prefix[..21].copy_from_slice(b"lares:zerocash:commit");
    commitment {
        bytes: runtime::persistent_hash((
            FixedBytes::new(prefix),
            coin.nonce.bytes,
            coin.opening.bytes,
            key.bytes,
        )),
    }
}
pub fn destination() -> public_key {
    public_key {
        zk: zk_public_key {
            bytes: FixedBytes::new([9; 32]),
        },
        encryption: OpaqueBytes::from(vec![10, 11, 12]),
    }
}
pub fn ciphertext(key: &OpaqueBytes, coin: &coin_info) -> OpaqueBytes {
    let mut bytes = key.0.clone();
    bytes.extend([coin.nonce.bytes.0[0], coin.opening.bytes.0[0], 255]);
    bytes.into()
}
impl Witness {
    fn step(
        &self,
        name: &'static str,
        context: &WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Private {
        self.calls.borrow_mut().push(name);
        let mut state = context.private_state.clone();
        state.calls += 1;
        state
    }
}
impl contract::TryWitnesses<Private> for Witness {
    fn private_zk_secret_key(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, zk_secret_key), CompactError> {
        Ok((
            self.step("secret", &context),
            zk_secret_key {
                bytes: FixedBytes::new([1; 32]),
            },
        ))
    }
    fn private_zk_public_key(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, zk_public_key), CompactError> {
        Ok((self.step("public_key", &context), public_key()))
    }
    fn private_add_coin(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        value: coin_info,
    ) -> Result<(Private, ()), CompactError> {
        assert_eq!(value, coin(context.private_state.next - 2));
        let mut state = self.step("add_coin", &context);
        state.owned += 1;
        Ok((state, ()))
    }
    fn private_remove_coin(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        value: coin_info,
    ) -> Result<(Private, ()), CompactError> {
        assert_eq!(value, coin(3));
        let mut state = self.step("remove_coin", &context);
        state.owned -= 1;
        Ok((state, ()))
    }
    fn context_new_coin_info(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, coin_info), CompactError> {
        let value = coin(context.private_state.next);
        let mut state = self.step("new_coin", &context);
        state.next += 2;
        Ok((state, value))
    }
    fn context_path_of(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        cm: commitment,
    ) -> Result<(Private, MerkleTreePath), CompactError> {
        let state = self.step("path", &context);
        assert_eq!(cm, commitment_of(&coin(3), public_key()));
        let path = if let Some(path) = &self.path {
            path.clone()
        } else {
            let value = if matches!(self.mode, Mode::WrongLeaf) {
                commitment_of(&coin(5), public_key())
            } else {
                cm
            };
            let mut raw = context
                .ledger
                .commitments()?
                .find_path_for_leaf(value)
                .ok_or_else(|| CompactError::AssertionFailed("missing test path".into()))?;
            if matches!(self.mode, Mode::Malformed) {
                raw.path.pop();
            }
            MerkleTreePath::from_ledger_path(raw)?
        };
        let mut path = path;
        if matches!(self.mode, Mode::WrongRoot) {
            let mut entries = path.path.into_array();
            entries[0].sibling.field = entries[0].sibling.field + runtime::Field::from(1_u64);
            path.path = runtime::FixedVector::new(entries);
        }
        Ok((state, path))
    }
    fn context_encrypt(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        key: OpaqueBytes,
        value: coin_info,
    ) -> Result<(Private, OpaqueBytes), CompactError> {
        let state = self.step("encrypt", &context);
        assert_eq!(key, destination().encryption);
        assert_eq!(value, coin(context.private_state.next - 2));
        if matches!(self.mode, Mode::EncryptionFailure) {
            return Err(CompactError::AssertionFailed("encryption rejected".into()));
        }
        Ok((state, ciphertext(&key, &value)))
    }
}
pub fn seeded(historical: bool) -> Result<(CircuitContext<Private>, MerkleTreePath), CompactError> {
    let witness = Witness::default();
    let initial = contract::initial_state(ConstructorContext::new(Private::default()))?;
    let context = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
    let native = contract::zerocash_mint(context, &witness)?;
    let initial = contract::initial_state(ConstructorContext::new(Private::default()))?;
    let recorded = contract::recorded::zerocash_mint(
        initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
        &Witness::default(),
    )?;
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        native.context.private_state,
        recorded.execution.context.private_state
    );
    let mut context = recorded.execution.context;
    let path = MerkleTreePath::from_ledger_path(
        runtime::ledger::historic_merkle_tree_view_at_path(context.query.state.get_ref(), &[1])?
            .find_path_for_leaf(commitment_of(&coin(3), public_key()))
            .unwrap(),
    )?;
    if historical {
        context = contract::recorded::zerocash_mint(context, &Witness::default())?
            .execution
            .context;
    }
    context.private_state.calls = 0;
    Ok((rehydrate(context), path))
}

pub fn contract_state(
    value: runtime::ledger::StateValue<runtime::ledger::DefaultDB>,
) -> runtime::ledger::ContractState<runtime::ledger::DefaultDB> {
    use midnight_onchain_state::state::{
        ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
    };
    let operations = midnight_storage::storage::HashMap::new()
        .insert(
            EntryPointBuf(b"zerocash_mint".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"spend".to_vec()),
            ContractOperation::new(None),
        );
    runtime::ledger::ContractState::new(value, operations, ContractMaintenanceAuthority::default())
}
pub fn rehydrate(context: CircuitContext<Private>) -> CircuitContext<Private> {
    CircuitContext::from_contract_state(
        context.private_state,
        runtime::ledger::ContractAddress::default(),
        &contract_state(context.query.state.get_ref().clone()),
    )
}
