// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite raw Rust qualification of native plus independently checked hash calls.
use super::*;

#[test]
#[ignore = "writes a raw consumer for explicit external Rust compilation"]
fn export_keccak_fab_domain_consumer() {
    let mut inputs = leaves();
    inputs.extend([
        Type::Tuple {
            elements: vec![Type::Field; 11],
        },
        Type::Struct {
            name: "TuplePayload".into(),
            fields: vec![StructField {
                name: "values".into(),
                ty: Type::Tuple {
                    elements: vec![Type::Field; 8],
                },
            }],
        },
        Type::Vector {
            element: Box::new(Type::OpaqueString),
            length: 2,
        },
        Type::Struct {
            name: "MixedPayload".into(),
            fields: vec![
                StructField {
                    name: "label".into(),
                    ty: Type::OpaqueString,
                },
                StructField {
                    name: "bytes".into(),
                    ty: Type::OpaqueBytes,
                },
                StructField {
                    name: "field".into(),
                    ty: Type::Field,
                },
            ],
        },
    ]);
    let circuits: Vec<_> = inputs
        .into_iter()
        .enumerate()
        .map(|(i, ty)| {
            let mut circuit = root(ty);
            circuit.name = format!("hash_{i}");
            circuit
        })
        .collect();
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: circuits,
    };
    let rendered = crate::render_with_capabilities(&contract).unwrap();
    let mut methods = Vec::<syn::ItemFn>::new();
    for circuit in &contract.stateful_circuits {
        let checked = checked(circuit).unwrap();
        let emitted = render(&checked).unwrap();
        let (steps, result) = (emitted.steps, emitted.result);
        let name = crate::ident(&circuit.name).unwrap();
        let input = crate::rust_type(&circuit.parameters[0].ty).unwrap();
        methods.push(syn::parse_quote! {
            pub fn #name<Private>(
                context: runtime::context::CircuitContext<Private>, __compact_param_0: #input,
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, runtime::FixedBytes<32>>, runtime::CompactError> {
                let frame = runtime::recording::RecordingFrame::new(context);
                #(#steps)*
                Ok(frame.finish(#result))
            }
        });
    }
    // Explicit candidate module: public production routing is not overridden.
    let module: syn::ItemMod = syn::parse_quote! {
        pub mod checked_keccak_domain { use super::*; #(#methods)* }
    };
    let module = prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![syn::Item::Mod(module)],
    });
    let output = std::path::PathBuf::from(
        std::env::var_os("COMPACT_KECCAK_DOMAIN_OUTPUT").expect("fresh output directory"),
    );
    std::fs::create_dir(&output).unwrap();
    std::fs::write(
        output.join("compact-rust-ir.json"),
        serde_json::to_vec_pretty(&contract).unwrap(),
    )
    .unwrap();
    std::fs::write(
        output.join("lib.rs"),
        format!("{}\n{}", rendered.source, module),
    )
    .unwrap();
    std::fs::write(output.join("case.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "cases": contract.stateful_circuits.len(),
        "capabilities": rendered.capabilities,
        "claim": "Finite native plus explicit checked candidate syntax. Caller must compile this unchanged raw file. No production dispatch or full language inclusion claim."
    })).unwrap()).unwrap();
}
