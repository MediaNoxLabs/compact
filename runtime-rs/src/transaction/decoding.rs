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

//! Exact ledger wire decoding and caller-selected encoded-byte admission.

use std::io::{self, Cursor};

use midnight_serialize::tagged_deserialize;

use super::{Observation, ObservedContractState, VerifierKey};
use crate::ledger::{ContractAddress, DefaultDB};

/// An integration-selected maximum for one already acquired encoded input.
///
/// This policy bounds the input bytes admitted to the upstream decoder, not its
/// total decoded heap, object count or CPU work. The pinned ledger serializer's
/// own recursion and allocation behavior still applies. Bound acquisition before
/// constructing the byte slice as well when reading from an untrusted transport.
/// [`Self::default`] opts into a 64 MiB application policy. It is not a ledger
/// protocol limit or the maximum supported artifact size. Use [`Self::new`] for
/// a different integration budget; legacy decoders do not apply this default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodedSizeLimit {
    max_bytes: usize,
}

impl EncodedSizeLimit {
    /// The opt-in default admits at most 64 MiB (67,108,864 encoded bytes).
    ///
    /// This is an application policy, not a protocol or global decoder cap,
    /// maximum supported ledger artifact size, or decoded heap/CPU bound.
    pub const DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;

    /// Select the largest encoded input to admit. Zero admits only an empty
    /// slice, which the ledger tagged decoder rejects as an invalid artifact.
    pub const fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    /// Return the selected encoded-byte budget, including any explicit override.
    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }

    fn admit(self, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() > self.max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "encoded input length {} exceeds byte limit {}",
                    bytes.len(),
                    self.max_bytes
                ),
            ));
        }
        Ok(())
    }
}

impl Default for EncodedSizeLimit {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_BYTES)
    }
}

impl ObservedContractState<DefaultDB> {
    /// Decode exact ledger-8 tagged `ContractState` bytes from an indexer.
    ///
    /// This compatibility entrypoint adds no encoded-byte limit beyond the
    /// upstream decoder's inherited behavior. Prefer [`Self::decode_with_limit`]
    /// at a boundary that needs an explicit caller-selected input-size policy.
    /// Neither entrypoint authenticates the supplied address or observation.
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

    /// Admit the encoded input size before exact ledger state decoding.
    ///
    /// An oversized input returns [`io::ErrorKind::InvalidData`] without invoking
    /// the upstream decoder. The limit does not bound total decoded heap or CPU
    /// work. There is no retry through the compatibility entrypoint on refusal.
    pub fn decode_with_limit(
        address: ContractAddress,
        bytes: &[u8],
        observation: Observation,
        limit: EncodedSizeLimit,
    ) -> io::Result<Self> {
        limit.admit(bytes)?;
        Self::decode(address, bytes, observation)
    }
}

/// Decode one compiler-emitted ledger-8 `.verifier` artifact exactly. This
/// keeps a generated-crate consumer from needing a direct serializer crate.
///
/// This compatibility entrypoint adds no encoded-byte policy. Prefer
/// [`decode_verifier_key_with_limit`] when admitting externally supplied bytes.
/// Decoding a key does not authenticate it or verify a proof.
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

/// Admit the encoded key size before exact ledger verifier decoding.
///
/// Oversized inputs return [`io::ErrorKind::InvalidData`] before upstream parsing.
/// The caller-selected limit owns encoded bytes only, not total decoded heap,
/// object count or CPU work. No limited refusal falls back to unlimited decoding.
pub fn decode_verifier_key_with_limit(
    bytes: &[u8],
    limit: EncodedSizeLimit,
) -> io::Result<VerifierKey> {
    limit.admit(bytes)?;
    decode_verifier_key(bytes)
}
