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

use std::error::Error;
use std::io;

#[path = "bin_support/input.rs"]
mod input;
use std::process;

use compact_rust_backend::{ir::Contract, render};

fn run() -> Result<(), Box<dyn Error>> {
    let bytes = input::read(io::stdin().lock())?;
    let text = String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })?;
    let contract: Contract = serde_json::from_str(&text)?;
    print!("{}", render(&contract)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("compact-rust-backend: {error}");
        process::exit(1);
    }
}
