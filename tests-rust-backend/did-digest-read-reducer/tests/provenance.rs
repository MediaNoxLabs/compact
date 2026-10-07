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
        "compact-did-digest-read-reducer-capture/v1"
    );
    let rows = capture["cases"].as_array().unwrap();
    let ids: BTreeSet<_> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        BTreeSet::from(["matching", "missing", "witness-denied", "wrong-point"])
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let provenance = capture["provenance"].as_array().unwrap();
    assert_eq!(provenance.len(), 7);
    for row in provenance {
        let path = row["path"].as_str().unwrap();
        assert!(!Path::new(path).is_absolute());
        assert!(!path.split('/').any(|part| part == ".."));
        let bytes = fs::read(root.join(path)).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(bytes)),
            row["sha256"],
            "stale {path}"
        );
    }
}
