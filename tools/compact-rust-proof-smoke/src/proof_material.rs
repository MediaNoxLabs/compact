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

//! Explicit acquisition of the pinned upstream built-in keys and their SRS.

use std::{collections::BTreeSet, error::Error};

use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_ledger::dust::DUST_EXPECTED_FILES;
use midnight_serialize::tagged_deserialize;
use midnight_zkir::IrSource;
use midnight_zswap::ZSWAP_EXPECTED_FILES;
use serde_json::json;

pub(super) fn run(verify_only: bool) -> Result<(), Box<dyn Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(prepare(verify_only))
}

async fn prepare(verify_only: bool) -> Result<(), Box<dyn Error>> {
    let files = ZSWAP_EXPECTED_FILES
        .iter()
        .chain(DUST_EXPECTED_FILES.iter())
        .copied()
        .collect::<Vec<_>>();
    // Keep reads synchronous even during preparation: only explicit fetch calls
    // below may use the network. Verify-only therefore cannot repair a cache.
    let provider =
        MidnightDataProvider::new(FetchMode::Synchronous, OutputMode::Log, files.clone())?;
    let mut assets = Vec::new();
    let mut sizes = BTreeSet::new();
    for (name, expected_hash, description) in files {
        if !verify_only {
            provider.fetch(name).await?;
        }
        let mut file = provider.get_file(name, description).await?;
        let bytes = file.get_ref().metadata()?.len();
        let mut asset = json!({
            "name": name,
            "sha256": hex::encode(expected_hash),
            "bytes": bytes,
        });
        if name.ends_with(".bzkir") {
            // This is the upstream cost model used by key generation, applied
            // only after the provider has checked the pinned IR digest.
            let ir: IrSource = tagged_deserialize(&mut file)?;
            let k = ir.model().k();
            sizes.insert(k);
            asset["k"] = json!(k);
        }
        assets.push(asset);
    }
    let mut parameters = Vec::new();
    for k in sizes {
        if !verify_only {
            provider.fetch_k(k).await?;
        }
        let name = MidnightDataProvider::name_k(k);
        let file = provider
            .get_file(&name, "built-in proof SRS missing")
            .await?;
        parameters.push(json!({
            "name": name,
            "k": k,
            "bytes": file.get_ref().metadata()?.len(),
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format": "compact-proof-material/v1",
            "mode": if verify_only { "verify-only" } else { "prepare" },
            "cache_directory": provider.dir,
            "asset_count": assets.len(),
            "assets": assets,
            "parameter_count": parameters.len(),
            "parameters": parameters,
            "verification": "all files checked against locked upstream SHA256 tables",
        }))?
    );
    Ok(())
}
