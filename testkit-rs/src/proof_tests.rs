// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Local nonproduction probes. No parameters, proof generation or network access.

use super::*;
use std::{cell::Cell, error::Error as _};

use futures_executor::block_on;
use midnight_serialize::{Serializable, Tagged, tagged_serialize};
use midnight_transient_crypto::{
    curve::Fr,
    proofs::{
        ParamsProver, ParamsProverProvider, ProofPreimage, ProvingKeyMaterial, ProvingProvider,
    },
};
use midnight_zkir::{Instruction, IrSource};
use rand::{Rng, SeedableRng, rngs::StdRng};

struct MemoryResolver {
    material: Option<ProvingKeyMaterial>,
    fail: bool,
    calls: Cell<usize>,
}

impl Resolver for MemoryResolver {
    async fn resolve_key(&self, key: KeyLocation) -> io::Result<Option<ProvingKeyMaterial>> {
        assert_eq!(key, location());
        self.calls.set(self.calls.get() + 1);
        if self.fail {
            Err(io::Error::other("private-provider-diagnostic"))
        } else {
            Ok(self.material.clone())
        }
    }
}

struct NoParameters(Cell<usize>);

impl ParamsProverProvider for NoParameters {
    async fn get_params(&self, _k: u8) -> io::Result<ParamsProver> {
        self.0.set(self.0.get() + 1);
        Err(io::Error::other("parameters disabled in local model test"))
    }
}

fn location() -> KeyLocation {
    KeyLocation("local-model-assert".into())
}

fn encoded(value: &(impl Serializable + Tagged)) -> Vec<u8> {
    let mut bytes = Vec::new();
    tagged_serialize(value, &mut bytes).unwrap();
    bytes
}

fn resolver(material: Option<ProvingKeyMaterial>) -> MemoryResolver {
    MemoryResolver {
        material,
        fail: false,
        calls: Cell::new(0),
    }
}

fn material(verifier_key: Vec<u8>, ir_source: Vec<u8>) -> ProvingKeyMaterial {
    ProvingKeyMaterial {
        prover_key: Vec::new(),
        verifier_key,
        ir_source,
    }
}

fn lab(source: MemoryResolver) -> ProofLab<MemoryResolver, NoParameters> {
    ProofLab::new(source, NoParameters(Cell::new(0)))
}

fn preimage(input: u64) -> ProofPreimage {
    ProofPreimage {
        inputs: vec![Fr::from(input)],
        private_transcript: Vec::new(),
        public_transcript_inputs: Vec::new(),
        public_transcript_outputs: Vec::new(),
        binding_input: Fr::from(0),
        communications_commitment: None,
        key_location: location(),
    }
}

#[test]
fn local_resolution_preserves_unavailable_io_and_decode_categories_without_retry() {
    let missing = lab(resolver(None));
    assert!(matches!(
        block_on(missing.verifier(location(), EncodedSizeLimit::default())),
        Err(ProofError::Unavailable)
    ));
    assert_eq!(missing.resolver.calls.get(), 1);

    let failed = lab(MemoryResolver {
        fail: true,
        ..resolver(None)
    });
    let error = block_on(failed.verifier(location(), EncodedSizeLimit::default())).unwrap_err();
    assert!(matches!(error, ProofError::Resolve(_)));
    assert!(!format!("{error} {error:?}").contains("private-provider-diagnostic"));
    assert_eq!(
        error.source().unwrap().to_string(),
        "private-provider-diagnostic"
    );
    assert_eq!(failed.resolver.calls.get(), 1);

    let malformed = lab(resolver(Some(material(vec![0xff], Vec::new()))));
    assert!(matches!(
        block_on(malformed.verifier(location(), EncodedSizeLimit::default())),
        Err(ProofError::Decode(_))
    ));
    assert_eq!(malformed.resolver.calls.get(), 1);
}

#[test]
fn local_resolution_admits_exact_bytes_and_rejects_one_under() {
    // A real serializable carrier, not a claim of a circuit-valid verifying key.
    let key: VerifierKey = StdRng::seed_from_u64(360).r#gen();
    let bytes = encoded(&key);
    let fixture = lab(resolver(Some(material(bytes.clone(), Vec::new()))));
    let got = block_on(fixture.verifier(location(), EncodedSizeLimit::new(bytes.len()))).unwrap();
    assert_eq!(encoded(&got), bytes);
    let error =
        block_on(fixture.verifier(location(), EncodedSizeLimit::new(bytes.len() - 1))).unwrap_err();
    assert!(
        matches!(error, ProofError::Decode(ref inner) if inner.kind() == io::ErrorKind::InvalidData)
    );
    assert_eq!(fixture.resolver.calls.get(), 2);
}

#[test]
fn local_official_provider_checks_real_assertion_without_proving_or_parameters() {
    let ir = IrSource {
        num_inputs: 1,
        do_communications_commitment: false,
        instructions: vec![Instruction::Assert { cond: 0 }].into(),
    };
    // Check consumes IR only. Empty keys deliberately cannot produce a proof.
    let fixture = lab(resolver(Some(material(Vec::new(), encoded(&ir)))));
    let provider = fixture.provider(StdRng::seed_from_u64(360));
    assert!(block_on(provider.check(&preimage(1))).unwrap().is_empty());
    assert!(block_on(provider.check(&preimage(0))).is_err());
    assert_eq!(fixture.params.0.get(), 0);
    // Official prove consumes the provider; invalid key material must fail.
    assert!(block_on(provider.prove(&preimage(1), None)).is_err());
    assert_eq!(fixture.resolver.calls.get(), 3);
    assert_eq!(fixture.params.0.get(), 0);
    assert!(block_on(fixture.params.get_params(1)).is_err());
    assert_eq!(fixture.params.0.get(), 1);
}

#[test]
fn local_official_provider_refuses_missing_and_malformed_ir_before_parameters() {
    for data in [None, Some(material(Vec::new(), vec![0xff]))] {
        let fixture = lab(resolver(data));
        let provider = fixture.provider(StdRng::seed_from_u64(360));
        assert!(block_on(provider.check(&preimage(1))).is_err());
        assert!(block_on(provider.prove(&preimage(1), Some(Fr::from(7)))).is_err());
        assert_eq!(fixture.resolver.calls.get(), 2);
        assert_eq!(fixture.params.0.get(), 0);
    }
}
