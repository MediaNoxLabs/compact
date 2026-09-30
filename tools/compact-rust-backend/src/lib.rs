//! Rust syntax construction from Compact's typed backend IR.

pub mod ir;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use ir::{Contract, Expr, SCHEMA_VERSION, Type};
use proc_macro2::Span;
use quote::quote;

#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    SchemaVersion(u32),
    InvalidIdentifier(String),
    DuplicateCircuit(String),
    DuplicateParameter(String),
    UnknownParameter(String),
    TypeMismatch { expected: Type, actual: Type },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion(version) => {
                write!(f, "unsupported Rust backend IR schema {version}")
            }
            Self::InvalidIdentifier(name) => write!(f, "invalid Rust identifier {name:?}"),
            Self::DuplicateCircuit(name) => write!(f, "duplicate circuit {name:?}"),
            Self::DuplicateParameter(name) => write!(f, "duplicate parameter {name:?}"),
            Self::UnknownParameter(name) => write!(f, "unknown parameter {name:?}"),
            Self::TypeMismatch { expected, actual } => {
                write!(f, "expression has type {actual:?}, expected {expected:?}")
            }
        }
    }
}

impl Error for RenderError {}

fn ident(name: &str) -> Result<syn::Ident, RenderError> {
    syn::parse_str::<syn::Ident>(name).map_err(|_| RenderError::InvalidIdentifier(name.to_owned()))
}

fn rust_type(ty: &Type) -> syn::Type {
    match ty {
        Type::Unit => syn::parse_quote!(()),
        Type::Boolean => syn::parse_quote!(bool),
        Type::Field => syn::parse_quote!(runtime::Field),
        Type::Tuple { elements } => {
            let elements: Vec<syn::Type> = elements.iter().map(rust_type).collect();
            let mut tuple = syn::TypeTuple {
                paren_token: syn::token::Paren::default(),
                elems: syn::punctuated::Punctuated::new(),
            };
            for element in elements {
                tuple.elems.push_value(element);
                tuple.elems.push_punct(syn::token::Comma::default());
            }
            if tuple.elems.len() > 1 {
                tuple.elems.pop_punct();
            }
            syn::Type::Tuple(tuple)
        }
        Type::Vector { element, length } => {
            let element = rust_type(element);
            let length = syn::LitInt::new(&length.to_string(), Span::call_site());
            syn::parse_quote!([#element; #length])
        }
    }
}

fn expression(
    expr: &Expr,
    parameters: &HashMap<&str, &Type>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::Unit => Ok((syn::parse_quote!(()), Type::Unit)),
        Expr::Boolean { value } => Ok((syn::parse_quote!(#value), Type::Boolean)),
        Expr::Parameter { name } => {
            let ty = parameters
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
            let name = ident(name)?;
            Ok((syn::parse_quote!(#name), (*ty).clone()))
        }
        Expr::Tuple { elements } => {
            let (exprs, types): (Vec<_>, Vec<_>) = elements
                .iter()
                .map(|element| expression(element, parameters))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .unzip();
            let mut tuple = syn::ExprTuple {
                attrs: vec![],
                paren_token: syn::token::Paren::default(),
                elems: syn::punctuated::Punctuated::new(),
            };
            for expr in exprs {
                tuple.elems.push_value(expr);
                tuple.elems.push_punct(syn::token::Comma::default());
            }
            if tuple.elems.len() > 1 {
                tuple.elems.pop_punct();
            }
            Ok((syn::Expr::Tuple(tuple), Type::Tuple { elements: types }))
        }
        Expr::Add { left, right } => {
            let (left, left_type) = expression(left, parameters)?;
            let (right, right_type) = expression(right, parameters)?;
            if left_type != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: left_type,
                });
            }
            if right_type != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: right_type,
                });
            }
            Ok((
                syn::Expr::Binary(syn::ExprBinary {
                    attrs: vec![],
                    left: Box::new(left),
                    op: syn::BinOp::Add(syn::token::Plus::default()),
                    right: Box::new(right),
                }),
                Type::Field,
            ))
        }
    }
}

pub fn render(contract: &Contract) -> Result<String, RenderError> {
    if contract.schema_version != SCHEMA_VERSION {
        return Err(RenderError::SchemaVersion(contract.schema_version));
    }

    let mut names = HashSet::new();
    let mut items = Vec::new();
    for circuit in &contract.circuits {
        let name = ident(&circuit.name)?;
        if !names.insert(circuit.name.as_str()) {
            return Err(RenderError::DuplicateCircuit(circuit.name.clone()));
        }
        let mut parameters = HashMap::new();
        let mut args = Vec::new();
        for parameter in &circuit.parameters {
            let arg_name = ident(&parameter.name)?;
            if parameters
                .insert(parameter.name.as_str(), &parameter.ty)
                .is_some()
            {
                return Err(RenderError::DuplicateParameter(parameter.name.clone()));
            }
            let arg_ty = rust_type(&parameter.ty);
            let arg: syn::FnArg = syn::parse_quote!(#arg_name: #arg_ty);
            args.push(arg);
        }
        let (body, actual) = expression(&circuit.body, &parameters)?;
        if actual != circuit.result {
            return Err(RenderError::TypeMismatch {
                expected: circuit.result.clone(),
                actual,
            });
        }
        let result = rust_type(&circuit.result);
        let item: syn::Item = syn::parse_quote! {
            pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                Ok(#body)
            }
        };
        items.push(item);
    }

    let file: syn::File = syn::parse2(quote! {
        pub mod pure_circuits {
            use midnight_compact_runtime as runtime;
            #(#items)*
        }
    })
    .expect("typed renderer constructed invalid Rust syntax");
    Ok(format!(
        "// Generated by compactc. Do not edit.\n\n{}",
        prettyplease::unparse(&file)
    ))
}
