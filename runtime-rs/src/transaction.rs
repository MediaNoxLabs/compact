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

use midnight_base_crypto::fab::AlignedValue;
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
use midnight_ledger::structure::INITIAL_PARAMETERS;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_transient_crypto::curve::Fr;
use midnight_transient_crypto::proofs::{KeyLocation, VerifierKey};

use crate::ledger::DB;
use crate::recording::RecordedCircuitResult;

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
