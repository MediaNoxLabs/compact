//! Typed witness declarations and contract ledger projection syntax.

use std::collections::{HashMap, HashSet};

use proc_macro2::Span;

use crate::ir::{LedgerField, LedgerFieldKind, WitnessDeclaration};
use crate::{RenderError, ident, rust_type};

pub(crate) struct WitnessSyntax<'a> {
    pub declarations: HashMap<&'a str, &'a WitnessDeclaration>,
    pub trait_methods: Vec<syn::TraitItemFn>,
    pub ledger_view_methods: Vec<syn::ImplItemFn>,
}

pub(crate) fn build<'a>(
    witnesses: &'a [WitnessDeclaration],
    ledger_fields: &[&LedgerField],
) -> Result<WitnessSyntax<'a>, RenderError> {
    let mut declarations = HashMap::new();
    let mut trait_methods = Vec::new();
    for witness in witnesses {
        let name = ident(&witness.name)?;
        if declarations
            .insert(witness.name.as_str(), witness)
            .is_some()
        {
            return Err(RenderError::DuplicateWitness(witness.name.clone()));
        }
        let mut args = Vec::<syn::FnArg>::new();
        let mut parameter_names = HashSet::new();
        for (index, parameter) in witness.parameters.iter().enumerate() {
            ident(&parameter.name)?;
            if !parameter_names.insert(parameter.name.as_str()) {
                return Err(RenderError::DuplicateParameter(parameter.name.clone()));
            }
            let parameter_name =
                syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
            let ty = rust_type(&parameter.ty)?;
            args.push(syn::parse_quote!(#parameter_name: #ty));
        }
        let result = rust_type(&witness.result)?;
        trait_methods.push(syn::parse_quote! {
            fn #name(
                &self,
                context: runtime::context::WitnessContext<'_, Private, LedgerView<'_>>,
                #(#args),*
            ) -> (Private, #result);
        });
    }

    let mut ledger_view_methods = Vec::new();
    if !witnesses.is_empty() {
        for field in ledger_fields {
            let name = ident(&field.id)?;
            let index = syn::LitInt::new(&field.index.to_string(), Span::call_site());
            match &field.declaration {
                LedgerFieldKind::Cell { ty } => {
                    let ty = rust_type(ty)?;
                    ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<#ty, runtime::CompactError> {
                            runtime::ledger::read_root_cell::<#ty, _>(self.state, #index)
                        }
                    });
                }
                LedgerFieldKind::Counter => {
                    let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
                    ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::BoundedUint<#max>, runtime::CompactError> {
                            let value = runtime::ledger::read_root_cell::<u64, _>(self.state, #index)?;
                            runtime::BoundedUint::<#max>::new(value as u128)
                        }
                    });
                }
                LedgerFieldKind::Set { .. }
                | LedgerFieldKind::List { .. }
                | LedgerFieldKind::Map { .. } => {}
            }
        }
    }

    Ok(WitnessSyntax {
        declarations,
        trait_methods,
        ledger_view_methods,
    })
}
