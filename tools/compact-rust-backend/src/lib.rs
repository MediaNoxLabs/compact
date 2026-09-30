//! Rust syntax construction from Compact's typed backend IR.

pub mod ir;
mod stateful;

const RUNTIME_ABI_VERSION: u32 = 2;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;

use ir::{
    Contract, Expr, LedgerFieldKind, PureCircuit, SCHEMA_VERSION, StructField, Type,
    WitnessDeclaration,
};
use proc_macro2::Span;
use quote::quote;

#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    SchemaVersion(u32),
    InvalidIdentifier(String),
    InvalidUnsignedMaximum(String),
    InvalidFieldLiteral(String),
    InvalidUnsignedLiteral {
        value: String,
        max: String,
    },
    DuplicateCircuit(String),
    DuplicateWitness(String),
    UnknownWitness(String),
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
    UnknownCircuit(String),
    ArgumentCount {
        circuit: String,
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        expected: Type,
        actual: Type,
    },
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
            Self::InvalidFieldLiteral(value) => {
                write!(
                    f,
                    "unsupported Field literal {value:?}; expected canonical u128"
                )
            }
            Self::InvalidUnsignedLiteral { value, max } => {
                write!(
                    f,
                    "Uint literal {value:?} does not fit declared maximum {max:?}"
                )
            }
            Self::DuplicateCircuit(name) => write!(f, "duplicate circuit {name:?}"),
            Self::DuplicateWitness(name) => write!(f, "duplicate witness {name:?}"),
            Self::UnknownWitness(name) => write!(f, "unknown witness {name:?}"),
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
            Self::UnknownCircuit(name) => write!(f, "unknown pure circuit {name:?}"),
            Self::ArgumentCount {
                circuit,
                expected,
                actual,
            } => write!(
                f,
                "circuit {circuit:?} takes {expected} arguments, received {actual}"
            ),
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

fn collect_expression_types(
    expr: &Expr,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), RenderError> {
    match expr {
        Expr::Tuple { elements } => {
            for element in elements {
                collect_expression_types(element, structs, enums)?;
            }
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums)?;
            collect_expression_types(then, structs, enums)?;
            collect_expression_types(otherwise, structs, enums)?;
        }
        Expr::Let { bindings, body } => {
            for binding in bindings {
                collect_named_types(&binding.ty, structs, enums)?;
                collect_expression_types(&binding.value, structs, enums)?;
            }
            collect_expression_types(body, structs, enums)?;
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums)?;
            }
        }
        Expr::UnsignedCast { value, .. } => collect_expression_types(value, structs, enums)?,
        Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => {
            collect_expression_types(left, structs, enums)?;
            collect_expression_types(right, structs, enums)?;
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right } => {
            collect_expression_types(left, structs, enums)?;
            collect_expression_types(right, structs, enums)?;
        }
        Expr::Unit
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::Parameter { .. } => {}
    }
    Ok(())
}

fn expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
) -> Result<(syn::Expr, Type), RenderError> {
    expression_with_calls(expr, parameters, &HashMap::new())
}

fn expression_with_calls(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::Unit => Ok((syn::parse_quote!(()), Type::Unit)),
        Expr::Boolean { value } => Ok((syn::parse_quote!(#value), Type::Boolean)),
        Expr::FieldLiteral { value } => {
            let parsed = value
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidFieldLiteral(value.clone()))?;
            if parsed.to_string() != *value {
                return Err(RenderError::InvalidFieldLiteral(value.clone()));
            }
            let value = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
            Ok((syn::parse_quote!(runtime::Field::from(#value)), Type::Field))
        }
        Expr::UnsignedLiteral { value, max } => {
            let parsed_max = max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(max.clone()))?;
            if parsed_max.to_string() != *max {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
            }
            let parsed_value =
                value
                    .parse::<u128>()
                    .map_err(|_| RenderError::InvalidUnsignedLiteral {
                        value: value.clone(),
                        max: max.clone(),
                    })?;
            if parsed_value.to_string() != *value || parsed_value > parsed_max {
                return Err(RenderError::InvalidUnsignedLiteral {
                    value: value.clone(),
                    max: max.clone(),
                });
            }
            let max_lit = syn::LitInt::new(max, Span::call_site());
            let value_lit = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
            Ok((
                syn::parse_quote!(runtime::BoundedUint::<#max_lit>::new(#value_lit).expect("Compact Uint literal fits its maximum")),
                Type::Unsigned { max: max.clone() },
            ))
        }
        Expr::Parameter { name } => {
            let (ty, rust_name) = parameters
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
            Ok((syn::parse_quote!(#rust_name), (*ty).clone()))
        }
        Expr::Tuple { elements } => {
            let (exprs, types): (Vec<_>, Vec<_>) = elements
                .iter()
                .map(|element| expression_with_calls(element, parameters, circuits))
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
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let (condition, condition_ty) = expression_with_calls(condition, parameters, circuits)?;
            if condition_ty != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: condition_ty,
                });
            }
            let (then, then_ty) = expression_with_calls(then, parameters, circuits)?;
            let (otherwise, otherwise_ty) = expression_with_calls(otherwise, parameters, circuits)?;
            if then_ty != otherwise_ty {
                return Err(RenderError::TypeMismatch {
                    expected: then_ty,
                    actual: otherwise_ty,
                });
            }
            Ok((
                syn::parse_quote!(if #condition { #then } else { #otherwise }),
                then_ty,
            ))
        }
        Expr::Let { bindings, body } => {
            let mut locals = parameters.clone();
            let mut statements = Vec::<syn::Stmt>::new();
            for (index, binding) in bindings.iter().enumerate() {
                ident(&binding.name)?;
                let (value, actual) = expression_with_calls(&binding.value, &locals, circuits)?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                let local_ty = rust_type(&binding.ty)?;
                let local_name =
                    syn::Ident::new(&format!("__compact_local_{index}"), Span::call_site());
                statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));
                locals.insert(binding.name.as_str(), (&binding.ty, local_name));
            }
            let (body, body_ty) = expression_with_calls(body, &locals, circuits)?;
            Ok((syn::parse_quote!({ #(#statements)* #body }), body_ty))
        }
        Expr::Call { name, arguments } => {
            let circuit = circuits
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
            if arguments.len() != circuit.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: circuit.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut rendered_arguments = Vec::with_capacity(arguments.len());
            for (argument, parameter) in arguments.iter().zip(&circuit.parameters) {
                let (value, actual) = expression_with_calls(argument, parameters, circuits)?;
                if actual != parameter.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: parameter.ty.clone(),
                        actual,
                    });
                }
                rendered_arguments.push(value);
            }
            let name = ident(name)?;
            Ok((
                syn::parse_quote!(crate::pure_circuits::#name(#(#rendered_arguments),*)?),
                circuit.result.clone(),
            ))
        }
        Expr::UnsignedCast { max, value } => {
            let target_max = max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(max.clone()))?;
            if target_max.to_string() != *max {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
            }
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            let Type::Unsigned { max: source_max } = actual else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual,
                });
            };
            let source_max = source_max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(source_max.clone()))?;
            let source_lit = syn::LitInt::new(&source_max.to_string(), Span::call_site());
            let target_lit = syn::LitInt::new(max, Span::call_site());
            Ok((
                syn::parse_quote!(runtime::cast_unsigned::<#source_lit, #target_lit>(#value)?),
                Type::Unsigned { max: max.clone() },
            ))
        }
        Expr::UnsignedAdd { max, left, right }
        | Expr::UnsignedSubtract { max, left, right }
        | Expr::UnsignedMultiply { max, left, right } => {
            let result_max = max
                .parse::<u128>()
                .map_err(|_| RenderError::InvalidUnsignedMaximum(max.clone()))?;
            if result_max.to_string() != *max {
                return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
            }
            let (left, left_type) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_type) = expression_with_calls(right, parameters, circuits)?;
            let Type::Unsigned { max: left_max } = left_type else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual: left_type,
                });
            };
            let Type::Unsigned { max: right_max } = right_type else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual: right_type,
                });
            };
            let left_max = syn::LitInt::new(&left_max, Span::call_site());
            let right_max = syn::LitInt::new(&right_max, Span::call_site());
            let result_max = syn::LitInt::new(max, Span::call_site());
            let operation: syn::Path = match expr {
                Expr::UnsignedAdd { .. } => syn::parse_quote!(runtime::add_unsigned),
                Expr::UnsignedSubtract { .. } => {
                    syn::parse_quote!(runtime::subtract_unsigned)
                }
                Expr::UnsignedMultiply { .. } => {
                    syn::parse_quote!(runtime::multiply_unsigned)
                }
                _ => unreachable!(),
            };
            Ok((
                syn::parse_quote!(#operation::<#left_max, #right_max, #result_max>(#left, #right)?),
                Type::Unsigned { max: max.clone() },
            ))
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right } => {
            let (left, left_type) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_type) = expression_with_calls(right, parameters, circuits)?;
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
            let op = match expr {
                Expr::Add { .. } => syn::BinOp::Add(syn::token::Plus::default()),
                Expr::Subtract { .. } => syn::BinOp::Sub(syn::token::Minus::default()),
                Expr::Multiply { .. } => syn::BinOp::Mul(syn::token::Star::default()),
                _ => unreachable!(),
            };
            Ok((
                syn::Expr::Binary(syn::ExprBinary {
                    attrs: vec![],
                    left: Box::new(left),
                    op,
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
        collect_expression_types(
            &circuit.body,
            &mut struct_definitions,
            &mut enum_definitions,
        )?;
    }
    for witness in &contract.witnesses {
        for parameter in &witness.parameters {
            collect_named_types(
                &parameter.ty,
                &mut struct_definitions,
                &mut enum_definitions,
            )?;
        }
        collect_named_types(
            &witness.result,
            &mut struct_definitions,
            &mut enum_definitions,
        )?;
    }
    for circuit in &contract.stateful_circuits {
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
        match &field.declaration {
            LedgerFieldKind::Cell { ty }
            | LedgerFieldKind::Set { ty }
            | LedgerFieldKind::List { ty } => {
                collect_named_types(ty, &mut struct_definitions, &mut enum_definitions)?;
            }
            LedgerFieldKind::Map { key, value } => {
                collect_named_types(key, &mut struct_definitions, &mut enum_definitions)?;
                collect_named_types(value, &mut struct_definitions, &mut enum_definitions)?;
            }
            LedgerFieldKind::Counter => {}
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
    let callable_circuits: HashMap<&str, &PureCircuit> = contract
        .circuits
        .iter()
        .map(|circuit| (circuit.name.as_str(), circuit))
        .collect();
    let mut witness_declarations = HashMap::<&str, &WitnessDeclaration>::new();
    let mut witness_methods = Vec::<syn::TraitItemFn>::new();
    for witness in &contract.witnesses {
        let name = ident(&witness.name)?;
        if witness_declarations
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
        witness_methods.push(syn::parse_quote! {
            fn #name(
                &self,
                context: runtime::context::WitnessContext<'_, Private>,
                #(#args),*
            ) -> (Private, #result);
        });
    }
    let mut items = Vec::new();
    for circuit in &contract.circuits {
        let name = ident(&circuit.name)?;
        if !names.insert(circuit.name.as_str()) {
            return Err(RenderError::DuplicateCircuit(circuit.name.clone()));
        }
        let mut parameters = HashMap::new();
        let mut args = Vec::<syn::FnArg>::new();
        for parameter in &circuit.parameters {
            let arg_name = ident(&parameter.name)?;
            if parameters
                .insert(parameter.name.as_str(), (&parameter.ty, arg_name.clone()))
                .is_some()
            {
                return Err(RenderError::DuplicateParameter(parameter.name.clone()));
            }
            let arg_ty = rust_type(&parameter.ty)?;
            let arg: syn::FnArg = syn::parse_quote!(#arg_name: #arg_ty);
            args.push(arg);
        }
        let (body, actual) = expression_with_calls(&circuit.body, &parameters, &callable_circuits)?;
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
        if !names.insert(circuit.name.as_str()) {
            return Err(RenderError::DuplicateCircuit(circuit.name.clone()));
        }
        stateful_items.push(stateful::render_stateful_circuit(
            circuit,
            &ledger_fields,
            &witness_declarations,
        )?);
    }

    let constructor_fields = ordered_fields.iter().map(|field| match &field.declaration {
        LedgerFieldKind::Counter => Ok(syn::parse_quote!(runtime::ledger::constructor_counter())),
        LedgerFieldKind::Set { .. } => Ok(syn::parse_quote!(runtime::ledger::constructor_set())),
        LedgerFieldKind::List { .. } => Ok(syn::parse_quote!(runtime::ledger::constructor_list())),
        LedgerFieldKind::Map { .. } => Ok(syn::parse_quote!(runtime::ledger::constructor_map())),
        LedgerFieldKind::Cell { ty } => {
            if !matches!(ty, Type::Boolean | Type::Field | Type::Unsigned { .. } | Type::Bytes { .. } | Type::Struct { .. } | Type::Enum { .. }) {
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
            #[derive(Clone, Debug, Default, PartialEq, Eq, CompactCellValue, BinaryHashRepr, FieldRepr, FromFieldRepr)]
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
            #[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue)]
            pub enum #name { #(#variants),* }
        });
        let first = &variants[0];
        struct_items.push(syn::parse_quote! {
            impl Default for #name {
                fn default() -> Self { Self::#first }
            }
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
                use runtime::{BinaryHashRepr, CompactCellValue, FieldRepr, Fr, FromFieldRepr, MemWrite};
                #(#struct_items)*
            }
        })
    };
    let ledger_module: Option<syn::Item> = if contract.ledger_fields.is_empty()
        && contract.stateful_circuits.is_empty()
        && contract.witnesses.is_empty()
    {
        None
    } else {
        Some(syn::parse_quote! {
            pub mod ledger_contract {
                use midnight_compact_runtime as runtime;
                const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
                pub trait Witnesses<Private> {
                    #(#witness_methods)*
                }
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
