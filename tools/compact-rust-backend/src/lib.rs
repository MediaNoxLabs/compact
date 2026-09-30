//! Rust syntax construction from Compact's typed backend IR.

pub mod ir;

const RUNTIME_ABI_VERSION: u32 = 1;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use ir::{Contract, Expr, LedgerFieldKind, SCHEMA_VERSION, StateAction, Type};
use proc_macro2::Span;
use quote::quote;

#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    SchemaVersion(u32),
    InvalidIdentifier(String),
    InvalidUnsignedMaximum(String),
    DuplicateCircuit(String),
    DuplicateParameter(String),
    DuplicateLedgerField(String),
    InvalidLedgerIndex(u8),
    UnknownLedgerField(String),
    UnsupportedLedgerCellType(Type),
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
            Self::InvalidUnsignedMaximum(max) => {
                write!(
                    f,
                    "unsupported Compact Uint maximum {max:?}; expected canonical u128"
                )
            }
            Self::DuplicateCircuit(name) => write!(f, "duplicate circuit {name:?}"),
            Self::DuplicateParameter(name) => write!(f, "duplicate parameter {name:?}"),
            Self::DuplicateLedgerField(name) => write!(f, "duplicate ledger field {name:?}"),
            Self::InvalidLedgerIndex(index) => {
                write!(f, "invalid or noncontiguous ledger field index {index}")
            }
            Self::UnknownLedgerField(name) => write!(f, "unknown ledger field {name:?}"),
            Self::UnsupportedLedgerCellType(ty) => write!(f, "unsupported ledger Cell type {ty:?}"),
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

fn rust_type(ty: &Type) -> Result<syn::Type, RenderError> {
    Ok(match ty {
        Type::Unit => syn::parse_quote!(()),
        Type::Boolean => syn::parse_quote!(bool),
        Type::Field => syn::parse_quote!(runtime::Field),
        Type::Bytes { length } => {
            let length = syn::LitInt::new(&length.to_string(), Span::call_site());
            syn::parse_quote!([u8; #length])
        }
        Type::Unsigned { max } => {
            let parsed = max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(max.clone()))?;
            if parsed.to_string() != *max {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
            }
            let max = syn::LitInt::new(max, Span::call_site());
            syn::parse_quote!(runtime::BoundedUint<#max>)
        }
        Type::Tuple { elements } => {
            let elements: Vec<syn::Type> =
                elements.iter().map(rust_type).collect::<Result<_, _>>()?;
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
            let element = rust_type(element)?;
            let length = syn::LitInt::new(&length.to_string(), Span::call_site());
            syn::parse_quote!([#element; #length])
        }
    })
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
    let mut ledger_fields = HashMap::new();
    let mut ordered_fields = contract.ledger_fields.iter().collect::<Vec<_>>();
    ordered_fields.sort_by_key(|field| field.index);
    for (expected_index, field) in ordered_fields.iter().enumerate() {
        if field.index as usize != expected_index || field.index >= 16 {
            return Err(RenderError::InvalidLedgerIndex(field.index));
        }
        if ledger_fields.insert(field.id.as_str(), *field).is_some() {
            return Err(RenderError::DuplicateLedgerField(field.id.clone()));
        }
    }
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
            let arg_ty = rust_type(&parameter.ty)?;
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
        let result = rust_type(&circuit.result)?;
        let item: syn::Item = syn::parse_quote! {
            pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                Ok(#body)
            }
        };
        items.push(item);
    }

    let mut stateful_items = Vec::new();
    for circuit in &contract.stateful_circuits {
        let name = ident(&circuit.name)?;
        if !names.insert(circuit.name.as_str()) {
            return Err(RenderError::DuplicateCircuit(circuit.name.clone()));
        }
        let mut statements = Vec::<syn::Stmt>::new();
        for action in &circuit.actions {
            match action {
                StateAction::CounterIncrement {
                    field,
                    index,
                    amount,
                } => {
                    let declaration = ledger_fields
                        .get(field.as_str())
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                    if declaration.declaration != LedgerFieldKind::Counter
                        || declaration.index != *index
                    {
                        return Err(RenderError::UnknownLedgerField(field.clone()));
                    }
                    let index = syn::LitInt::new(&declaration.index.to_string(), Span::call_site());
                    let amount = syn::LitInt::new(&amount.to_string(), Span::call_site());
                    statements.push(syn::parse_quote! {
                        let step = context.increment_counter(#index, #amount)?;
                    });
                    statements.push(syn::parse_quote! {
                        let context = step.context;
                    });
                    statements.push(syn::parse_quote! {
                        total_cost += step.gas_cost;
                    });
                }
                StateAction::CellWrite {
                    field,
                    index,
                    value,
                } => {
                    let declaration = ledger_fields
                        .get(field.as_str())
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                    let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                        return Err(RenderError::UnknownLedgerField(field.clone()));
                    };
                    if declaration.index != *index {
                        return Err(RenderError::UnknownLedgerField(field.clone()));
                    }
                    let (value, actual) = expression(value, &HashMap::new())?;
                    if &actual != ty {
                        return Err(RenderError::TypeMismatch {
                            expected: ty.clone(),
                            actual,
                        });
                    }
                    let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                    statements.push(syn::parse_quote! {
                        let step = context.write_cell(#index, #value)?;
                    });
                    statements.push(syn::parse_quote! {
                        let context = step.context;
                    });
                    statements.push(syn::parse_quote! {
                        total_cost += step.gas_cost;
                    });
                }
            }
        }
        let item: syn::Item = syn::parse_quote! {
            pub fn #name<Private>(
                context: runtime::context::CircuitContext<Private>,
            ) -> Result<runtime::context::CircuitResult<Private, ()>, runtime::CompactError> {
                let mut total_cost = runtime::context::RunningCost::default();
                #(#statements)*
                Ok(runtime::context::CircuitResult { context, result: (), gas_cost: total_cost })
            }
        };
        stateful_items.push(item);
    }

    let constructor_fields = ordered_fields.iter().map(|field| match &field.declaration {
        LedgerFieldKind::Counter => Ok(syn::parse_quote!(runtime::ledger::constructor_counter())),
        LedgerFieldKind::Cell { ty } => {
            if !matches!(ty, Type::Boolean | Type::Field | Type::Unsigned { .. } | Type::Bytes { .. }) {
                return Err(RenderError::UnsupportedLedgerCellType(ty.clone()));
            }
            let ty = rust_type(ty)?;
            Ok(syn::parse_quote!(runtime::ledger::constructor_cell::<#ty, runtime::ledger::DefaultDB>(Default::default())))
        }
    }).collect::<Result<Vec<syn::Expr>, RenderError>>()?;

    let runtime_abi = syn::LitInt::new(&RUNTIME_ABI_VERSION.to_string(), Span::call_site());
    let ledger_module: Option<syn::Item> =
        if contract.ledger_fields.is_empty() && contract.stateful_circuits.is_empty() {
            None
        } else {
            Some(syn::parse_quote! {
                pub mod ledger_contract {
                    use midnight_compact_runtime as runtime;
                    const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
                    pub fn initial_state<Private>(
                        context: runtime::context::ConstructorContext<Private>,
                    ) -> runtime::context::ConstructorResult<Private> {
                        let state = runtime::ledger::contract_state(vec![#(#constructor_fields),*]);
                        runtime::context::ConstructorResult::new(context, state)
                    }
                    #(#stateful_items)*
                }
            })
        };
    let file: syn::File = syn::parse2(quote! {
        pub mod pure_circuits {
            use midnight_compact_runtime as runtime;
            const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
            #(#items)*
        }
        #ledger_module
    })
    .expect("typed renderer constructed invalid Rust syntax");
    Ok(format!(
        "// Generated by compactc. Do not edit.\n\n{}",
        prettyplease::unparse(&file)
    ))
}
