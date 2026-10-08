// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Small proof-test fixture using the official ledger provider contracts.
//!
//! Resolution and decoding do not authenticate a key or verify a proof. The byte
//! policy applies after resolution; the resolver owns acquisition limits. Actual
//! checking/proving uses the upstream `ProvingProvider` directly, without another
//! provider trait or a wrapper that changes its ownership/error semantics.

use std::{fmt, io};

use midnight_base_crypto::rng::SplittableRng;
use midnight_compact_runtime::transaction::{
    EncodedSizeLimit, VerifierKey, decode_verifier_key_with_limit,
};
use midnight_transient_crypto::proofs::{KeyLocation, ParamsProverProvider, Resolver};
use midnight_zkir::LocalProvingProvider;
use rand::{CryptoRng, Rng};

/// Reusable resolver/parameter fixture for tests alongside `ContractLab`.
///
/// The returned provider is the official local runner. Use its upstream
/// `ProvingProvider::check` and `prove` methods directly; successful checking
/// alone is not proof generation or ledger acceptance.
///
/// Enable the testkit's `proof` feature. The following example is compile-checked;
/// executing it requires matching circuit artifacts and prover parameters. Seeded
/// randomness is for repeatable local tests, not production proving.
///
/// ```no_run
/// use midnight_compact_testkit::ProofLab;
/// use midnight_transient_crypto::proofs::{
///     ParamsProverProvider, ProofPreimage, ProvingProvider, Resolver,
/// };
/// use rand::{SeedableRng, rngs::StdRng};
///
/// # async fn example<S: Resolver, P: ParamsProverProvider>(
/// #     resolver: S, params: P, preimage: ProofPreimage,
/// # ) -> Result<(), Box<dyn std::error::Error>> {
/// let lab = ProofLab::new(resolver, params);
/// let provider = lab.provider(StdRng::seed_from_u64(42));
/// let skip_sequence = provider.check(&preimage).await?;
/// // Explicit, separate operation: proving consumes the official provider.
/// let proof = provider.prove(&preimage, None).await?;
/// # let _ = (skip_sequence, proof);
/// # Ok(())
/// # }
/// ```
pub struct ProofLab<S, P> {
    resolver: S,
    params: P,
}

impl<S: Resolver, P: ParamsProverProvider> ProofLab<S, P> {
    /// Own the test's artifact resolver and parameter provider.
    pub fn new(resolver: S, params: P) -> Self {
        Self { resolver, params }
    }

    /// Create an official prover borrowing this fixture and owning the supplied RNG.
    pub fn provider<R: Rng + CryptoRng + SplittableRng>(
        &self,
        rng: R,
    ) -> LocalProvingProvider<'_, R, S, P> {
        LocalProvingProvider {
            rng,
            resolver: &self.resolver,
            params: &self.params,
        }
    }

    /// Resolve a verifier with explicit encoded-byte admission, without retry.
    pub async fn verifier(
        &self,
        key: KeyLocation,
        limit: EncodedSizeLimit,
    ) -> Result<VerifierKey, ProofError> {
        let material = self
            .resolver
            .resolve_key(key)
            .await
            .map_err(ProofError::Resolve)?
            .ok_or(ProofError::Unavailable)?;
        decode_verifier_key_with_limit(&material.verifier_key, limit).map_err(ProofError::Decode)
    }
}

/// Experimental resolution failures, kept distinct from cryptographic refusal.
pub enum ProofError {
    Unavailable,
    Resolve(io::Error),
    Decode(io::Error),
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "verifier material unavailable",
            Self::Resolve(_) => "verifier resolution failed",
            Self::Decode(_) => "verifier decoding failed",
        })
    }
}

impl fmt::Debug for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for ProofError {
    // Explicit source access retains the underlying diagnostic; only this
    // adapter's default Display/Debug avoids interpolating provider payloads.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unavailable => None,
            Self::Resolve(error) | Self::Decode(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "proof_tests.rs"]
mod tests;
