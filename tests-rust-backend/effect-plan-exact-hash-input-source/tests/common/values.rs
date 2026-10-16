// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_exact_hash_input_source_fixture::{
    runtime as r,
    types::{Envelope, Hashes, Payload},
};
fn field(text: &str) -> r::Field {
    text.bytes().fold(r::Field::from(0_u64), |v, d| {
        assert!(d.is_ascii_digit());
        v * r::Field::from(10_u64) + r::Field::from(u64::from(d - b'0'))
    })
}
pub fn expected(row: &serde_json::Value) -> Hashes {
    Hashes {
        x: field(row["result"]["x"].as_str().unwrap()),
        y: field(row["result"]["y"].as_str().unwrap()),
        transient: field(row["result"]["transient"].as_str().unwrap()),
        committed: field(row["result"]["committed"].as_str().unwrap()),
    }
}
pub fn empty() -> Hashes {
    Hashes {
        x: r::Field::from(0_u64),
        y: r::Field::from(0_u64),
        transient: r::Field::from(0_u64),
        committed: r::Field::from(0_u64),
    }
}
pub fn envelope(text: &str, bytes: &[u8], nonce: u64, samples: [&[u8]; 2]) -> Envelope {
    Envelope {
        payload: Payload {
            text: text.into(),
            bytes: r::OpaqueBytes(bytes.to_vec()),
        },
        nonce: r::Field::from(nonce),
        samples: r::FixedVector(samples.map(|v| r::OpaqueBytes(v.to_vec()))),
    }
}
