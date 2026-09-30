//! Rust syntax construction from Compact's typed backend IR.

pub mod ir;

const RUNTIME_ABI_VERSION: u32 = 1;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;

use ir::{Contract, Expr, LedgerFieldKind, SCHEMA_VERSION, StateAction, StructField, Type};
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
    ConflictingStruct(String),
    DuplicateStructField(String),
    ConflictingEnum(String),
    EmptyEnum(String),
    DuplicateEnumVariant(String),
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
            Self::ConflictingStruct(name) => {
                write!(f, "conflicting definitions for struct {name:?}")
            }
            Self::DuplicateStructField(name) => write!(f, "duplicate struct field {name:?}"),
            Self::ConflictingEnum(name) => write!(f, "conflicting definitions for enum {name:?}"),
            Self::EmptyEnum(name) => write!(f, "enum {name:?} has no variants"),
            Self::DuplicateEnumVariant(name) => write!(f, "duplicate enum variant {name:?}"),
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
            syn::parse_quote!(runtime::FixedBytes<#length>)
        }
        Type::Struct { name, .. } | Type::Enum { name, .. } => {
            let name = ident(name)?;
            syn::parse_quote!(crate::types::#name)
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
            syn::parse_quote!(runtime::FixedVector<#element, #length>)
        }
    })
}

fn collect_named_types(
    ty: &Type,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), RenderError> {
    match ty {
        Type::Struct { name, fields } => {
            ident(name)?;
            if enums.contains_key(name) {
                return Err(RenderError::ConflictingStruct(name.clone()));
            }
            if let Some(existing) = structs.get(name) {
                if existing != fields {
                    return Err(RenderError::ConflictingStruct(name.clone()));
                }
            } else {
                let mut names = HashSet::new();
                for field in fields {
                    ident(&field.name)?;
                    if !names.insert(field.name.as_str()) {
                        return Err(RenderError::DuplicateStructField(field.name.clone()));
                    }
                }
                structs.insert(name.clone(), fields.clone());
            }
            for field in fields {
                collect_named_types(&field.ty, structs, enums)?;
            }
        }
        Type::Enum { name, variants } => {
            ident(name)?;
            if structs.contains_key(name) {
                return Err(RenderError::ConflictingEnum(name.clone()));
            }
            if variants.is_empty() {
                return Err(RenderError::EmptyEnum(name.clone()));
            }
            let mut names = HashSet::new();
            for variant in variants {
                ident(variant)?;
                if !names.insert(variant.as_str()) {
                    return Err(RenderError::DuplicateEnumVariant(variant.clone()));
                }
            }
            if let Some(existing) = enums.get(name) {
                if existing != variants {
                    return Err(RenderError::ConflictingEnum(name.clone()));
                }
            } else {
                enums.insert(name.clone(), variants.clone());
            }
        }
        Type::Tuple { elements } => {
            for element in elements {
                collect_named_types(element, structs, enums)?;
            }
        }
        Type::Vector { element, .. } => collect_named_types(element, structs, enums)?,
        _ => {}
    }
    Ok(())
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
    let mut struct_definitions = BTreeMap::new();
    let mut enum_definitions = BTreeMap::new();
    for circuit in &contract.circuits {
        for parameter in &circuit.parameters {
            collect_named_types(
                &parameter.ty,
                &mut struct_definitions,
                &mut enum_definitions,
            )?;
        }
        collect_named_types(
            &circuit.result,
            &mut struct_definitions,
            &mut enum_definitions,
        )?;
    }
    for field in &contract.ledger_fields {
        if let LedgerFieldKind::Cell { ty } = &field.declaration {
            collect_named_types(ty, &mut struct_definitions, &mut enum_definitions)?;
        }
    }
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
    let mut struct_items = Vec::<syn::Item>::new();
    for (name, fields) in &struct_definitions {
        let name = ident(name)?;
        let fields = fields
            .iter()
            .map(|field| {
                let field_name = ident(&field.name)?;
                let field_ty = rust_type(&field.ty)?;
                Ok(syn::parse_quote!(pub #field_name: #field_ty))
            })
            .collect::<Result<Vec<syn::Field>, RenderError>>()?;
        struct_items.push(syn::parse_quote! {
            #[derive(Clone, Debug, PartialEq, Eq, BinaryHashRepr, FieldRepr, FromFieldRepr)]
            pub struct #name { #(#fields),* }
        });
    }
    for (name, variants) in &enum_definitions {
        let name = ident(name)?;
        let variants = variants
            .iter()
            .map(|variant| ident(variant))
            .collect::<Result<Vec<_>, _>>()?;
        let to_ordinal = variants
            .iter()
            .enumerate()
            .map(|(number, variant)| {
                let number = syn::LitInt::new(&number.to_string(), Span::call_site());
                quote!(Self::#variant => #number,)
            })
            .collect::<Vec<_>>();
        let from_ordinal = variants
            .iter()
            .enumerate()
            .map(|(number, variant)| {
                let number = syn::LitInt::new(&number.to_string(), Span::call_site());
                quote!(#number => Some(Self::#variant),)
            })
            .collect::<Vec<_>>();
        let max_ordinal = variants.len() - 1;
        let byte_length = ((usize::BITS - max_ordinal.leading_zeros()) as usize).div_ceil(8);
        let byte_length = syn::LitInt::new(&byte_length.to_string(), Span::call_site());
        struct_items.push(syn::parse_quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum #name { #(#variants),* }
        });
        struct_items.push(syn::parse_quote! {
            impl FieldRepr for #name {
                fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
                    let ordinal: u128 = match self { #(#to_ordinal)* };
                    ordinal.field_repr(writer);
                }
                fn field_size(&self) -> usize { 1 }
            }
        });
        struct_items.push(syn::parse_quote! {
            impl BinaryHashRepr for #name {
                fn binary_repr<W: MemWrite<u8>>(&self, writer: &mut W) {
                    let ordinal: u128 = match self { #(#to_ordinal)* };
                    writer.write(&ordinal.to_le_bytes()[..#byte_length]);
                }
                fn binary_len(&self) -> usize { #byte_length }
            }
        });
        struct_items.push(syn::parse_quote! {
            impl FromFieldRepr for #name {
                const FIELD_SIZE: usize = 1;
                fn from_field_repr(repr: &[Fr]) -> Option<Self> {
                    let ordinal = <u128 as FromFieldRepr>::from_field_repr(repr)?;
                    match ordinal { #(#from_ordinal)* _ => None }
                }
            }
        });
    }
    let types_module: Option<syn::Item> = if struct_items.is_empty() {
        None
    } else {
        Some(syn::parse_quote! {
            pub mod types {
                use midnight_compact_runtime as runtime;
                use runtime::{BinaryHashRepr, FieldRepr, Fr, FromFieldRepr, MemWrite};
                #(#struct_items)*
            }
        })
    };
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
        #types_module
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
