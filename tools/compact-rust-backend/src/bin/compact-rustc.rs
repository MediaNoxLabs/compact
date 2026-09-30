//! Compile a Compact source with the Scheme frontend, then render its Rust IR.
//!
//! `COMPACTC` selects the compiler executable. It defaults to `compactc` on
//! PATH. Process arguments are passed directly, without shell interpolation.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use compact_rust_backend::{ir::Contract, render};

fn main() -> Result<(), Box<dyn Error>> {
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
