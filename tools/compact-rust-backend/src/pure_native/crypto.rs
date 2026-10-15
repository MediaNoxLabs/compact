// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact runtime call syntax from arity-safe checked crypto facts.
//! No Type/source inspection, eligibility decision or operand checking occurs.
use super::leaf::NativeLeafSink;
use crate::{
    hash_input::render_input,
    jubjub_value,
    pure_semantics::{crypto::CheckedCrypto, sink::CryptoSink},
};

impl CryptoSink for NativeLeafSink {
    fn crypto(&mut self, checked: CheckedCrypto<syn::Expr>) -> syn::Expr {
        match checked {
            CheckedCrypto::TransientHash { value, encoding } => {
                let value = render_input(value, encoding);
                syn::parse_quote!(runtime::transient_hash(#value))
            }
            CheckedCrypto::TransientCommit {
                value,
                opening,
                encoding,
            } => {
                let value = render_input(value, encoding);
                syn::parse_quote!(runtime::transient_commit(#value, #opening))
            }
            CheckedCrypto::PersistentHash(value) => {
                syn::parse_quote!(runtime::persistent_hash(#value))
            }
            CheckedCrypto::PersistentCommit { value, opening } => {
                syn::parse_quote!(runtime::persistent_commit(#value, #opening))
            }
            CheckedCrypto::Keccak256(value) => syn::parse_quote!(runtime::keccak256(#value)),
            CheckedCrypto::DegradeToTransient(value) => {
                syn::parse_quote!(runtime::degrade_to_transient(#value))
            }
            CheckedCrypto::UpgradeFromTransient(value) => {
                syn::parse_quote!(runtime::upgrade_from_transient(#value))
            }
            CheckedCrypto::HashToCurve { value, encoding } => {
                let value = render_input(value, encoding);
                syn::parse_quote!(runtime::hash_to_curve(#value))
            }
            CheckedCrypto::Jubjub(checked) => jubjub_value::render(checked),
        }
    }
}
