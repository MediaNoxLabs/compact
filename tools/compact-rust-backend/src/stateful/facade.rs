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

//! Public native contract methods forwarding to already emitted functions.
//!
//! This layer owns signatures and witness bounds; it performs no VM lowering.

use crate::circuit_analysis::circuit_uses_witness;
use crate::ir::StatefulCircuit;
use crate::{RenderError, ident, public_parameter_idents, rust_type};
use std::collections::{HashMap, HashSet};

/// A discoverable public method over the already-rendered circuit function.
/// This layer contains no VM operations or Compact semantics.
pub(crate) fn render_contract_method(
    circuit: &StatefulCircuit,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<syn::ImplItemFn>, RenderError> {
    if circuit.internal {
        return Ok(None);
    }
    let name = ident(&circuit.name)?;
    let uses_witness = circuit_uses_witness(circuit, circuits, &mut HashSet::new())?;
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
    let result_ty = rust_type(&circuit.result)?;
    let method = if uses_witness {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError>
            where
                W: TryWitnesses<Private>,
            {
                crate::ledger_contract::#name(context, &self.witnesses, #(#call_args),*)
            }
        }
    } else {
        syn::parse_quote! {
            pub fn #name<Private>(
                &self,
                context: runtime::context::CircuitContext<Private>,
                #(#args),*
            ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
                crate::ledger_contract::#name(context, #(#call_args),*)
            }
        }
    };
    Ok(Some(method))
}
