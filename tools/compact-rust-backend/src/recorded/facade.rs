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

//! Emit consumer-facing recording handles and observed-call methods.
//!
//! This component owns method signatures, witness borrowing, identifier hygiene,
//! and declared input/entry-point packaging. Recording admission and execution
//! lowering remain in the parent module and its planners.

use proc_macro2::Span;

use crate::ir::StatefulCircuit;
use crate::{RenderError, ident, public_parameter_idents, retained_value, rust_type};

/// A recording handle that borrows the user-supplied witness implementation.
pub(crate) fn render_borrowed_recorded_contract_method(
    circuit: &StatefulCircuit,
    uses_witness: bool,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        call_args.push(arg);
    }
    // Compare emitted identifiers: Compact `$` normalization and Rust raw
    // identifiers can make distinct source spellings shadow the circuit name.
    let spelling = |id: &syn::Ident| id.to_string().trim_start_matches("r#").to_owned();
    let callee: syn::Expr = if spelling(&name) == "context"
        || call_args.iter().any(|arg| spelling(arg) == spelling(&name))
    {
        syn::parse_quote!(crate::ledger_contract::recorded::#name)
    } else {
        syn::parse_quote!(#name)
    };
    let result = rust_type(&circuit.result)?;
    let method = if uses_witness {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError>
            where W: super::TryWitnesses<Private> {
                #callee(context, self.witnesses, #(#call_args),*)
            }
        }
    } else {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError> {
                #callee(context, #(#call_args),*)
            }
        }
    };
    Ok(method)
}

/// A typed method on the generated recording handle. The ledger program is
/// still produced by the corresponding `recorded` module function.
pub(crate) fn render_recorded_contract_method(
    circuit: &StatefulCircuit,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        call_args.push(arg);
    }
    let result = rust_type(&circuit.result)?;
    Ok(syn::parse_quote! {
        pub fn #name<Private>(
            &self,
            context: runtime::context::CircuitContext<Private>,
            #(#args),*
        ) -> Result<runtime::recording::RecordedCircuitResult<Private, #result>, runtime::CompactError> {
            crate::ledger_contract::recorded::#name(context, #(#call_args),*)
        }
    })
}

/// A generated call handle carries the circuit's declared input and entry
/// point into ledger preparation, so consumers cannot repeat them incorrectly.
pub(crate) fn render_observed_call_method(
    circuit: &StatefulCircuit,
    uses_witness: bool,
) -> Result<syn::ImplItemFn, RenderError> {
    let name = ident(&circuit.name)?;
    let call_name = ident(&format!("{}_call", circuit.name))?;
    let entry_point = syn::LitStr::new(&circuit.name, Span::call_site());
    let mut args = Vec::<syn::FnArg>::new();
    let mut call_args = Vec::<syn::Ident>::new();
    let mut input_args = Vec::<syn::Expr>::new();
    for (parameter, arg) in circuit
        .parameters
        .iter()
        .zip(public_parameter_idents(&circuit.parameters))
    {
        let ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#arg: #ty));
        input_args.push(retained_value(syn::parse_quote!(#arg), &parameter.ty));
        call_args.push(arg);
    }
    let input: syn::Expr = match input_args.as_slice() {
        [] => syn::parse_quote!(runtime::fab::AlignedValue::from(())),
        [single] => syn::parse_quote!(runtime::fab::AlignedValue::from(#single)),
        [first, second] => {
            syn::parse_quote!(runtime::fab::AlignedValue::from((#first, #second)))
        }
        _ => syn::parse_quote!(runtime::fab::AlignedValue::concat(&[
            #(runtime::fab::AlignedValue::from(#input_args)),*
        ])),
    };
    let result = rust_type(&circuit.result)?;
    let method = if uses_witness {
        syn::parse_quote! {
            #[cfg(feature = "ledger-transaction")]
            pub fn #call_name<'observed, Private>(
                &self,
                observed: &'observed runtime::transaction::ObservedContractState,
                private_state: Private,
                #(#args),*
            ) -> Result<runtime::transaction::RecordedCall<'observed, Private, #result>, runtime::CompactError>
            where W: super::TryWitnesses<Private> {
                let input = #input;
                let recorded = self.#name(observed.circuit_context(private_state), #(#call_args),*)?;
                Ok(runtime::transaction::RecordedCall::new(observed, recorded, #entry_point, input))
            }
        }
    } else {
        syn::parse_quote! {
            #[cfg(feature = "ledger-transaction")]
            pub fn #call_name<'observed, Private>(
                &self,
                observed: &'observed runtime::transaction::ObservedContractState,
                private_state: Private,
                #(#args),*
            ) -> Result<runtime::transaction::RecordedCall<'observed, Private, #result>, runtime::CompactError> {
                let input = #input;
                let recorded = self.#name(observed.circuit_context(private_state), #(#call_args),*)?;
                Ok(runtime::transaction::RecordedCall::new(observed, recorded, #entry_point, input))
            }
        }
    };
    Ok(method)
}
