// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete crypto-family old/native/neutral controls with independent helpers.
//! Root adds ordinary-worker boundaries after the author source handoff.
use super::*;
use protocol::compare;
mod encoding;
mod hash_helpers;
mod jubjub_helpers;
mod nested;
mod old_hash_input;
mod old_jubjub;
mod ordering;
mod outcomes;
mod protocol;
mod source;
mod workers;
use source::{Kind, operation};

fn bytes(length: usize) -> Type {
    Type::Bytes { length }
}
fn tuple(element: Type, length: usize) -> Type {
    Type::Tuple {
        elements: vec![element; length],
    }
}
fn map(element: Type) -> Type {
    Type::LedgerMap {
        key: Box::new(Type::Field),
        value: Box::new(element),
    }
}

#[test]
#[should_panic(expected = "private pure-expression dispatcher selects this family")]
fn wrong_family_private_crypto_adapter_retains_misuse_contract() {
    let _ = crate::pure_crypto_expression(&Expr::Unit, &HashMap::new(), &HashMap::new());
}
