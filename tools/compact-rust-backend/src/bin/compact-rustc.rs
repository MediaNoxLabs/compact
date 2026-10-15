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

//! Compile a Compact source with the Scheme frontend, then render its Rust IR.
//!
//! `COMPACTC` selects the compiler executable. It defaults to `compactc` on
//! PATH. Process arguments are passed directly, without shell interpolation.

// Retain the original binary input-reader tests; production ingress is issued.
#[path = "../bin_support/input.rs"]
mod input;
#[path = "../bin_support/source_diagnostic.rs"]
mod source_diagnostic;
#[path = "../bin_support/source_maps.rs"]
mod source_maps;
#[path = "../bin_support/staged_output.rs"]
mod staged_output;
use staged_output::StagedOutput;

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process;

use compact_rust_backend::{ConfiguredFrontend, FrontendError, FrontendRequest, RenderOptions};

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let source = PathBuf::from(
        args.next()
            .ok_or("usage: compact-rustc <source.compact> <output-directory>")?,
    );
    let output = PathBuf::from(
        args.next()
            .ok_or("usage: compact-rustc <source.compact> <output-directory>")?,
    );
    if args.next().is_some() {
        return Err("usage: compact-rustc <source.compact> <output-directory>".into());
    }

    let compiler = env::var_os("COMPACTC").unwrap_or_else(|| "compactc".into());
    let staging = StagedOutput::new(&output)?;
    let artifacts = ConfiguredFrontend::new(compiler)
        .invoke(FrontendRequest {
            source,
            arguments: vec!["--skip-zk".into()],
            stage_parent: staging.path().to_path_buf(),
        })
        .map_err(|error| match error {
            FrontendError::ProcessExit { code: Some(code) } => {
                format!("compactc failed with exit status: {code}").into()
            }
            error => Box::new(error) as Box<dyn Error>,
        })?;
    // Preserve the original bounded file-reader and JSON decoding error phase.
    // Drop this prevalidation value before the issued facade's semantic pass.
    let decoded: compact_rust_backend::ir::Contract = serde_json::from_slice(&input::read(
        fs::File::open(artifacts.output_dir().join("contract/compact-rust-ir.json"))?,
    )?)?;
    drop(decoded);
    let issued = artifacts
        .render(RenderOptions::default())
        .map_err(|error| error as Box<dyn Error>)?;
    // Only successful issued results may be moved out of the exclusive child.
    // Promotion rechecks metadata and exact pair bytes under our output lock.
    let original_frontend = artifacts.output_dir().to_path_buf();
    artifacts.promote_into(&issued, staging.path())?;
    source_maps::rebase(staging.path(), &original_frontend, &output)?;
    fs::write(
        staging.path().join("contract/lib.rs"),
        &issued.rendered().source,
    )?;
    staging.publish(&output)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("compact-rustc: {error}");
        source_diagnostic::failure("compact-rustc", error.as_ref());
        process::exit(1);
    }
}
