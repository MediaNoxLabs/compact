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

//! Prove and verify the emitted counter increment circuit using ledger-8.
//!
//! The transcript is the fixture's deterministic ZKIR statement. This checks
//! artifact compatibility, while the generated Rust runtime's transcript-to-
//! transaction bridge remains a separate integration gate.

use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_serialize::tagged_deserialize;
use midnight_transient_crypto::curve::Fr;
use midnight_transient_crypto::hash::transient_commit;
use midnight_transient_crypto::proofs::{
    KeyLocation, PARAMS_VERIFIER, ProofPreimage, ProvingKeyMaterial, Resolver, VerifierKey,
};
use midnight_zkir::IrSource;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

struct ArtifactResolver {
    root: PathBuf,
}

impl Resolver for ArtifactResolver {
    async fn resolve_key(&self, location: KeyLocation) -> io::Result<Option<ProvingKeyMaterial>> {
        if location.0.as_ref() != "increment" {
            return Ok(None);
        }
        Ok(Some(ProvingKeyMaterial {
            prover_key: fs::read(self.root.join("keys/increment.prover"))?,
            verifier_key: fs::read(self.root.join("keys/increment.verifier"))?,
            ir_source: fs::read(self.root.join("zkir/increment.bzkir"))?,
        }))
    }
}

fn prove_counter(root: &Path) -> Result<(), Box<dyn Error>> {
    let ir: IrSource = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("zkir/increment.bzkir"),
    )?))?;
    let statement = [0x70u64, 1, 1, 0, 0x0e, 1, 0xa1]
        .into_iter()
        .map(Fr::from)
        .collect::<Vec<_>>();
    let opening = Fr::from(0u64);
    let preimage = ProofPreimage {
        inputs: vec![],
        private_transcript: vec![],
        public_transcript_inputs: statement,
        public_transcript_outputs: vec![],
        binding_input: Fr::from(0u64),
        communications_commitment: Some((transient_commit(&[] as &[Fr], opening), opening)),
        key_location: KeyLocation(Cow::Borrowed("increment")),
    };
    let skips = preimage.check(&ir)?;
    if skips != [None, None, None] {
        return Err(format!("unexpected counter public input skips: {skips:?}").into());
    }

    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let (proof, proof_skips) = futures_executor::block_on(preimage.prove::<IrSource>(
        ChaCha20Rng::from_seed([42; 32]),
        &params,
        &ArtifactResolver {
            root: root.to_owned(),
        },
    ))?;
    if proof_skips != skips {
        return Err("proof and preimage checks disagree about skipped public inputs".into());
    }

    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/increment.verifier"),
    )?))?;
    let mut public_inputs = vec![
        preimage.binding_input,
        preimage
            .communications_commitment
            .expect("counter fixture supplies a communications commitment")
            .0,
    ];
    public_inputs.extend(preimage.public_transcript_inputs.iter().copied());
    verifier.verify(&PARAMS_VERIFIER, &proof, public_inputs.iter().copied())?;
    public_inputs[0] = Fr::from(1u64);
    if verifier
        .verify(&PARAMS_VERIFIER, &proof, public_inputs.iter().copied())
        .is_ok()
    {
        return Err("counter proof accepted a different binding input".into());
    }
    println!(
        "counter proof verified ({} bytes); changed binding rejected",
        proof.0.len()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args_os()
        .nth(1)
        .ok_or("usage: compact-rust-proof-smoke <compiler-output-directory>")?;
    prove_counter(Path::new(&root))
}
