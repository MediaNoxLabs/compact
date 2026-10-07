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

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};
#[test]
fn reviewed_source_capture_and_assertions_have_exact_identities() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(
        capture["format"],
        "compact-did-primitive-reducer-capture/v1"
    );
    let rows = capture["cases"].as_array().unwrap();
    let ids: BTreeSet<_> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        BTreeSet::from([
            "insert-unicode",
            "update-empty",
            "remove",
            "duplicate",
            "update-missing",
            "remove-missing",
            "kind-before-query"
        ])
    );
    assert_eq!(rows.len(), ids.len());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let provenance = capture["provenance"].as_array().unwrap();
    assert_eq!(provenance.len(), 9);
    assert_eq!(
        provenance
            .iter()
            .map(|r| r["path"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "tools/compact-rust-backend/tests/map-nested-product/nested-enum.compact",
            "tests-rust-backend/did-nested-map-reducer/lib.rs",
            "tests-rust-backend/did-nested-map-reducer/tests/behavior.rs",
            "tests-rust-backend/did-nested-map-reducer/tests/provenance.rs",
            "tools/compact-rust-backend/did_primitive_reducers.json",
            "tools/compact-rust-backend/did_primitive_reducer_capture.mjs",
            "tools/compact-rust-backend/did_primitive_reducer_cases.mjs",
            "runtime/src/built-ins.ts",
            "runtime/src/compact-types.ts"
        ])
    );
    for row in provenance {
        let path = row["path"].as_str().unwrap();
        assert!(!Path::new(path).is_absolute());
        assert!(!path.split('/').any(|part| part == ".."));
        assert_eq!(
            hex::encode(Sha256::digest(fs::read(root.join(path)).unwrap())),
            row["sha256"],
            "stale {path}"
        );
    }
}
