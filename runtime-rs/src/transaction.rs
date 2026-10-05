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

//! Adapt a complete recorded circuit to ledger-8 call construction.
//!
//! This module is opt-in because native execution and pure contracts do not
//! need the ledger transaction dependency. A generated circuit only has a
//! usable recorded method after the backend can capture every VM operation in
//! that circuit.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{self, Cursor};

use midnight_base_crypto::fab::AlignedValue;
use midnight_base_crypto::signatures::Signature;
use midnight_base_crypto::time::Timestamp;
use midnight_coin_structure::coin::Commitment;
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
use midnight_ledger::structure::{
    INITIAL_PARAMETERS, Intent, LedgerState, ProofPreimageMarker, Transaction,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::tagged_deserialize;
use midnight_storage::storage::{HashMap, Map};
use midnight_transient_crypto::commitment::PedersenRandomness;
use midnight_transient_crypto::curve::Fr;
pub use midnight_transient_crypto::proofs::VerifierKey;
use midnight_transient_crypto::proofs::{KeyLocation, ProofPreimage};
use midnight_zswap::Offer;
use rand::{CryptoRng, Rng};

use crate::context::CircuitContext;
use crate::ledger::{ContractAddress, ContractState, DB, DefaultDB};
use crate::recording::RecordedCircuitResult;

/// Where a caller observed public contract state. These values do not prove
/// finality or authenticate the response from an indexer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub transaction_hash: [u8; 32],
    pub block_hash: [u8; 32],
    pub block_height: u64,
}

/// An upstream ledger contract state paired with its explicit address and
/// caller-supplied observation metadata.
pub struct ObservedContractState<D: DB = DefaultDB> {
    address: ContractAddress,
    contract: ContractState<D>,
    observation: Observation,
    com_indices: Map<Commitment, u64>,
}

impl<D: DB> ObservedContractState<D> {
    pub fn new(
        address: ContractAddress,
        contract: ContractState<D>,
        observation: Observation,
    ) -> Self {
        Self {
            address,
            contract,
            observation,
            com_indices: Map::new(),
        }
    }

    pub fn address(&self) -> ContractAddress {
        self.address
    }

    pub fn contract(&self) -> &ContractState<D> {
        &self.contract
    }

    pub fn observation(&self) -> Observation {
        self.observation
    }

    pub fn circuit_context<Private>(&self, private_state: Private) -> CircuitContext<Private, D> {
        let mut context =
            CircuitContext::from_contract_state(private_state, self.address, &self.contract);
        context.query.call_context.com_indices = self.com_indices.clone();
        context
    }
}

/// An observed contract paired with an upstream offer that derives the exact
/// commitment indices used by its generated call and final transaction.
pub struct OfferBackedObservedState<D: DB = DefaultDB> {
    observed: ObservedContractState<D>,
    offer: Offer<ProofPreimage, D>,
}

impl<D: DB> OfferBackedObservedState<D> {
    pub fn new(
        mut observed: ObservedContractState<D>,
        ledger: &LedgerState<D>,
        offer: Offer<ProofPreimage, D>,
    ) -> Result<Self, crate::CompactError> {
        let Some(ledger_contract) = ledger.contract.get(&observed.address) else {
            return Err(crate::CompactError::InvalidLedgerCell(
                "offer observation contract missing from ledger".into(),
            ));
        };
        if ledger_contract != &observed.contract {
            return Err(crate::CompactError::InvalidLedgerCell(
                "offer observation differs from ledger contract".into(),
            ));
        }
        let (_, indices) = ledger.zswap.try_apply(&offer, None).map_err(|error| {
            crate::CompactError::InvalidLedgerCell(format!("offer rejected: {error:?}"))
        })?;
        observed.com_indices = indices;
        Ok(Self { observed, offer })
    }

    pub fn observed(&self) -> &ObservedContractState<D> {
        &self.observed
    }

    /// Prepare only a call recorded against this exact offer-backed observation.
    pub fn prepare<Private, Output: Into<AlignedValue>>(
        &self,
        call: RecordedCall<'_, Private, Output, D>,
        verifier: VerifierKey,
        communication_commitment_rand: Fr,
    ) -> Result<OfferBoundPreparedCall<D>, ObservedCallError> {
        if !std::ptr::eq(call.observed, &self.observed)
            || call.recorded.public.initial().call_context.com_indices != self.observed.com_indices
        {
            return Err(ObservedCallError::OfferMismatch);
        }
        let prepared = call.prepare(verifier, communication_commitment_rand)?;
        Ok(OfferBoundPreparedCall {
            call: prepared,
            offer: self.offer.clone(),
        })
    }
}

/// A prepared call and the same validated offer from its observed context.
pub struct OfferBoundPreparedCall<D: DB = DefaultDB> {
    call: ContractCallPrototype<D>,
    offer: Offer<ProofPreimage, D>,
}

impl<D: DB> OfferBoundPreparedCall<D> {
    pub fn into_transaction<R: Rng + CryptoRng + ?Sized>(
        self,
        rng: &mut R,
        network_id: impl Into<String>,
        ttl: Timestamp,
    ) -> Transaction<Signature, ProofPreimageMarker, PedersenRandomness, D> {
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, D> =
            Intent::empty(rng, ttl).add_call::<ProofPreimage>(self.call);
        Transaction::new(
            network_id,
            HashMap::new().insert(1_u16, intent),
            Some(self.offer),
            HashMap::new(),
        )
    }
}

impl ObservedContractState<DefaultDB> {
    /// Decode exact ledger-8 tagged `ContractState` bytes from an indexer.
    pub fn decode(
        address: ContractAddress,
        bytes: &[u8],
        observation: Observation,
    ) -> io::Result<Self> {
        let mut cursor = Cursor::new(bytes);
        let contract = tagged_deserialize(&mut cursor)?;
        if cursor.position() != bytes.len() as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "trailing contract-state bytes",
            ));
        }
        Ok(Self::new(address, contract, observation))
    }
}

/// Decode one compiler-emitted ledger-8 `.verifier` artifact exactly. This
/// keeps a generated-crate consumer from needing a direct serializer crate.
pub fn decode_verifier_key(bytes: &[u8]) -> io::Result<VerifierKey> {
    let mut cursor = Cursor::new(bytes);
    let verifier = tagged_deserialize(&mut cursor)?;
    if cursor.position() != bytes.len() as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "trailing verifier-key bytes",
        ));
    }
    Ok(verifier)
}

/// A generated circuit's recorded trace with its source-owned entry point and
/// input, tied to the state from which the trace was recorded.
pub struct RecordedCall<'a, Private, Output, D: DB = DefaultDB> {
    observed: &'a ObservedContractState<D>,
    recorded: RecordedCircuitResult<Private, Output, D>,
    entry_point: &'static str,
    input: AlignedValue,
}

impl<'a, Private, Output, D: DB> RecordedCall<'a, Private, Output, D> {
    pub fn new<Input: Into<AlignedValue>>(
        observed: &'a ObservedContractState<D>,
        recorded: RecordedCircuitResult<Private, Output, D>,
        entry_point: &'static str,
        input: Input,
    ) -> Self {
        Self {
            observed,
            recorded,
            entry_point,
            input: input.into(),
        }
    }

    pub fn recorded(&self) -> &RecordedCircuitResult<Private, Output, D> {
        &self.recorded
    }
}

#[derive(Debug)]
pub enum ObservedCallError {
    AddressMismatch,
    StateMismatch,
    OfferMismatch,
    MissingOperation(String),
    VerifierMismatch(String),
    Prepare(PrepareCallError),
}

impl fmt::Display for ObservedCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressMismatch => {
                formatter.write_str("recorded call address differs from observation")
            }
            Self::StateMismatch => {
                formatter.write_str("recorded call initial state differs from observation")
            }
            Self::OfferMismatch => {
                formatter.write_str("recorded call is not bound to this observed offer")
            }
            Self::MissingOperation(name) => {
                write!(formatter, "observed contract has no {name} operation")
            }
            Self::VerifierMismatch(name) => write!(
                formatter,
                "observed {name} verifier differs from call artifact"
            ),
            Self::Prepare(error) => error.fmt(formatter),
        }
    }
}

impl Error for ObservedCallError {}

impl<'a, Private, Output: Into<AlignedValue>, D: DB> RecordedCall<'a, Private, Output, D> {
    /// Check the observed state and installed verifier, then replay and
    /// partition the trace through the ordinary ledger-8 call adapter.
    pub fn prepare(
        self,
        verifier: VerifierKey,
        communication_commitment_rand: Fr,
    ) -> Result<ContractCallPrototype<D>, ObservedCallError> {
        let initial = self.recorded.public.initial();
        if initial.address != self.observed.address {
            return Err(ObservedCallError::AddressMismatch);
        }
        if initial.state.get_ref() != self.observed.contract.data.get_ref() {
            return Err(ObservedCallError::StateMismatch);
        }
        let operation = self
            .observed
            .contract
            .operations
            .get(&EntryPointBuf(self.entry_point.as_bytes().to_vec()))
            .ok_or_else(|| ObservedCallError::MissingOperation(self.entry_point.to_owned()))?;
        if operation.latest() != Some(&verifier) {
            return Err(ObservedCallError::VerifierMismatch(
                self.entry_point.to_owned(),
            ));
        }
        prepare_call(
            self.recorded,
            CallSpec::new(
                self.entry_point,
                verifier,
                self.input,
                communication_commitment_rand,
            ),
        )
        .map_err(ObservedCallError::Prepare)
    }
}

/// The entry point artifacts and public input for one recorded call.
pub struct CallSpec {
    pub entry_point: String,
    pub key_location: KeyLocation,
    pub verifier_key: VerifierKey,
    pub input: AlignedValue,
    pub communication_commitment_rand: Fr,
}

impl CallSpec {
    pub fn new<Input: Into<AlignedValue>>(
        entry_point: impl Into<String>,
        verifier_key: VerifierKey,
        input: Input,
        communication_commitment_rand: Fr,
    ) -> Self {
        let entry_point = entry_point.into();
        Self {
            key_location: KeyLocation(Cow::Owned(entry_point.clone())),
            entry_point,
            verifier_key,
            input: input.into(),
            communication_commitment_rand,
        }
    }

    /// Select a prover artifact whose name differs from the entry point.
    pub fn with_key_location(mut self, location: impl Into<Cow<'static, str>>) -> Self {
        self.key_location = KeyLocation(location.into());
        self
    }
}

#[derive(Debug)]
pub enum PrepareCallError {
    Replay(String),
    ReplayEffectsMismatch,
    ReplayStateMismatch,
    Partition(String),
    EmptyTranscript,
    PartitionEffectsMismatch,
}

impl fmt::Display for PrepareCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Replay(reason) => write!(formatter, "recorded call replay failed: {reason}"),
            Self::ReplayEffectsMismatch => {
                formatter.write_str("recorded call replay effects differ from execution")
            }
            Self::ReplayStateMismatch => {
                formatter.write_str("recorded call replay state differs from execution")
            }
            Self::Partition(reason) => {
                write!(formatter, "recorded call partition failed: {reason}")
            }
            Self::EmptyTranscript => formatter.write_str("recorded call has no ledger transcript"),
            Self::PartitionEffectsMismatch => {
                formatter.write_str("partitioned call effects differ from replay")
            }
        }
    }
}

impl Error for PrepareCallError {}

/// Replay and partition the recorded VM program, then build a ledger call.
/// The public output comes from the typed circuit result; the caller supplies
/// the verifier artifact and the public input encoding for the circuit.
pub fn prepare_call<Private, Output, D: DB>(
    recorded: RecordedCircuitResult<Private, Output, D>,
    spec: CallSpec,
) -> Result<ContractCallPrototype<D>, PrepareCallError>
where
    Output: Into<AlignedValue>,
{
    if recorded.public.verify_ops().is_empty() {
        return Err(PrepareCallError::EmptyTranscript);
    }
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .map_err(|error| PrepareCallError::Replay(format!("{error:?}")))?;
    if replay.context.effects != recorded.execution.context.query.effects {
        return Err(PrepareCallError::ReplayEffectsMismatch);
    }
    if replay.context.state.get_ref() != recorded.execution.context.query.state.get_ref() {
        return Err(PrepareCallError::ReplayStateMismatch);
    }
    let (context, program) = recorded.public.into_parts();
    let address = context.address;
    let partitioned = partition_transcripts(
        &[PreTranscript {
            context,
            program,
            comm_comm: None,
        }],
        &INITIAL_PARAMETERS,
    )
    .map_err(|error| PrepareCallError::Partition(format!("{error:?}")))?;
    let (guaranteed, fallible) = partitioned
        .into_iter()
        .next()
        .ok_or(PrepareCallError::EmptyTranscript)?;
    let transcript = guaranteed
        .as_ref()
        .or(fallible.as_ref())
        .ok_or(PrepareCallError::EmptyTranscript)?;
    if transcript.effects != replay.context.effects {
        return Err(PrepareCallError::PartitionEffectsMismatch);
    }
    Ok(ContractCallPrototype {
        address,
        entry_point: EntryPointBuf(spec.entry_point.as_bytes().to_vec()),
        op: ContractOperation::new(Some(spec.verifier_key)),
        guaranteed_public_transcript: guaranteed,
        fallible_public_transcript: fallible,
        private_transcript_outputs: recorded.execution.private_transcript_outputs,
        input: spec.input,
        output: recorded.execution.result.into(),
        communication_commitment_rand: spec.communication_commitment_rand,
        key_location: spec.key_location,
    })
}
