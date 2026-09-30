use std::error::Error;
use std::io::{self, Read};

use compact_rust_backend::{ir::Contract, render};

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let contract: Contract = serde_json::from_str(&input)?;
    print!("{}", render(&contract)?);
    Ok(())
}
