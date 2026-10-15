// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Genuine source roots qualify the new boundary separately from local inlining.
use super::*;

#[test]
fn genuine_internal_roots_check_directly_and_stay_out_of_public_capabilities() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-internal-interfaces-schema20-ir.json"
    ))
    .unwrap();
    let ledger = source
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = source
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = source
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let circuits: HashMap<_, _> = source
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let mut harness = Vec::<syn::ItemFn>::new();
    for name in ["touch", "project"] {
        let root = circuits[name];
        assert!(root.internal);
        assert_eq!(root.parameters.len(), 1);
        assert!(
            matches!(&root.parameters[0].ty, Type::Tuple { elements } if elements.len() == 12 && elements[0] == Type::OpaqueBytes)
        );
        if name == "touch" {
            assert_eq!(root.result, root.parameters[0].ty);
        }
        let checked = check_with_calls(root, &ledger, &witnesses, &pure, &circuits).unwrap();
        let typed = render(&checked).unwrap();
        // Same source body and types, changed boundary only. Do not weaken the
        // public FAB policy to make the internal plan pass.
        let mut public = root.clone();
        public.internal = false;
        assert!(matches!(
            check_with_calls(&public, &ledger, &witnesses, &pure, &circuits),
            Err(PlanError::Unsupported(_))
        ));
        let ident = crate::ident(name).unwrap();
        let argument = crate::rust_type(&root.parameters[0].ty).unwrap();
        let result = crate::rust_type(&root.result).unwrap();
        let steps = typed.steps;
        let value = typed.result;
        // This test-only shell compiles the exact checked root fragment. The
        // normal public source only tests already-supported nested call bodies.
        harness.push(syn::parse_quote! {
            pub fn #ident<Private, W: crate::ledger_contract::TryWitnesses<Private>>(
                context: runtime::context::CircuitContext<Private>,
                witnesses: &W,
                __compact_param_0: #argument,
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError> {
                let frame = runtime::recording::RecordingFrame::new(context);
                #(#steps)*
                Ok(frame.finish(#value))
            }
        });
    }
    // Additional local-only carriers compile with the same checked fragment
    // shell; they are synthetic controls, not extra source-derived exports.
    for (name, ty) in [
        ("echo_tuple13", tuple(13)),
        ("echo_vector_tuple12", vector(tuple(12), 2)),
    ] {
        let synthetic = root(ty, true);
        let typed = render(&checked(&synthetic)).unwrap();
        let ident = crate::ident(name).unwrap();
        let ty = crate::rust_type(&synthetic.result).unwrap();
        let steps = typed.steps;
        let value = typed.result;
        harness.push(syn::parse_quote! {
            pub fn #ident<Private>(
                context: runtime::context::CircuitContext<Private>,
                __compact_param_0: #ty,
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #ty>, runtime::CompactError> {
                let frame = runtime::recording::RecordingFrame::new(context);
                #(#steps)*
                Ok(frame.finish(#value))
            }
        });
    }
    let output = crate::render_with_capabilities(&source).unwrap();
    assert_eq!(output.capabilities.circuits.len(), 1);
    let capability = &output.capabilities.circuits[0];
    assert_eq!(capability.name, "run");
    assert!(capability.recorded && capability.observed_call);
    assert!(!output.source.contains("fn touch_call"));
    assert!(!output.source.contains("fn project_call"));
    // Renderer fragments construct the private LedgerView representation and
    // must live under the same ledger_contract parent as production recording.
    let injected: syn::ItemMod = syn::parse_quote! {
        pub mod checked_roots {
            use midnight_compact_runtime as runtime;
            #(#harness)*
        }
    };
    let original = syn::parse_file(&output.source).unwrap();
    let mut assembled = original.clone();
    let ledger_module = assembled
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => Some(module),
            _ => None,
        })
        .unwrap();
    let ledger_items = &mut ledger_module.content.as_mut().unwrap().1;
    assert!(
        !ledger_items
            .iter()
            .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == "checked_roots"))
    );
    ledger_items.push(syn::Item::Mod(injected.clone()));
    assembled.items.push(syn::parse_quote!(
        pub use crate::ledger_contract::checked_roots;
    ));
    let qualification = prettyplease::unparse(&assembled);
    // Formatting/injection must preserve every existing generated AST item.
    let mut restored = syn::parse_file(&qualification).unwrap();
    assert!(matches!(restored.items.pop(), Some(syn::Item::Use(_))));
    let restored_module = restored
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => Some(module),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(restored_module.content.as_mut().unwrap().1.pop(), Some(syn::Item::Mod(module)) if module.ident == "checked_roots")
    );
    assert_eq!(
        quote::quote!(#restored).to_string(),
        quote::quote!(#original).to_string()
    );
    if let Some(destination) = std::env::var_os("COMPACT_INTERNAL_ROOT_PROBE_OUTPUT") {
        let dir = std::path::PathBuf::from(destination);
        std::fs::create_dir(&dir).expect("use a fresh internal root export directory");
        let fragment: syn::File = syn::parse_quote!(#injected);
        std::fs::write(dir.join("generated.rs"), output.source).unwrap();
        std::fs::write(
            dir.join("checked-roots.rs"),
            prettyplease::unparse(&fragment),
        )
        .unwrap();
        std::fs::write(dir.join("qualification-source.rs"), qualification).unwrap();
        std::fs::write(
            dir.join("lib.rs"),
            "include!(\"qualification-source.rs\");\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(&source).unwrap(),
        )
        .unwrap();
    }
}
