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

use compact_rust_backend::{ir::Contract, render_with_capabilities};

#[test]
fn original_did_digest_gets_recorded_and_observed_api() {
    let source = include_str!("did-digest-schema20-ir.json");
    let contract: Contract = serde_json::from_str(source).unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    let circuit = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
        .unwrap();
    assert!(circuit.recorded, "{circuit:?}");
    assert!(circuit.observed_call, "{circuit:?}");
    assert!(
        rendered
            .source
            .contains("pub fn verifySchnorrJubjubDigestSignature<")
    );
    assert!(
        rendered
            .source
            .contains("record_lookup(frame, __compact_recorded_map_key)")
    );
}
