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

//! Small real ledger encodings: exact decoding and encoded-byte admission only.
//! These tests neither generate proofs nor measure upstream heap/CPU bounds.
#![cfg(feature = "ledger-transaction")]

use std::io;

use midnight_compact_runtime::ledger::{
    ContractAddress, ContractState, DefaultDB, StateValue, constructor_cell,
};
use midnight_compact_runtime::transaction::{
    EncodedSizeLimit, Observation, ObservedContractState, VerifierKey, decode_verifier_key,
    decode_verifier_key_with_limit,
};
use midnight_serialize::{Serializable, Tagged, tagged_serialize};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn bytes<T: Serializable + Tagged>(value: &T) -> Vec<u8> {
    let mut encoded = Vec::new();
    tagged_serialize(value, &mut encoded).unwrap();
    encoded
}

fn state() -> ContractState<DefaultDB> {
    ContractState::new(
        StateValue::Array(vec![constructor_cell(true), constructor_cell(17_u64)].into()),
        Default::default(),
        Default::default(),
    )
}

fn observation() -> Observation {
    Observation {
        transaction_hash: [35; 32],
        block_hash: [53; 32],
        block_height: 353,
    }
}

fn key() -> VerifierKey {
    // Real serializable verifier carrier; this is not a proof-validity claim.
    StdRng::seed_from_u64(353).r#gen()
}

#[derive(Clone, Copy, Debug)]
enum Artifact {
    State,
    Key,
}

impl Artifact {
    fn encoded(self) -> Vec<u8> {
        match self {
            Self::State => bytes(&state()),
            Self::Key => bytes(&key()),
        }
    }

    fn decode(self, encoded: &[u8], limit: Option<EncodedSizeLimit>) -> io::Result<Vec<u8>> {
        match (self, limit) {
            (Self::State, None) => {
                ObservedContractState::decode(ContractAddress::default(), encoded, observation())
                    .map(|value| bytes(value.contract()))
            }
            (Self::State, Some(limit)) => ObservedContractState::decode_with_limit(
                ContractAddress::default(),
                encoded,
                observation(),
                limit,
            )
            .map(|value| bytes(value.contract())),
            (Self::Key, None) => decode_verifier_key(encoded).map(|value| bytes(&value)),
            (Self::Key, Some(limit)) => {
                decode_verifier_key_with_limit(encoded, limit).map(|value| bytes(&value))
            }
        }
    }
}

#[test]
fn state_decoder_preserves_exact_state_and_caller_metadata() {
    let expected = state();
    let encoded = bytes(&expected);
    let address = ContractAddress::default();
    for decoded in [
        ObservedContractState::decode(address, &encoded, observation()).unwrap(),
        ObservedContractState::decode_with_limit(
            address,
            &encoded,
            observation(),
            EncodedSizeLimit::new(encoded.len()),
        )
        .unwrap(),
    ] {
        assert_eq!(decoded.contract(), &expected);
        assert_eq!(decoded.address(), address);
        assert_eq!(decoded.observation(), observation());
        assert_eq!(bytes(decoded.contract()), encoded);
    }
}

#[test]
fn seeded_verifier_round_trips_through_legacy_and_limited_decoders() {
    let encoded = bytes(&key());
    assert_eq!(bytes(&decode_verifier_key(&encoded).unwrap()), encoded);
    assert_eq!(
        bytes(
            &decode_verifier_key_with_limit(&encoded, EncodedSizeLimit::new(encoded.len()))
                .unwrap()
        ),
        encoded
    );
}

#[test]
fn default_policy_round_trips_real_state_and_seeded_verifier() {
    let policy = EncodedSizeLimit::default();
    assert_eq!(EncodedSizeLimit::DEFAULT_MAX_BYTES, 67_108_864);
    assert_eq!(policy.max_bytes(), EncodedSizeLimit::DEFAULT_MAX_BYTES);
    for artifact in [Artifact::State, Artifact::Key] {
        let encoded = artifact.encoded();
        assert!(encoded.len() < policy.max_bytes());
        assert_eq!(
            artifact.decode(&encoded, Some(policy)).unwrap(),
            artifact.decode(&encoded, None).unwrap(),
            "{artifact:?}: default policy must preserve exact decoding"
        );
    }
}

#[test]
fn custom_policy_preserves_budgets_below_and_above_the_default() {
    // Exercise policy boundaries without allocating a default-sized input.
    // A generous preset must not clamp explicit application budgets.
    for max_bytes in [
        0,
        EncodedSizeLimit::DEFAULT_MAX_BYTES - 1,
        EncodedSizeLimit::DEFAULT_MAX_BYTES,
        EncodedSizeLimit::DEFAULT_MAX_BYTES + 1,
        usize::MAX,
    ] {
        let policy = EncodedSizeLimit::new(max_bytes);
        assert_eq!(policy.max_bytes(), max_bytes);
        for artifact in [Artifact::State, Artifact::Key] {
            let encoded = artifact.encoded();
            if max_bytes == 0 {
                assert_eq!(
                    artifact.decode(&encoded, Some(policy)).unwrap_err().kind(),
                    io::ErrorKind::InvalidData
                );
            } else {
                assert_eq!(artifact.decode(&encoded, Some(policy)).unwrap(), encoded);
            }
        }
    }
}

#[test]
fn malformed_tag_truncation_trailing_and_empty_preserve_legacy_refusals() {
    for artifact in [Artifact::State, Artifact::Key] {
        let valid = artifact.encoded();
        let mut wrong_tag = valid.clone();
        wrong_tag[0] = b'!';
        let mut trailing = valid.clone();
        trailing.push(0);
        let corpus = [
            ("empty", Vec::new()),
            ("wrong tag", wrong_tag),
            ("truncated header", valid[..3].to_vec()),
            ("truncated body", valid[..valid.len() - 1].to_vec()),
            ("trailing byte", trailing),
        ];
        for (case, encoded) in corpus {
            let legacy = artifact.decode(&encoded, None).expect_err(case);
            let limited = artifact
                .decode(&encoded, Some(EncodedSizeLimit::new(encoded.len())))
                .expect_err(case);
            assert_eq!(limited.kind(), legacy.kind(), "{artifact:?}: {case}");
            assert_eq!(
                limited.to_string(),
                legacy.to_string(),
                "{artifact:?}: {case}"
            );
            assert!(
                !limited.to_string().starts_with("encoded input length"),
                "exact size must reach normal decoding: {artifact:?}: {case}"
            );
        }
    }
}

#[test]
fn exact_and_larger_budgets_admit_while_one_under_and_zero_refuse() {
    for artifact in [Artifact::State, Artifact::Key] {
        let encoded = artifact.encoded();
        for max_bytes in [encoded.len(), encoded.len() + 1] {
            assert_eq!(
                artifact
                    .decode(&encoded, Some(EncodedSizeLimit::new(max_bytes)))
                    .unwrap(),
                encoded,
                "{artifact:?}: budget {max_bytes}"
            );
        }
        for max_bytes in [encoded.len() - 1, 0] {
            let error = artifact
                .decode(&encoded, Some(EncodedSizeLimit::new(max_bytes)))
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert_eq!(
                error.to_string(),
                format!(
                    "encoded input length {} exceeds byte limit {max_bytes}",
                    encoded.len()
                )
            );
        }
        // A zero-byte policy admits the empty input to the normal invalid-tag
        // refusal; it does not make an empty encoding a valid state or key.
        let zero = artifact
            .decode(&[], Some(EncodedSizeLimit::new(0)))
            .unwrap_err();
        assert_eq!(zero.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            zero.to_string(),
            artifact.decode(&[], None).unwrap_err().to_string()
        );
    }
}

#[test]
fn oversized_invalid_bytes_are_refused_before_parsing_without_payload_disclosure() {
    let payload = b"SECRET_DECODE_PAYLOAD_353";
    let max_bytes = payload.len() - 1;
    for artifact in [Artifact::State, Artifact::Key] {
        let limited = artifact
            .decode(payload, Some(EncodedSizeLimit::new(max_bytes)))
            .unwrap_err();
        let expected = format!(
            "encoded input length {} exceeds byte limit {max_bytes}",
            payload.len()
        );
        assert_eq!(limited.kind(), io::ErrorKind::InvalidData);
        assert_eq!(limited.to_string(), expected);
        assert!(!format!("{limited:?}").contains("SECRET_DECODE_PAYLOAD_353"));
        assert_ne!(
            limited.to_string(),
            artifact.decode(payload, None).unwrap_err().to_string()
        );
    }
}
