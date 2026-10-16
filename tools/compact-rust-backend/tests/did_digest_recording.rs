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
    // Check the declared map operation, independently of emitter temporaries.
    // This is a syntax regression guard; execution is covered by DID consumers.
    struct DigestLookup(usize);
    impl syn::visit_mut::VisitMut for DigestLookup {
        fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
            if call.method == "record_lookup"
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
                self.0 += 1;
            }
            syn::visit_mut::visit_expr_method_call_mut(self, call);
        }
    }
    let mut syntax = syn::parse_file(&rendered.source).unwrap();
    let mut lookup = DigestLookup(0);
    syn::visit_mut::VisitMut::visit_file_mut(&mut lookup, &mut syntax);
    assert_eq!(
        lookup.0, 1,
        "recorded digest lookup must use its declared map"
    );
}
