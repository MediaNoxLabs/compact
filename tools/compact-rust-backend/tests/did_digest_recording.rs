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

use compact_rust_backend::{ir::Contract, render_with_capabilities};

#[test]
fn original_did_digest_gets_recorded_and_observed_api() {
    let source = include_str!("did-digest-schema20-ir.json");
    let contract: Contract = serde_json::from_str(source).unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    let circuit = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
        .unwrap();
    assert!(circuit.recorded, "{circuit:?}");
    assert!(circuit.observed_call, "{circuit:?}");
    assert!(
        rendered
            .source
            .contains("pub fn verifySchnorrJubjubDigestSignature<")
    );
    assert!(circuit.recording_unavailable.is_none());
    assert!(circuit.observed_call_unavailable.is_none());

    // Select the recorded root, excluding its native and facade counterparts.
    fn recorded_body(items: &[syn::Item]) -> Option<syn::Block> {
        for item in items {
            let syn::Item::Mod(module) = item else {
                continue;
            };
            let Some((_, children)) = &module.content else {
                continue;
            };
            if module.ident == "recorded" {
                for child in children {
                    if let syn::Item::Fn(function) = child
                        && function.sig.ident == "verifySchnorrJubjubDigestSignature"
                    {
                        return Some((*function.block).clone());
                    }
                }
            }
            if let Some(body) = recorded_body(children) {
                return Some(body);
            }
        }
        None
    }
    // Forward only the syntax correction from c64c6bb0. Check the declared
    // map/frame/input identity independently of emitter temporary spellings.
    #[derive(Default)]
    struct DigestLookup {
        member_key: Option<String>,
        lookup_key: Option<String>,
        members: usize,
        lookups: usize,
    }
    impl syn::visit_mut::VisitMut for DigestLookup {
        fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
            use quote::ToTokens;
            if (call.method == "record_lookup" || call.method == "record_member")
                && let syn::Expr::Path(receiver) = call.receiver.as_ref()
                && receiver
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .eq(["crate", "ledger_slots", "schnorrJubjubVerificationMethods"])
            {
                assert_eq!(call.args.len(), 2);
                assert!(
                    matches!(call.args.first(), Some(syn::Expr::Path(arg)) if arg.path.is_ident("frame"))
                );
                let key = call
                    .args
                    .iter()
                    .nth(1)
                    .unwrap()
                    .to_token_stream()
                    .to_string();
                if call.method == "record_lookup" {
                    self.lookups += 1;
                    self.lookup_key = Some(key);
                } else {
                    self.members += 1;
                    self.member_key = Some(key);
                }
            }
            syn::visit_mut::visit_expr_method_call_mut(self, call);
        }
    }
    use quote::ToTokens;
    let syntax = syn::parse_file(&rendered.source).unwrap();
    let mut body = recorded_body(&syntax.items).expect("recorded DID digest root");
    let tokens = body.to_token_stream().to_string();
    let mut lookup = DigestLookup::default();
    syn::visit_mut::VisitMut::visit_block_mut(&mut lookup, &mut body);
    assert_eq!((lookup.members, lookup.lookups), (1, 1));
    assert_eq!(lookup.member_key, lookup.lookup_key);
    // Guard, membership and lookup must precede the inlined verifier, once each.
    let operations = [
        "ledger_slots :: active . record_read",
        "Contract is not active",
        "schnorrJubjubVerificationMethods . record_member",
        "Verification method does not exist",
        "schnorrJubjubVerificationMethods . record_lookup",
        "runtime :: transient_hash (",
        "getSchnorrReduction",
        "runtime :: ec_mul_generator (",
        "runtime :: ec_mul (",
        "runtime :: ec_add (",
    ];
    let positions: Vec<_> = operations
        .into_iter()
        .map(|operation| {
            assert_eq!(tokens.matches(operation).count(), 1, "{operation}");
            tokens.find(operation).unwrap()
        })
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!tokens.contains("call_local"));

    // Typed lookup keys and canonical slot identity remain native authorities.
    let mut wrong_key = contract.clone();
    let field = wrong_key
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "schnorrJubjubVerificationMethods")
        .unwrap();
    let compact_rust_backend::ir::LedgerFieldKind::Map { key, .. } = &mut field.declaration else {
        unreachable!()
    };
    *key = compact_rust_backend::ir::Type::Field;
    assert!(render_with_capabilities(&wrong_key).is_err());
    let mut wrong_slot = contract.clone();
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "schnorrJubjubVerificationMethods")
        .unwrap()
        .index = 99;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}
