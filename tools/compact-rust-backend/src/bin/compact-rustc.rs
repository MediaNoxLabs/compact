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

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};

use compact_rust_backend::{ir::Contract, render};

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
    let status = Command::new(compiler)
        .arg("--skip-zk")
        .arg("--emit-rust-ir")
        .arg(&source)
        .arg(&output)
        .status()?;
    if !status.success() {
        return Err(format!("compactc failed with {status}").into());
    }

    let contract_dir = output.join("contract");
    let ir_path = contract_dir.join("compact-rust-ir.json");
    let ir: Contract = serde_json::from_slice(&fs::read(&ir_path)?)?;
    let rust = render(&ir)?;
    fs::write(contract_dir.join("lib.rs"), rust)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("compact-rustc: {error}");
        process::exit(1);
    }
}
