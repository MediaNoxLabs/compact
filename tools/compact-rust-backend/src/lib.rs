//! Rust syntax construction from Compact's typed backend IR.

mod capabilities;
mod coin_shapes;
pub mod ir;
pub use capabilities::{
    RUST_CAPABILITY_SCHEMA_VERSION, RecordingStatus, RustCapabilityReport, RustCircuitCapability,
};
mod native_frame;
mod recorded;
pub use recorded::{RecordingGap, RecordingGapCode};
mod stateful;
mod type_declarations;
mod witness;

const RUNTIME_ABI_VERSION: u32 = 50;

const GENERATED_HEADER: &str = r#"// This file is part of Compact.
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

"#;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;

use ir::{
    ComparisonOperator, ConstructorStep, Contract, CounterAmount, Expr, LedgerFieldKind,
    PureCircuit, SCHEMA_VERSION, StateAction, StatefulCircuit, StructField, Type,
};
use proc_macro2::Span;
use quote::quote;
use serde_json::Value;
use syn::visit_mut::{self, VisitMut};

pub struct RenderedContract {
    pub source: String,
    pub capabilities: RustCapabilityReport,
}

pub(crate) fn ledger_path_expr(field: &ir::LedgerField) -> syn::Expr {
    let path = field.physical_path();
    if path.len() == 1 {
        let index = syn::LitInt::new(&path[0].to_string(), Span::call_site());
        syn::parse_quote!(#index)
    } else {
        let indices = path
            .iter()
            .map(|part| syn::LitInt::new(&part.to_string(), Span::call_site()))
            .collect::<Vec<_>>();
        syn::parse_quote!(&[#(#indices),*])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    Located {
        location: ir::SourceLocation,
        error: Box<RenderError>,
    },
    SchemaVersion(u32),
    ProofApplicability(String),
    InvalidIdentifier(String),
    InvalidUnsignedMaximum(String),
    InvalidFieldLiteral(String),
    InvalidUnsignedLiteral {
        value: String,
        max: String,
    },
    DuplicateCircuit(String),
    ConflictingCircuitIdentifier {
        first: String,
        second: String,
        rust_identifier: String,
    },
    DuplicateWitness(String),
    UnknownWitness(String),
    DuplicateParameter(String),
    DuplicateLedgerField(String),
    InvalidLedgerIndex(u8),
    InvalidLedgerPath(Vec<u8>),
    InvalidMerkleTreeDepth(u8),
    UnsupportedLedgerPath(Vec<u8>),
    UnknownLedgerField(String),
    InvalidConstructorInitializer(String),
    UnsupportedLedgerCellType(Type),
    UnsupportedLedgerValueType(Type),
    ConflictingTypeAlias(String),
    ConflictingStruct(String),
    DuplicateStructField(String),
    InvalidStructField(String),
    InvalidTupleIndex(usize),
    ExpectedTuple(Type),
    ConflictingEnum(String),
    EmptyEnum(String),
    DuplicateEnumVariant(String),
    InvalidEnumVariant(String),
    UnknownParameter(String),
    UnknownCircuit(String),
    UnsupportedStatefulCall(String),
    ArgumentCount {
        circuit: String,
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        expected: Type,
        actual: Type,
    },
    ExpectedUnsigned(Type),
    EffectfulExpression,
    MalformedReturnPlan,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Located { location, error } => write!(
                f,
                "{} line {} char {}: {}",
                location.file, location.line, location.column, error
            ),
            Self::SchemaVersion(version) => {
                write!(f, "unsupported Rust backend IR schema {version}")
            }
            Self::MalformedReturnPlan => write!(
                f,
                "effectful return plan must own the entire ordered circuit body"
            ),
            Self::ProofApplicability(message) => {
                write!(f, "invalid proof applicability: {message}")
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
                    "invalid Field literal {value:?}; expected a canonical decimal ledger8 field element"
                )
            }
            Self::InvalidUnsignedLiteral { value, max } => {
                write!(
                    f,
                    "Uint literal {value:?} does not fit declared maximum {max:?}"
                )
            }
            Self::DuplicateCircuit(name) => write!(f, "duplicate circuit {name:?}"),
            Self::ConflictingCircuitIdentifier {
                first,
                second,
                rust_identifier,
            } => write!(
                f,
                "circuits {first:?} and {second:?} both emit Rust identifier {rust_identifier:?}"
            ),
            Self::DuplicateWitness(name) => write!(f, "duplicate witness {name:?}"),
            Self::UnknownWitness(name) => write!(f, "unknown witness {name:?}"),
            Self::DuplicateParameter(name) => write!(f, "duplicate parameter {name:?}"),
            Self::DuplicateLedgerField(name) => write!(f, "duplicate ledger field {name:?}"),
            Self::InvalidLedgerIndex(index) => {
                write!(f, "invalid or noncontiguous ledger field index {index}")
            }
            Self::InvalidLedgerPath(path) => write!(f, "invalid ledger field path {path:?}"),
            Self::InvalidMerkleTreeDepth(depth) => {
                write!(f, "invalid Merkle tree depth {depth}; expected 2..=32")
            }
            Self::UnsupportedLedgerPath(path) => {
                write!(f, "unsupported ledger field kind at chunked path {path:?}")
            }
            Self::UnknownLedgerField(name) => write!(f, "unknown ledger field {name:?}"),
            Self::InvalidConstructorInitializer(name) => write!(
                f,
                "constructor can initialize root Cells from parameters only: {name:?}"
            ),
            Self::UnsupportedLedgerCellType(ty) => write!(f, "unsupported ledger Cell type {ty:?}"),
            Self::UnsupportedLedgerValueType(ty) => {
                write!(
                    f,
                    "ledger collection type cannot be used as a Cell value: {ty:?}"
                )
            }
            Self::ConflictingTypeAlias(name) => {
                write!(f, "conflicting exported type alias {name:?}")
            }
            Self::ConflictingStruct(name) => {
                write!(f, "conflicting definitions for struct {name:?}")
            }
            Self::DuplicateStructField(name) => write!(f, "duplicate struct field {name:?}"),
            Self::InvalidStructField(name) => write!(f, "invalid struct field {name:?}"),
            Self::InvalidTupleIndex(index) => write!(f, "invalid tuple index {index}"),
            Self::ExpectedTuple(ty) => write!(f, "expected tuple value, got {ty:?}"),
            Self::ConflictingEnum(name) => write!(f, "conflicting definitions for enum {name:?}"),
            Self::EmptyEnum(name) => write!(f, "enum {name:?} has no variants"),
            Self::DuplicateEnumVariant(name) => write!(f, "duplicate enum variant {name:?}"),
            Self::InvalidEnumVariant(name) => write!(f, "invalid enum variant {name:?}"),
            Self::UnknownParameter(name) => write!(f, "unknown parameter {name:?}"),
            Self::UnknownCircuit(name) => write!(f, "unknown circuit {name:?}"),
            Self::UnsupportedStatefulCall(name) => {
                write!(f, "unsupported stateful circuit call {name:?}")
            }
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
            Self::ExpectedUnsigned(actual) => {
                write!(f, "expression has type {actual:?}, expected Compact Uint")
            }
            Self::EffectfulExpression => {
                write!(f, "stateful expression requires stateful evaluation")
            }
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Located { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl RenderError {
    pub(crate) fn at(self, location: Option<&ir::SourceLocation>) -> Self {
        if matches!(&self, Self::Located { .. }) {
            return self;
        }
        match location {
            Some(location) => Self::Located {
                location: location.clone(),
                error: Box::new(self),
            },
            None => self,
        }
    }
}

pub(crate) fn located<T>(
    source: Option<&ir::SourceLocation>,
    work: impl FnOnce() -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    work().map_err(|error| error.at(source))
}

fn ident(name: &str) -> Result<syn::Ident, RenderError> {
    let rust_name = name.replace('$', "_");
    syn::parse_str::<syn::Ident>(&rust_name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{rust_name}")))
        .map_err(|_| RenderError::InvalidIdentifier(name.to_owned()))
}

fn validate_circuit_function_namespace<'a>(
    declarations: impl IntoIterator<Item = (&'a str, Option<&'a ir::SourceLocation>)>,
) -> Result<(), RenderError> {
    let mut emitted = HashMap::<String, &'a str>::new();
    for (name, source) in declarations {
        located(source, || {
            // `foo` and `r#foo` bind the same Rust name. Compare the
            // semantic identifier after `ident` has normalized Compact `$`
            // and escaped Rust keywords.
            let rendered_identifier = ident(name)?.to_string();
            let rust_identifier = rendered_identifier
                .strip_prefix("r#")
                .unwrap_or(&rendered_identifier)
                .to_owned();
            if let Some(first) = emitted.get(&rust_identifier) {
                return Err(RenderError::ConflictingCircuitIdentifier {
                    first: (*first).to_owned(),
                    second: name.to_owned(),
                    rust_identifier,
                });
            }
            emitted.insert(rust_identifier, name);
            Ok(())
        })?;
    }
    Ok(())
}

/// Name public wrapper arguments after their Compact parameters. Wrapper
/// locals and names that normalize to the same Rust identifier need stable
/// positional fallbacks; the underlying circuit still receives arguments in
/// declaration order.
pub(crate) fn public_parameter_idents(parameters: &[ir::Parameter]) -> Vec<syn::Ident> {
    let mut used = HashSet::<String>::new();
    let reserved = [
        "self",
        "Self",
        "super",
        "crate",
        "context",
        "witnesses",
        "observed",
        "private_state",
        "input",
        "recorded",
    ];
    used.extend(reserved.into_iter().map(str::to_owned));
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let candidate = ident(&parameter.name).ok().filter(|name| {
                let spelling = name.to_string();
                spelling != "_"
                    && !used.contains(&spelling)
                    && !used.contains(spelling.strip_prefix("r#").unwrap_or(&spelling))
            });
            let name = candidate.unwrap_or_else(|| {
                let base = format!("__compact_param_{index}");
                let mut fallback = base.clone();
                let mut suffix = 1;
                while used.contains(&fallback) {
                    fallback = format!("{base}_{suffix}");
                    suffix += 1;
                }
                syn::Ident::new(&fallback, Span::call_site())
            });
            used.insert(name.to_string());
            name
        })
        .collect()
}

#[cfg(test)]
mod public_parameter_name_tests {
    use super::{ir, public_parameter_idents};

    #[test]
    fn preserves_source_names_and_allocates_unique_wrapper_fallbacks() {
        let names = [
            "amount",
            "type",
            "a$b",
            "a_b",
            "context",
            "input",
            "__compact_param_6",
            "self",
            "unused_8",
        ];
        let parameters = names
            .into_iter()
            .map(|name| ir::Parameter {
                name: name.into(),
                ty: ir::Type::Field,
            })
            .collect::<Vec<_>>();
        let actual = public_parameter_idents(&parameters)
            .into_iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [
                "amount",
                "r#type",
                "a_b",
                "__compact_param_3",
                "__compact_param_4",
                "__compact_param_5",
                "__compact_param_6",
                "__compact_param_7",
                "unused_8",
            ]
        );
    }
}

fn field_literal_bytes(value: &str) -> Result<[u8; 32], RenderError> {
    let invalid = || RenderError::InvalidFieldLiteral(value.to_owned());
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let mut bytes = [0_u8; 32];
    for digit in value.bytes() {
        let mut carry = u16::from(digit - b'0');
        for byte in &mut bytes {
            let expanded = u16::from(*byte) * 10 + carry;
            *byte = expanded as u8;
            carry = expanded >> 8;
        }
        if carry != 0 {
            return Err(invalid());
        }
    }
    if midnight_transient_crypto::curve::Fr::from_le_bytes(&bytes).is_none() {
        return Err(invalid());
    }
    Ok(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsignedMaximum {
    Small(u128),
    Wide { high: u128, low: u128 },
}

pub(crate) fn unsigned_maximum(value: &str) -> Result<UnsignedMaximum, RenderError> {
    let bytes = field_literal_bytes(value)
        .map_err(|_| RenderError::InvalidUnsignedMaximum(value.to_owned()))?;
    if bytes[31] != 0 {
        return Err(RenderError::InvalidUnsignedMaximum(value.to_owned()));
    }
    let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
    let high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
    if high == 0 {
        Ok(UnsignedMaximum::Small(low))
    } else {
        Ok(UnsignedMaximum::Wide { high, low })
    }
}

fn wide_uint_type(high: u128, low: u128) -> syn::Type {
    let high = syn::LitInt::new(&format!("{high}u128"), Span::call_site());
    let low = syn::LitInt::new(&format!("{low}u128"), Span::call_site());
    syn::parse_quote!(runtime::WideUint<#high, #low>)
}

pub(crate) fn field_to_bytes_32_syntax(value: syn::Expr) -> syn::Expr {
    // Compact's Field-to-Bytes cast is zero-padded little endian. The pinned
    // midnight-zk Field has a canonical 32-byte representation.
    syn::parse_quote!(runtime::FixedBytes::<32>::new(
        (#value).as_le_bytes().try_into().expect("canonical Field is 32 bytes")
    ))
}

pub(crate) fn unsigned_cast_syntax(
    value: syn::Expr,
    source_max: &str,
    target_max: &str,
) -> Result<syn::Expr, RenderError> {
    let source = unsigned_maximum(source_max)?;
    let target = unsigned_maximum(target_max)?;
    Ok(match (source, target) {
        (UnsignedMaximum::Small(source), UnsignedMaximum::Small(target)) => {
            let source = syn::LitInt::new(&source.to_string(), Span::call_site());
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            syn::parse_quote!(runtime::cast_unsigned::<#source, #target>(#value)?)
        }
        (UnsignedMaximum::Small(_), UnsignedMaximum::Wide { high, low }) => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes(&(#value).value().to_le_bytes())?)
        }
        (UnsignedMaximum::Wide { high, low }, UnsignedMaximum::Small(target)) => {
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            let high = syn::LitInt::new(&high.to_string(), Span::call_site());
            let low = syn::LitInt::new(&low.to_string(), Span::call_site());
            syn::parse_quote!(runtime::narrow_wide_uint::<#target, #high, #low>(#value)?)
        }
        (UnsignedMaximum::Wide { .. }, UnsignedMaximum::Wide { high, low }) => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes((#value).as_le_bytes())?)
        }
    })
}

/// Both expression emitters share admission and exact runtime arithmetic selection.
pub(crate) fn unsigned_arithmetic_syntax(
    operation: &Expr,
    left: syn::Expr,
    right: syn::Expr,
    left_max: &str,
    right_max: &str,
    result_max: &str,
) -> Result<syn::Expr, RenderError> {
    let left_bound = unsigned_maximum(left_max)?;
    let right_bound = unsigned_maximum(right_max)?;
    let result_bound = unsigned_maximum(result_max)?;
    if matches!(operation, Expr::UnsignedAdd { .. })
        && let UnsignedMaximum::Wide { high, low } = result_bound
    {
        let high = syn::LitInt::new(&format!("{high}u128"), Span::call_site());
        let low = syn::LitInt::new(&format!("{low}u128"), Span::call_site());
        return Ok(syn::parse_quote!(runtime::add_wide_unsigned::<#high,#low,_,_>(#left,#right)?));
    }
    for (bound, text) in [
        (left_bound, left_max),
        (right_bound, right_max),
        (result_bound, result_max),
    ] {
        if matches!(bound, UnsignedMaximum::Wide { .. }) {
            return Err(RenderError::InvalidUnsignedMaximum(text.to_owned()));
        }
    }
    let left_max = syn::LitInt::new(left_max, Span::call_site());
    let right_max = syn::LitInt::new(right_max, Span::call_site());
    let result_max = syn::LitInt::new(result_max, Span::call_site());
    let helper: syn::Path = match operation {
        Expr::UnsignedAdd { .. } => syn::parse_quote!(runtime::add_unsigned),
        Expr::UnsignedSubtract { .. } => syn::parse_quote!(runtime::subtract_unsigned),
        Expr::UnsignedMultiply { .. } => syn::parse_quote!(runtime::multiply_unsigned),
        _ => unreachable!("unsigned arithmetic caller"),
    };
    Ok(syn::parse_quote!(#helper::<#left_max,#right_max,#result_max>(#left,#right)?))
}

fn rust_type(ty: &Type) -> Result<syn::Type, RenderError> {
    Ok(match ty {
        Type::Unit => syn::parse_quote!(()),
        Type::Boolean => syn::parse_quote!(bool),
        Type::Field => syn::parse_quote!(runtime::Field),
        Type::JubjubPoint => syn::parse_quote!(runtime::JubjubPoint),
        Type::OpaqueString => syn::parse_quote!(runtime::OpaqueString),
        Type::OpaqueBytes => syn::parse_quote!(runtime::OpaqueBytes),
        Type::Bytes { length } => {
            let length = syn::LitInt::new(&length.to_string(), Span::call_site());
            syn::parse_quote!(runtime::FixedBytes<#length>)
        }
        Type::Struct { name, .. } | Type::Enum { name, .. } => {
            let name = ident(name)?;
            syn::parse_quote!(crate::types::#name)
        }
        Type::Unsigned { max } => match unsigned_maximum(max)? {
            UnsignedMaximum::Small(_) => {
                let max = syn::LitInt::new(max, Span::call_site());
                syn::parse_quote!(runtime::BoundedUint<#max>)
            }
            UnsignedMaximum::Wide { high, low } => wide_uint_type(high, low),
        },
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
        Type::LedgerMap { .. } => return Err(RenderError::UnsupportedLedgerValueType(ty.clone())),
    })
}

fn map_slot_value_type(value: &Type) -> Result<Option<syn::Type>, RenderError> {
    if let Type::LedgerMap { key, value } = value {
        let Some((key, value)) = map_slot_types(key, value)? else {
            return Ok(None);
        };
        return Ok(Some(
            syn::parse_quote!(runtime::slots::MapNode<#key, #value>),
        ));
    }
    match rust_type(value) {
        Ok(ty) => Ok(Some(ty)),
        Err(RenderError::UnsupportedLedgerValueType(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn map_slot_types(
    key: &Type,
    value: &Type,
) -> Result<Option<(syn::Type, syn::Type)>, RenderError> {
    let key = match rust_type(key) {
        Ok(ty) => ty,
        Err(RenderError::UnsupportedLedgerValueType(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let value = match map_slot_value_type(value)? {
        Some(ty) => ty,
        None => return Ok(None),
    };
    Ok(Some((key, value)))
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
        Type::LedgerMap { key, value } => {
            collect_named_types(key, structs, enums)?;
            collect_named_types(value, structs, enums)?;
        }
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
        Expr::Default { ty } | Expr::EnumVariant { ty, .. } => {
            collect_named_types(ty, structs, enums)?
        }
        Expr::StructField { value, .. } | Expr::TupleIndex { value, .. } => {
            collect_expression_types(value, structs, enums)?
        }
        Expr::StructLiteral { ty, fields } => {
            collect_named_types(ty, structs, enums)?;
            for field in fields {
                collect_expression_types(field, structs, enums)?;
            }
        }
        Expr::Tuple { elements } => {
            for element in elements {
                collect_expression_types(element, structs, enums)?;
            }
        }
        Expr::Vector { element, elements } => {
            collect_named_types(element, structs, enums)?;
            for value in elements {
                collect_expression_types(value, structs, enums)?;
            }
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            ..
        } => {
            collect_named_types(&parameter.ty, structs, enums)?;
            collect_named_types(result, structs, enums)?;
            collect_expression_types(source, structs, enums)?;
            collect_expression_types(body, structs, enums)?;
        }
        Expr::VectorFoldCall {
            initial,
            source,
            accumulator,
            element,
            ..
        } => {
            collect_named_types(accumulator, structs, enums)?;
            collect_named_types(element, structs, enums)?;
            collect_expression_types(initial, structs, enums)?;
            collect_expression_types(source, structs, enums)?;
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
        Expr::Sequence { steps, value } => {
            for step in steps {
                collect_expression_types(step, structs, enums)?;
            }
            collect_expression_types(value, structs, enums)?;
        }
        Expr::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums)?;
        }
        Expr::SetMember { value, .. }
        | Expr::CounterLessThan {
            threshold: value, ..
        } => collect_expression_types(value, structs, enums)?,
        Expr::MapMember { key, .. } | Expr::MapLookup { key, .. } => {
            collect_expression_types(key, structs, enums)?
        }
        Expr::MerkleCheckRoot { root, .. } | Expr::HistoricMerkleCheckRoot { root, .. } => {
            collect_expression_types(root, structs, enums)?
        }
        Expr::Call { arguments, .. } | Expr::WitnessCall { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums)?;
            }
        }
        Expr::TransientHash { value }
        | Expr::PersistentHash { value }
        | Expr::Keccak256 { value }
        | Expr::DegradeToTransient { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::HashToCurve { value }
        | Expr::JubjubPointX { value }
        | Expr::JubjubPointY { value }
        | Expr::EcNeg { value }
        | Expr::JubjubScalarFromNative { value } => {
            collect_expression_types(value, structs, enums)?
        }
        Expr::EcMulGenerator { scalar } => collect_expression_types(scalar, structs, enums)?,
        Expr::EcMul { point, scalar } => {
            collect_expression_types(point, structs, enums)?;
            collect_expression_types(scalar, structs, enums)?;
        }
        Expr::ConstructJubjubPoint { x, y } => {
            collect_expression_types(x, structs, enums)?;
            collect_expression_types(y, structs, enums)?;
        }
        Expr::TransientCommit { value, opening } | Expr::PersistentCommit { value, opening } => {
            collect_expression_types(value, structs, enums)?;
            collect_expression_types(opening, structs, enums)?;
        }
        Expr::UnsignedCast { value, .. }
        | Expr::FieldCast { value }
        | Expr::FieldToBytes32 { value } => collect_expression_types(value, structs, enums)?,
        Expr::Coerce { value, ty } => {
            collect_expression_types(value, structs, enums)?;
            collect_named_types(ty, structs, enums)?;
        }
        Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => {
            collect_expression_types(left, structs, enums)?;
            collect_expression_types(right, structs, enums)?;
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::Equal { left, right }
        | Expr::NotEqual { left, right }
        | Expr::Compare { left, right, .. }
        | Expr::EcAdd { left, right } => {
            collect_expression_types(left, structs, enums)?;
            collect_expression_types(right, structs, enums)?;
        }
        Expr::Unit
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::Parameter { .. } => {}
        Expr::KernelClaim { value, .. } => collect_expression_types(value, structs, enums)?,
        Expr::KernelMintShielded { domain, amount } => {
            collect_expression_types(domain, structs, enums)?;
            collect_expression_types(amount, structs, enums)?;
        }
        Expr::CreateZswapInput { coin } => collect_expression_types(coin, structs, enums)?,
        Expr::CreateZswapOutput { coin, recipient } => {
            collect_expression_types(coin, structs, enums)?;
            collect_expression_types(recipient, structs, enums)?;
        }
        Expr::NativeWitnessCall { builtin } => {
            collect_named_types(&builtin.result_type(), structs, enums)?;
        }
        Expr::KernelSelf { ty } | Expr::ListHead { ty, .. } => {
            collect_named_types(ty, structs, enums)?
        }
        Expr::SetSize { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. } => {}
    }
    Ok(())
}

fn collect_action_types(
    action: &StateAction,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), RenderError> {
    match action {
        StateAction::Sequence { actions } => {
            for action in actions {
                collect_action_types(action, structs, enums)?;
            }
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums)?;
            collect_action_types(then, structs, enums)?;
            collect_action_types(otherwise, structs, enums)?;
        }
        StateAction::PureCall { arguments, .. } | StateAction::CircuitCall { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums)?;
            }
        }
        StateAction::Expression { value } => {
            collect_expression_types(value, structs, enums)?;
        }
        StateAction::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums)?;
        }
        StateAction::Let { bindings, action } => {
            for binding in bindings {
                collect_named_types(&binding.ty, structs, enums)?;
                collect_expression_types(&binding.value, structs, enums)?;
            }
            collect_action_types(action, structs, enums)?;
        }
        StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. } => {
            collect_expression_types(value, structs, enums)?;
        }
        StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        } => {
            collect_expression_types(coin, structs, enums)?;
            collect_expression_types(recipient, structs, enums)?;
        }
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => {
            collect_expression_types(value, structs, enums)?;
            collect_expression_types(position, structs, enums)?;
        }
        StateAction::MapInsert { key, value, .. } => {
            collect_expression_types(key, structs, enums)?;
            collect_expression_types(value, structs, enums)?;
        }
        StateAction::MapInsertDefault { key, .. } | StateAction::MapRemove { key, .. } => {
            collect_expression_types(key, structs, enums)?;
        }
        StateAction::MerkleInsertIndexDefault { position, .. }
        | StateAction::HistoricMerkleInsertIndexDefault { position, .. } => {
            collect_expression_types(position, structs, enums)?;
        }
        StateAction::NativeWitnessCall { .. }
        | StateAction::CounterIncrement { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapReset { .. }
        | StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. } => {}
    }
    Ok(())
}

fn collect_return_plan_types(
    plan: &ir::ReturnPlan,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), RenderError> {
    match plan {
        ir::ReturnPlan::Value { value } => collect_expression_types(value, structs, enums),
        ir::ReturnPlan::Sequence { actions, result } => {
            for action in actions {
                collect_action_types(action, structs, enums)?;
            }
            collect_return_plan_types(result, structs, enums)
        }
        ir::ReturnPlan::Let { bindings, result } => {
            for binding in bindings {
                collect_named_types(&binding.ty, structs, enums)?;
                collect_expression_types(&binding.value, structs, enums)?;
            }
            collect_return_plan_types(result, structs, enums)
        }
        ir::ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums)?;
            collect_return_plan_types(then, structs, enums)?;
            collect_return_plan_types(otherwise, structs, enums)
        }
    }
}

fn coerce_aggregate_fields(
    sources: &[Type],
    targets: &[Type],
    depth: usize,
) -> Result<(Vec<syn::Ident>, Vec<syn::Expr>), RenderError> {
    debug_assert_eq!(sources.len(), targets.len());
    let items = (0..sources.len())
        .map(|index| {
            syn::Ident::new(
                &format!("__compact_cast_item_{depth}_{index}"),
                Span::call_site(),
            )
        })
        .collect::<Vec<_>>();
    let mapped = items
        .iter()
        .zip(sources.iter().zip(targets))
        .map(|(item, (source_ty, target_ty))| {
            coerce_expression(syn::parse_quote!(#item), source_ty, target_ty, depth + 1)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((items, mapped))
}

pub(crate) fn coerce_expression(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<syn::Expr, RenderError> {
    if actual == target {
        return Ok(value);
    }
    match (actual, target) {
        (Type::Unsigned { max }, Type::Field) => match unsigned_maximum(max)? {
            UnsignedMaximum::Small(_) => {
                Ok(syn::parse_quote!(runtime::Field::from((#value).value())))
            }
            UnsignedMaximum::Wide { .. } => Ok(syn::parse_quote!((#value).as_field())),
        },
        (Type::Unsigned { max: source_max }, Type::Unsigned { max: target_max }) => {
            let bounds = |value: UnsignedMaximum| match value {
                UnsignedMaximum::Small(low) => (0, low),
                UnsignedMaximum::Wide { high, low } => (high, low),
            };
            if bounds(unsigned_maximum(source_max)?) > bounds(unsigned_maximum(target_max)?) {
                return Err(RenderError::TypeMismatch {
                    expected: target.clone(),
                    actual: actual.clone(),
                });
            }
            unsigned_cast_syntax(value, source_max, target_max)
        }
        (
            Type::Vector {
                element: source_element,
                length: source_length,
            },
            Type::Vector {
                element: target_element,
                length: target_length,
            },
        ) if source_length == target_length => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let item = syn::Ident::new(&format!("__compact_cast_item_{depth}"), Span::call_site());
            let mapped = coerce_expression(
                syn::parse_quote!(#item),
                source_element,
                target_element,
                depth + 1,
            )?;
            Ok(syn::parse_quote!({
                let #source = #value;
                runtime::FixedVector::new(#source.into_array().map(|#item| #mapped))
            }))
        }
        (Type::Tuple { elements: sources }, Type::Tuple { elements: targets })
            if sources.len() == targets.len() && !sources.is_empty() =>
        {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let (items, mapped) = coerce_aggregate_fields(sources, targets, depth)?;
            Ok(syn::parse_quote!({
                let #source = #value;
                let (#(#items),*,) = #source;
                (#(#mapped),*,)
            }))
        }
        (
            Type::Tuple { elements: sources },
            Type::Vector {
                element: target_element,
                length,
            },
        ) if sources.len() == *length && !sources.is_empty() => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let targets = vec![*target_element.clone(); sources.len()];
            let (items, mapped) = coerce_aggregate_fields(sources, &targets, depth)?;
            Ok(syn::parse_quote!({
                let #source = #value;
                let (#(#items),*,) = #source;
                runtime::FixedVector::new([#(#mapped),*])
            }))
        }
        (
            Type::Vector {
                element: source_element,
                length,
            },
            Type::Tuple { elements: targets },
        ) if *length == targets.len() && !targets.is_empty() => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let sources = vec![*source_element.clone(); targets.len()];
            let (items, mapped) = coerce_aggregate_fields(&sources, targets, depth)?;
            Ok(syn::parse_quote!({
                let #source = #value;
                let [#(#items),*] = #source.into_array();
                (#(#mapped),*,)
            }))
        }
        _ => Err(RenderError::TypeMismatch {
            expected: target.clone(),
            actual: actual.clone(),
        }),
    }
}

pub(crate) fn copy_type(ty: &Type) -> bool {
    match ty {
        Type::Struct { .. }
        | Type::Vector { .. }
        | Type::LedgerMap { .. }
        | Type::OpaqueString
        | Type::OpaqueBytes => false,
        Type::Tuple { elements } => elements.iter().all(copy_type),
        Type::Unit
        | Type::Boolean
        | Type::Field
        | Type::JubjubPoint
        | Type::Bytes { .. }
        | Type::Enum { .. }
        | Type::Unsigned { .. } => true,
    }
}

/// Keep an owned value available at an emission boundary without cloning Copy types.
pub(crate) fn retained_value(value: syn::Expr, ty: &Type) -> syn::Expr {
    if copy_type(ty) {
        value
    } else {
        syn::parse_quote!((#value).clone())
    }
}

/// A Compact circuit's positional signature is part of the generated Rust API.
/// Scope the Clippy exception to functions that cross its seven-input limit.
struct CompactSignatureLint;

impl CompactSignatureLint {
    fn mark(attrs: &mut Vec<syn::Attribute>, signature: &syn::Signature) {
        if signature.inputs.len() > 7 {
            attrs.push(syn::parse_quote!(
                #[allow(
                    clippy::too_many_arguments,
                    reason = "preserves the declared Compact circuit signature"
                )]
            ));
        }
    }
}

impl VisitMut for CompactSignatureLint {
    fn visit_item_fn_mut(&mut self, item: &mut syn::ItemFn) {
        Self::mark(&mut item.attrs, &item.sig);
        visit_mut::visit_item_fn_mut(self, item);
    }

    fn visit_impl_item_fn_mut(&mut self, item: &mut syn::ImplItemFn) {
        Self::mark(&mut item.attrs, &item.sig);
        visit_mut::visit_impl_item_fn_mut(self, item);
    }

    fn visit_trait_item_fn_mut(&mut self, item: &mut syn::TraitItemFn) {
        Self::mark(&mut item.attrs, &item.sig);
        visit_mut::visit_trait_item_fn_mut(self, item);
    }
}

/// Reduce only opposite, literal Boolean arms after pure and stateful AST lowering.
/// The condition still runs once, including any nested fallible reads.
struct CompactBooleanLiterals;

impl CompactBooleanLiterals {
    fn conditional(expression: &syn::Expr) -> Option<&syn::ExprIf> {
        match expression {
            syn::Expr::If(conditional) => Some(conditional),
            syn::Expr::Paren(paren) => Self::conditional(&paren.expr),
            syn::Expr::Group(group) => Self::conditional(&group.expr),
            _ => None,
        }
    }

    fn branch_literal(block: &syn::Block) -> Option<bool> {
        let [syn::Stmt::Expr(syn::Expr::Lit(expression), None)] = block.stmts.as_slice() else {
            return None;
        };
        let syn::Lit::Bool(value) = &expression.lit else {
            return None;
        };
        Some(value.value)
    }

    fn reduced_if(conditional: &syn::ExprIf, negated: bool) -> Option<syn::Expr> {
        if !conditional.attrs.is_empty() || matches!(&*conditional.cond, syn::Expr::Let(_)) {
            return None;
        }
        let then_value = Self::branch_literal(&conditional.then_branch)?;
        let (_, otherwise) = conditional.else_branch.as_ref()?;
        let syn::Expr::Block(otherwise) = &**otherwise else {
            return None;
        };
        let otherwise_value = Self::branch_literal(&otherwise.block)?;
        if then_value == otherwise_value {
            return None;
        }
        let condition = &conditional.cond;
        Some(if then_value != negated {
            (**condition).clone()
        } else {
            syn::parse_quote!(!( #condition ))
        })
    }
}

impl VisitMut for CompactBooleanLiterals {
    fn visit_expr_mut(&mut self, expression: &mut syn::Expr) {
        if let syn::Expr::Unary(unary) = expression
            && unary.attrs.is_empty()
            && matches!(unary.op, syn::UnOp::Not(_))
            && let Some(conditional) = Self::conditional(&unary.expr)
            && let Some(reduced) = Self::reduced_if(conditional, true)
        {
            *expression = reduced;
            visit_mut::visit_expr_mut(self, expression);
            return;
        }
        visit_mut::visit_expr_mut(self, expression);
        let syn::Expr::If(conditional) = expression else {
            return;
        };
        if let Some(reduced) = Self::reduced_if(conditional, false) {
            *expression = reduced;
        }
    }
}

/// A stateful expression's effects are emitted before its returned value.
/// Drop only a materialized Unit value; keep effect-bearing expressions in order.
pub(crate) fn discard_expression(value: syn::Expr, ty: &Type) -> Vec<syn::Stmt> {
    if *ty != Type::Unit {
        return vec![syn::parse_quote!(let _ = #value;)];
    }
    match value {
        syn::Expr::Tuple(tuple) if tuple.elems.is_empty() => Vec::new(),
        syn::Expr::Path(path)
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && path.path.segments.len() == 1 =>
        {
            Vec::new()
        }
        syn::Expr::Field(field)
            if matches!(&*field.base, syn::Expr::Path(path)
                if path.qself.is_none()
                    && path.path.leading_colon.is_none()
                    && path.path.segments.len() == 1) =>
        {
            Vec::new()
        }
        syn::Expr::Block(block)
            if block.attrs.is_empty()
                && matches!(
                    block.block.stmts.as_slice(),
                    [syn::Stmt::Expr(syn::Expr::If(_), _)]
                ) =>
        {
            let statement = block.block.stmts.into_iter().next().expect("one statement");
            vec![statement]
        }
        other => vec![syn::parse_quote!(#other;)],
    }
}

/// Only a Boolean literal or Copy Boolean parameter is effect-free by itself.
pub(crate) fn condition_needs_statement(condition: &Expr) -> bool {
    !matches!(condition, Expr::Boolean { .. } | Expr::Parameter { .. })
}

fn lift_block_condition(
    condition: syn::Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
) -> (Vec<syn::Stmt>, syn::Expr) {
    fn contains_local(expression: &syn::Expr) -> bool {
        match expression {
            syn::Expr::Block(block) => block
                .block
                .stmts
                .iter()
                .any(|statement| matches!(statement, syn::Stmt::Local(_))),
            syn::Expr::Paren(paren) => contains_local(&paren.expr),
            syn::Expr::Group(group) => contains_local(&group.expr),
            _ => false,
        }
    }
    if !contains_local(&condition) {
        return (Vec::new(), condition);
    }
    let mut suffix = 0;
    let name = loop {
        let candidate = if suffix == 0 {
            "__compact_condition".to_owned()
        } else {
            format!("__compact_condition_{suffix}")
        };
        if parameters.values().all(|(_, name)| *name != candidate) {
            break syn::Ident::new(&candidate, Span::call_site());
        }
        suffix += 1;
    };
    (
        vec![syn::parse_quote!(let #name: bool = #condition;)],
        syn::parse_quote!(#name),
    )
}

#[cfg(test)]
mod discarded_expression_tests {
    use super::{Type, discard_expression};
    use quote::quote;

    #[test]
    fn drops_materialized_unit_but_executes_unit_effects() {
        let unit = Type::Unit;
        assert!(discard_expression(syn::parse_quote!(()), &unit).is_empty());
        assert!(discard_expression(syn::parse_quote!(__compact_witness_20), &unit).is_empty());
        assert!(discard_expression(syn::parse_quote!(__compact_call_3.result), &unit).is_empty());

        let call = discard_expression(syn::parse_quote!(pure_check()?), &unit);
        assert_eq!(quote!(#(#call)*).to_string(), "pure_check () ? ;");

        let assertion = discard_expression(
            syn::parse_quote!({
                if !condition {
                    return Err(error);
                }
            }),
            &unit,
        );
        assert_eq!(assertion.len(), 1);
        assert!(matches!(assertion[0], syn::Stmt::Expr(syn::Expr::If(_), _)));

        let value = discard_expression(syn::parse_quote!(result), &Type::Field);
        assert_eq!(quote!(#(#value)*).to_string(), "let _ = result ;");
    }
}

fn unit_statements(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Vec<syn::Stmt>, RenderError> {
    match expr {
        Expr::Unit | Expr::Default { ty: Type::Unit } => Ok(Vec::new()),
        Expr::Sequence { steps, value } => {
            let mut statements = Vec::new();
            for step in steps {
                statements.extend(unit_statements(step, parameters, circuits)?);
            }
            statements.extend(unit_statements(value, parameters, circuits)?);
            Ok(statements)
        }
        Expr::Assert { condition, message } => {
            let (condition, actual) = expression_with_calls(condition, parameters, circuits)?;
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            let (mut statements, condition) = lift_block_condition(condition, parameters);
            statements.push(syn::parse_quote!(if !(#condition) {
                return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
            }));
            Ok(statements)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let evaluate_condition = condition_needs_statement(condition);
            let (condition, actual) = expression_with_calls(condition, parameters, circuits)?;
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            let (mut statements, condition) = lift_block_condition(condition, parameters);
            if then == otherwise {
                if evaluate_condition && statements.is_empty() {
                    statements.push(syn::parse_quote!(let _ = #condition;));
                }
                statements.extend(unit_statements(then, parameters, circuits)?);
                return Ok(statements);
            }
            let then = unit_statements(then, parameters, circuits)?;
            let otherwise = unit_statements(otherwise, parameters, circuits)?;
            if otherwise.is_empty()
                && let [syn::Stmt::Expr(syn::Expr::If(inner), _)] = then.as_slice()
                && inner.else_branch.is_none()
            {
                let inner_condition = &inner.cond;
                let inner_body = &inner.then_branch;
                statements
                    .push(syn::parse_quote!(if (#condition) && (#inner_condition) #inner_body));
                return Ok(statements);
            }
            if otherwise.is_empty() && then.is_empty() {
                if statements.is_empty() && evaluate_condition {
                    statements.push(syn::parse_quote!(let _ = #condition;));
                }
            } else if otherwise.is_empty() {
                statements.push(syn::parse_quote!(if #condition { #(#then)* }));
            } else if then.is_empty() {
                statements.push(syn::parse_quote!(if !(#condition) { #(#otherwise)* }));
            } else {
                statements.push(syn::parse_quote!(if #condition {
                    #(#then)*
                } else {
                    #(#otherwise)*
                }));
            }
            Ok(statements)
        }
        Expr::Let { bindings, body } => {
            let mut locals = parameters.clone();
            let mut statements = Vec::<syn::Stmt>::new();
            for binding in bindings {
                ident(&binding.name)?;
                let (value, actual) = expression_with_calls(&binding.value, &locals, circuits)?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                let local_ty = rust_type(&binding.ty)?;
                let local_name = syn::Ident::new(
                    &format!("__compact_local_{}", binding.name),
                    Span::call_site(),
                );
                statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));
                locals.insert(binding.name.as_str(), (&binding.ty, local_name));
            }
            let body = unit_statements(body, &locals, circuits)?;
            if statements.is_empty() {
                Ok(body)
            } else {
                statements.extend(body);
                Ok(vec![syn::parse_quote!({ #(#statements)* })])
            }
        }
        _ => {
            let (rendered, actual) = expression_with_calls(expr, parameters, circuits)?;
            if actual != Type::Unit {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unit,
                    actual,
                });
            }
            Ok(vec![syn::parse_quote!(#rendered;)])
        }
    }
}

fn expression_with_calls(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::Unit => Ok((syn::parse_quote!(()), Type::Unit)),
        Expr::Default { ty } => {
            let rendered_type = rust_type(ty)?;
            Ok((
                syn::parse_quote!(<#rendered_type as Default>::default()),
                ty.clone(),
            ))
        }
        Expr::Boolean { value } => Ok((syn::parse_quote!(#value), Type::Boolean)),
        Expr::FieldLiteral { value } => {
            let bytes = field_literal_bytes(value)?;
            if let Ok(parsed) = value.parse::<u128>() {
                let value = syn::LitInt::new(&format!("{parsed}u128"), Span::call_site());
                return Ok((syn::parse_quote!(runtime::Field::from(#value)), Type::Field));
            }
            let bytes = bytes
                .iter()
                .map(|byte| syn::LitInt::new(&format!("{byte}u8"), Span::call_site()))
                .collect::<Vec<_>>();
            Ok((
                syn::parse_quote!(runtime::Field::from_le_bytes(&[#(#bytes),*]).expect("validated Compact Field literal")),
                Type::Field,
            ))
        }
        Expr::BytesLiteral { bytes } => {
            let values = bytes
                .iter()
                .map(|value| syn::LitInt::new(&format!("{value}u8"), Span::call_site()))
                .collect::<Vec<_>>();
            Ok((
                syn::parse_quote!(runtime::FixedBytes::new([#(#values),*])),
                Type::Bytes {
                    length: bytes.len(),
                },
            ))
        }
        Expr::UnsignedLiteral { value, max } => {
            let maximum = unsigned_maximum(max)?;
            let bytes =
                field_literal_bytes(value).map_err(|_| RenderError::InvalidUnsignedLiteral {
                    value: value.clone(),
                    max: max.clone(),
                })?;
            let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
            let high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
            let maximum_limbs = match maximum {
                UnsignedMaximum::Small(maximum) => (0, maximum),
                UnsignedMaximum::Wide { high, low } => (high, low),
            };
            if (high, low) > maximum_limbs {
                return Err(RenderError::InvalidUnsignedLiteral {
                    value: value.clone(),
                    max: max.clone(),
                });
            }
            let rendered: syn::Expr = match maximum {
                UnsignedMaximum::Small(_) => {
                    let max_lit = syn::LitInt::new(max, Span::call_site());
                    let value_lit = syn::LitInt::new(&format!("{value}u128"), Span::call_site());
                    syn::parse_quote!(runtime::BoundedUint::<#max_lit>::new(#value_lit).expect("Compact Uint literal fits its maximum"))
                }
                UnsignedMaximum::Wide { high, low } => {
                    let ty = wide_uint_type(high, low);
                    let bytes = bytes[..31]
                        .iter()
                        .map(|byte| syn::LitInt::new(&format!("{byte}u8"), Span::call_site()))
                        .collect::<Vec<_>>();
                    syn::parse_quote!(<#ty>::from_le_bytes(&[#(#bytes),*]).expect("Compact Uint literal fits its maximum"))
                }
            };
            Ok((rendered, Type::Unsigned { max: max.clone() }))
        }
        Expr::Parameter { name } => {
            let (ty, rust_name) = parameters
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
            let value = if copy_type(ty) {
                syn::parse_quote!(#rust_name)
            } else {
                syn::parse_quote!(#rust_name.clone())
            };
            Ok((value, (*ty).clone()))
        }
        Expr::EnumVariant { ty, variant } => {
            let Type::Enum { variants, .. } = ty else {
                return Err(RenderError::InvalidEnumVariant(variant.clone()));
            };
            if !variants.contains(variant) {
                return Err(RenderError::InvalidEnumVariant(variant.clone()));
            }
            let rust_ty = rust_type(ty)?;
            let rust_variant = ident(variant)?;
            Ok((syn::parse_quote!(#rust_ty::#rust_variant), ty.clone()))
        }
        Expr::StructField {
            value,
            field,
            index,
        } => {
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            let Type::Struct { fields, .. } = ty else {
                return Err(RenderError::InvalidStructField(field.clone()));
            };
            let declaration = fields
                .get(*index)
                .filter(|declaration| declaration.name == *field)
                .ok_or_else(|| RenderError::InvalidStructField(field.clone()))?;
            let name = ident(field)?;
            let field_value = retained_value(syn::parse_quote!((#value).#name), &declaration.ty);
            Ok((field_value, declaration.ty.clone()))
        }
        Expr::TupleIndex { value, index } => {
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            let Type::Tuple { elements } = ty else {
                return Err(RenderError::ExpectedTuple(ty));
            };
            let result = elements
                .get(*index)
                .ok_or(RenderError::InvalidTupleIndex(*index))?
                .clone();
            let index = syn::Index::from(*index);
            Ok((syn::parse_quote!((#value).#index), result))
        }
        Expr::StructLiteral { ty, fields } => {
            let Type::Struct {
                name,
                fields: declarations,
            } = ty
            else {
                return Err(RenderError::InvalidStructField("<literal>".into()));
            };
            if fields.len() != declarations.len() {
                return Err(RenderError::InvalidStructField(name.clone()));
            }
            let mut field_values = Vec::<syn::FieldValue>::with_capacity(fields.len());
            for (value, declaration) in fields.iter().zip(declarations) {
                let (rendered, actual) = expression_with_calls(value, parameters, circuits)?;
                if actual != declaration.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: declaration.ty.clone(),
                        actual,
                    });
                }
                let field_name = ident(&declaration.name)?;
                let shorthand = matches!(
                    &rendered,
                    syn::Expr::Path(path)
                        if path.qself.is_none()
                            && path.path.leading_colon.is_none()
                            && path.path.segments.len() == 1
                            && path.path.segments[0].ident == field_name
                            && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
                );
                field_values.push(if shorthand {
                    syn::parse_quote!(#field_name)
                } else {
                    syn::parse_quote!(#field_name: #rendered)
                });
            }
            let name = ident(name)?;
            Ok((
                syn::parse_quote!(crate::types::#name { #(#field_values),* }),
                ty.clone(),
            ))
        }
        Expr::Assert { condition, message } => {
            let (condition, actual) = expression_with_calls(condition, parameters, circuits)?;
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            let (prefix, condition) = lift_block_condition(condition, parameters);
            Ok((
                syn::parse_quote!({
                    #(#prefix)*
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                }),
                Type::Unit,
            ))
        }
        Expr::Sequence { steps, value } => {
            let mut statements = Vec::<syn::Stmt>::new();
            for step in steps {
                let (_, actual) = expression_with_calls(step, parameters, circuits)?;
                if actual != Type::Unit {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Unit,
                        actual,
                    });
                }
                statements.extend(unit_statements(step, parameters, circuits)?);
            }
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            Ok((syn::parse_quote!({ #(#statements)* #value }), ty))
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
        Expr::Vector { element, elements } => {
            let mut values = Vec::new();
            for value in elements {
                let (rendered, actual) = expression_with_calls(value, parameters, circuits)?;
                if actual != *element {
                    return Err(RenderError::TypeMismatch {
                        expected: element.clone(),
                        actual,
                    });
                }
                values.push(rendered);
            }
            let length = elements.len();
            Ok((
                syn::parse_quote!(runtime::FixedVector::new([#(#values),*])),
                Type::Vector {
                    element: Box::new(element.clone()),
                    length,
                },
            ))
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => {
            let (source, source_ty) = expression_with_calls(source, parameters, circuits)?;
            let target_source_ty = Type::Vector {
                element: Box::new(parameter.ty.clone()),
                length: *length,
            };
            let source = coerce_expression(source, &source_ty, &target_source_ty, 0)?;
            let item_name = ident(&parameter.name)?;
            let mut body_parameters = parameters.clone();
            body_parameters.insert(parameter.name.as_str(), (&parameter.ty, item_name.clone()));
            let (body, actual) = expression_with_calls(body, &body_parameters, circuits)?;
            if actual != *result {
                return Err(RenderError::TypeMismatch {
                    expected: result.clone(),
                    actual,
                });
            }
            let mapped = syn::Ident::new("__compact_mapped", Span::call_site());
            let source_name = syn::Ident::new("__compact_map_source", Span::call_site());
            let length_lit = syn::LitInt::new(&length.to_string(), Span::call_site());
            Ok((
                syn::parse_quote!({
                    let #source_name = #source;
                    let mut #mapped = Vec::with_capacity(#length_lit);
                    for #item_name in #source_name.into_array() {
                        #mapped.push(#body);
                    }
                    runtime::FixedVector::new(#mapped.try_into().expect("Vector map preserves its length"))
                }),
                Type::Vector {
                    element: Box::new(result.clone()),
                    length: *length,
                },
            ))
        }
        Expr::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => {
            let circuit = circuits
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
            if circuit.parameters.len() != 2
                || circuit.parameters[0].ty != *accumulator
                || circuit.parameters[1].ty != *element
                || circuit.result != *accumulator
            {
                return Err(RenderError::TypeMismatch {
                    expected: accumulator.clone(),
                    actual: circuit.result.clone(),
                });
            }
            let (initial, initial_ty) = expression_with_calls(initial, parameters, circuits)?;
            let initial = coerce_expression(initial, &initial_ty, accumulator, 0)?;
            let (source, source_ty) = expression_with_calls(source, parameters, circuits)?;
            let expected_source = Type::Vector {
                element: Box::new(element.clone()),
                length: *length,
            };
            let source = coerce_expression(source, &source_ty, &expected_source, 0)?;
            let function = ident(name)?;
            let acc = syn::Ident::new("__compact_fold_accumulator", Span::call_site());
            let item = syn::Ident::new("__compact_fold_item", Span::call_site());
            let source_name = syn::Ident::new("__compact_fold_source", Span::call_site());
            Ok((
                syn::parse_quote!({
                    let #source_name = #source;
                    let mut #acc = #initial;
                    for #item in #source_name.into_array() {
                        #acc = crate::pure_circuits::#function(#acc, #item)?;
                    }
                    #acc
                }),
                accumulator.clone(),
            ))
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let evaluate_condition = condition_needs_statement(condition);
            let (condition, condition_ty) = expression_with_calls(condition, parameters, circuits)?;
            if condition_ty != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: condition_ty,
                });
            }
            let (mut prefix, condition) = lift_block_condition(condition, parameters);
            if then == otherwise {
                let (value, ty) = expression_with_calls(then, parameters, circuits)?;
                if evaluate_condition && prefix.is_empty() {
                    prefix.push(syn::parse_quote!(let _ = #condition;));
                }
                return Ok((
                    if prefix.is_empty() {
                        value
                    } else {
                        syn::parse_quote!({ #(#prefix)* #value })
                    },
                    ty,
                ));
            }
            let (then, then_ty) = expression_with_calls(then, parameters, circuits)?;
            let (otherwise, otherwise_ty) = expression_with_calls(otherwise, parameters, circuits)?;
            if then_ty != otherwise_ty {
                return Err(RenderError::TypeMismatch {
                    expected: then_ty,
                    actual: otherwise_ty,
                });
            }
            let conditional: syn::Expr =
                syn::parse_quote!(if #condition { #then } else { #otherwise });
            Ok((
                if prefix.is_empty() {
                    conditional
                } else {
                    syn::parse_quote!({ #(#prefix)* #conditional })
                },
                then_ty,
            ))
        }
        Expr::Let { bindings, body } => {
            let mut locals = parameters.clone();
            let mut statements = Vec::<syn::Stmt>::new();
            for binding in bindings {
                ident(&binding.name)?;
                let (value, actual) = expression_with_calls(&binding.value, &locals, circuits)?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                let local_ty = rust_type(&binding.ty)?;
                let local_name = syn::Ident::new(
                    &format!("__compact_local_{}", binding.name),
                    Span::call_site(),
                );
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
        Expr::TransientHash { value } => {
            let (value, _) = expression_with_calls(value, parameters, circuits)?;
            Ok((
                syn::parse_quote!(runtime::transient_hash(#value)),
                Type::Field,
            ))
        }
        Expr::TransientCommit { value, opening } => {
            let (value, _) = expression_with_calls(value, parameters, circuits)?;
            let (opening, actual) = expression_with_calls(opening, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::transient_commit(#value, #opening)),
                Type::Field,
            ))
        }
        Expr::PersistentHash { value } => {
            let (value, _) = expression_with_calls(value, parameters, circuits)?;
            Ok((
                syn::parse_quote!(runtime::persistent_hash(#value)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::Keccak256 { value } => {
            let (value, _) = expression_with_calls(value, parameters, circuits)?;
            Ok((
                syn::parse_quote!(runtime::keccak256(#value)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::PersistentCommit { value, opening } => {
            let (value, _) = expression_with_calls(value, parameters, circuits)?;
            let (opening, actual) = expression_with_calls(opening, parameters, circuits)?;
            if actual != (Type::Bytes { length: 32 }) {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Bytes { length: 32 },
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::persistent_commit(#value, #opening)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::DegradeToTransient { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != (Type::Bytes { length: 32 }) {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Bytes { length: 32 },
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::degrade_to_transient(#value)),
                Type::Field,
            ))
        }
        Expr::UpgradeFromTransient { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::upgrade_from_transient(#value)),
                Type::Bytes { length: 32 },
            ))
        }
        Expr::HashToCurve { value } => {
            let (value, _) = expression_with_calls(value, parameters, circuits)?;
            Ok((
                syn::parse_quote!(runtime::hash_to_curve(#value)),
                Type::JubjubPoint,
            ))
        }
        Expr::JubjubPointX { value } | Expr::JubjubPointY { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::JubjubPoint {
                return Err(RenderError::TypeMismatch {
                    expected: Type::JubjubPoint,
                    actual,
                });
            }
            let operation: syn::Path = match expr {
                Expr::JubjubPointX { .. } => syn::parse_quote!(runtime::jubjub_point_x),
                Expr::JubjubPointY { .. } => syn::parse_quote!(runtime::jubjub_point_y),
                _ => unreachable!(),
            };
            Ok((syn::parse_quote!(#operation(#value)), Type::Field))
        }
        Expr::EcAdd { left, right } => {
            let (left, left_ty) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_ty) = expression_with_calls(right, parameters, circuits)?;
            for actual in [left_ty, right_ty] {
                if actual != Type::JubjubPoint {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::JubjubPoint,
                        actual,
                    });
                }
            }
            Ok((
                syn::parse_quote!(runtime::ec_add(#left, #right)),
                Type::JubjubPoint,
            ))
        }
        Expr::ConstructJubjubPoint { x, y } => {
            let (x, x_ty) = expression_with_calls(x, parameters, circuits)?;
            let (y, y_ty) = expression_with_calls(y, parameters, circuits)?;
            for actual in [x_ty, y_ty] {
                if actual != Type::Field {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Field,
                        actual,
                    });
                }
            }
            Ok((
                syn::parse_quote!(runtime::construct_jubjub_point(#x, #y)?),
                Type::JubjubPoint,
            ))
        }
        Expr::EcNeg { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::JubjubPoint {
                return Err(RenderError::TypeMismatch {
                    expected: Type::JubjubPoint,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::ec_neg(#value)),
                Type::JubjubPoint,
            ))
        }
        Expr::EcMul { point, scalar } => {
            let (point, point_ty) = expression_with_calls(point, parameters, circuits)?;
            let (scalar, scalar_ty) = expression_with_calls(scalar, parameters, circuits)?;
            if point_ty != Type::JubjubPoint {
                return Err(RenderError::TypeMismatch {
                    expected: Type::JubjubPoint,
                    actual: point_ty,
                });
            }
            if scalar_ty != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: scalar_ty,
                });
            }
            Ok((
                syn::parse_quote!(runtime::ec_mul(#point, #scalar)?),
                Type::JubjubPoint,
            ))
        }
        Expr::EcMulGenerator { scalar } => {
            let (scalar, actual) = expression_with_calls(scalar, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::ec_mul_generator(#scalar)?),
                Type::JubjubPoint,
            ))
        }
        Expr::JubjubScalarFromNative { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((
                syn::parse_quote!(runtime::jubjub_scalar_from_native(#value)),
                Type::Field,
            ))
        }
        Expr::WitnessCall { .. }
        | Expr::KernelClaim { .. }
        | Expr::KernelMintShielded { .. }
        | Expr::CreateZswapInput { .. }
        | Expr::CreateZswapOutput { .. }
        | Expr::NativeWitnessCall { .. }
        | Expr::SetMember { .. }
        | Expr::MapMember { .. }
        | Expr::MapLookup { .. }
        | Expr::MerkleCheckRoot { .. }
        | Expr::HistoricMerkleCheckRoot { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::SetSize { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::ListHead { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. }
        | Expr::CounterLessThan { .. }
        | Expr::KernelSelf { .. } => Err(RenderError::EffectfulExpression),
        Expr::FieldCast { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            let Type::Unsigned { max } = &actual else {
                return Err(RenderError::ExpectedUnsigned(actual));
            };
            let rendered = match unsigned_maximum(max)? {
                UnsignedMaximum::Small(_) => {
                    syn::parse_quote!(runtime::Field::from((#value).value()))
                }
                UnsignedMaximum::Wide { .. } => syn::parse_quote!((#value).as_field()),
            };
            Ok((rendered, Type::Field))
        }
        Expr::FieldToBytes32 { value } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            Ok((field_to_bytes_32_syntax(value), Type::Bytes { length: 32 }))
        }
        Expr::Coerce { value, ty } => {
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            Ok((coerce_expression(value, &actual, ty, 0)?, ty.clone()))
        }
        Expr::UnsignedCast { max, value } => {
            unsigned_maximum(max)?;
            let (value, actual) = expression_with_calls(value, parameters, circuits)?;
            let Type::Unsigned { max: source_max } = actual else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned { max: max.clone() },
                    actual,
                });
            };
            Ok((
                unsigned_cast_syntax(value, &source_max, max)?,
                Type::Unsigned { max: max.clone() },
            ))
        }
        Expr::UnsignedAdd { max, left, right }
        | Expr::UnsignedSubtract { max, left, right }
        | Expr::UnsignedMultiply { max, left, right } => {
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
            Ok((
                unsigned_arithmetic_syntax(expr, left, right, &left_max, &right_max, max)?,
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
        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {
            let (left, left_ty) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_ty) = expression_with_calls(right, parameters, circuits)?;
            if right_ty != left_ty {
                return Err(RenderError::TypeMismatch {
                    expected: left_ty,
                    actual: right_ty,
                });
            }
            let rendered = if matches!(expr, Expr::Equal { .. }) {
                syn::parse_quote!(#left == #right)
            } else {
                syn::parse_quote!(#left != #right)
            };
            Ok((rendered, Type::Boolean))
        }
        Expr::Compare {
            operator,
            left,
            right,
        } => {
            let (left, left_ty) = expression_with_calls(left, parameters, circuits)?;
            let (right, right_ty) = expression_with_calls(right, parameters, circuits)?;
            for actual in [left_ty, right_ty] {
                let Type::Unsigned { max } = &actual else {
                    return Err(RenderError::ExpectedUnsigned(actual));
                };
                if matches!(unsigned_maximum(max)?, UnsignedMaximum::Wide { .. }) {
                    return Err(RenderError::InvalidUnsignedMaximum(max.clone()));
                }
            }
            let rendered = match operator {
                ComparisonOperator::Less => syn::parse_quote!(#left.value() < #right.value()),
                ComparisonOperator::LessEqual => syn::parse_quote!(#left.value() <= #right.value()),
                ComparisonOperator::Greater => syn::parse_quote!(#left.value() > #right.value()),
                ComparisonOperator::GreaterEqual => {
                    syn::parse_quote!(#left.value() >= #right.value())
                }
            };
            Ok((rendered, Type::Boolean))
        }
    }
}

fn infallible_constructor_expr(value: &Expr) -> bool {
    match value {
        Expr::Unit
        | Expr::Default { .. }
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Parameter { .. } => true,
        Expr::Vector { elements, .. } | Expr::Tuple { elements } => {
            elements.iter().all(infallible_constructor_expr)
        }
        Expr::StructLiteral { fields, .. } => fields.iter().all(infallible_constructor_expr),
        Expr::PersistentHash { value }
        | Expr::HashToCurve { value }
        | Expr::JubjubPointX { value }
        | Expr::JubjubPointY { value }
        | Expr::StructField { value, .. }
        | Expr::Coerce { value, .. }
        | Expr::FieldCast { value } => infallible_constructor_expr(value),
        _ => false,
    }
}

fn collect_constructor_step_types(
    step: &ConstructorStep,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), RenderError> {
    match step {
        ConstructorStep::Expression { value } => {
            collect_expression_types(value, structs, enums)?;
        }
        ConstructorStep::Let { bindings, step } => {
            for binding in bindings {
                collect_named_types(&binding.ty, structs, enums)?;
                collect_expression_types(&binding.value, structs, enums)?;
            }
            collect_constructor_step_types(step, structs, enums)?;
        }
        ConstructorStep::Sequence { steps } => {
            for step in steps {
                collect_constructor_step_types(step, structs, enums)?;
            }
        }
        ConstructorStep::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums)?
        }
        ConstructorStep::CellWrite { value, .. }
        | ConstructorStep::SetInsert { value, .. }
        | ConstructorStep::SetRemove { value, .. }
        | ConstructorStep::ListPushFront { value, .. } => {
            collect_expression_types(value, structs, enums)?
        }
        ConstructorStep::MapInsert { key, value, .. } => {
            collect_expression_types(key, structs, enums)?;
            collect_expression_types(value, structs, enums)?;
        }
        ConstructorStep::MapInsertDefault { key, .. } | ConstructorStep::MapRemove { key, .. } => {
            collect_expression_types(key, structs, enums)?
        }
        ConstructorStep::CounterIncrement { .. }
        | ConstructorStep::CounterDecrement { .. }
        | ConstructorStep::CounterReset { .. }
        | ConstructorStep::SetReset { .. }
        | ConstructorStep::ListPopFront { .. }
        | ConstructorStep::ListReset { .. }
        | ConstructorStep::MapReset { .. } => {}
        ConstructorStep::ForEach {
            binding,
            values,
            steps,
        } => {
            collect_named_types(&binding.ty, structs, enums)?;
            for value in values {
                collect_expression_types(value, structs, enums)?;
            }
            for step in steps {
                collect_constructor_step_types(step, structs, enums)?;
            }
        }
        ConstructorStep::ForEachVector {
            binding,
            source,
            steps,
            ..
        } => {
            collect_named_types(&binding.ty, structs, enums)?;
            collect_expression_types(source, structs, enums)?;
            for step in steps {
                collect_constructor_step_types(step, structs, enums)?;
            }
        }
        ConstructorStep::If {
            condition,
            then_steps,
            otherwise_steps,
        } => {
            collect_expression_types(condition, structs, enums)?;
            for step in then_steps.iter().chain(otherwise_steps) {
                collect_constructor_step_types(step, structs, enums)?;
            }
        }
    }
    Ok(())
}

fn constructor_step_uses_witness(
    step: &ConstructorStep,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<bool, RenderError> {
    let requires = |value: &Expr| stateful::expression_requires_witness(value, stateful_circuits);
    match step {
        ConstructorStep::Expression { value } => requires(value),
        ConstructorStep::Let { bindings, step } => {
            for binding in bindings {
                if requires(&binding.value)? {
                    return Ok(true);
                }
            }
            constructor_step_uses_witness(step, stateful_circuits)
        }
        ConstructorStep::Sequence { steps } => {
            for step in steps {
                if constructor_step_uses_witness(step, stateful_circuits)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ConstructorStep::Assert { condition, .. } => requires(condition),
        ConstructorStep::CellWrite { value, .. }
        | ConstructorStep::SetInsert { value, .. }
        | ConstructorStep::SetRemove { value, .. }
        | ConstructorStep::ListPushFront { value, .. }
        | ConstructorStep::MapInsertDefault { key: value, .. }
        | ConstructorStep::MapRemove { key: value, .. } => requires(value),
        ConstructorStep::MapInsert { key, value, .. } => Ok(requires(key)? || requires(value)?),
        ConstructorStep::ForEach { values, steps, .. } => {
            for value in values {
                if requires(value)? {
                    return Ok(true);
                }
            }
            for step in steps {
                if constructor_step_uses_witness(step, stateful_circuits)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ConstructorStep::ForEachVector { source, steps, .. } => {
            if requires(source)? {
                return Ok(true);
            }
            for step in steps {
                if constructor_step_uses_witness(step, stateful_circuits)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ConstructorStep::If {
            condition,
            then_steps,
            otherwise_steps,
        } => {
            if requires(condition)? {
                return Ok(true);
            }
            for step in then_steps.iter().chain(otherwise_steps) {
                if constructor_step_uses_witness(step, stateful_circuits)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ConstructorStep::CounterIncrement { .. }
        | ConstructorStep::CounterDecrement { .. }
        | ConstructorStep::CounterReset { .. }
        | ConstructorStep::SetReset { .. }
        | ConstructorStep::ListPopFront { .. }
        | ConstructorStep::ListReset { .. }
        | ConstructorStep::MapReset { .. } => Ok(false),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "constructor VM lowering threads declarations and ordered loop and temporary state explicitly"
)]
fn render_constructor_vm_steps<'a>(
    steps: &'a [ConstructorStep],
    ledger_fields: &HashMap<&str, &ir::LedgerField>,
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    witnesses: &HashMap<&str, &ir::WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    next_loop: &mut usize,
    next_temp: &mut usize,
) -> Result<Vec<syn::Stmt>, RenderError> {
    let mut actions = Vec::new();
    for step in steps {
        match step {
            ConstructorStep::Expression { value } => {
                let mut expression_steps = Vec::new();
                let mut query_effect = false;
                let (value, ty, _) = stateful::render_state_expression(
                    value,
                    parameters,
                    witnesses,
                    &mut expression_steps,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                actions.extend(expression_steps);
                actions.extend(discard_expression(value, &ty));
            }
            ConstructorStep::Let { bindings, step } => {
                let mut locals = parameters.clone();
                for binding in bindings {
                    ident(&binding.name)?;
                    let mut expression_steps = Vec::new();
                    let mut query_effect = false;
                    let (value, actual, _witness_effect) = stateful::render_state_expression(
                        &binding.value,
                        &locals,
                        witnesses,
                        &mut expression_steps,
                        next_temp,
                        circuits,
                        stateful_circuits,
                        ledger_fields,
                        &mut query_effect,
                    )?;
                    if actual != binding.ty {
                        return Err(RenderError::TypeMismatch {
                            expected: binding.ty.clone(),
                            actual,
                        });
                    }
                    actions.extend(expression_steps);
                    let local_name = syn::Ident::new(
                        &format!("__compact_constructor_local_{}", *next_temp),
                        Span::call_site(),
                    );
                    *next_temp += 1;
                    let ty = rust_type(&binding.ty)?;
                    actions.push(syn::parse_quote!(let #local_name: #ty = #value;));
                    locals.insert(binding.name.as_str(), (&binding.ty, local_name));
                }
                actions.extend(render_constructor_vm_steps(
                    std::slice::from_ref(step.as_ref()),
                    ledger_fields,
                    &locals,
                    witnesses,
                    circuits,
                    stateful_circuits,
                    next_loop,
                    next_temp,
                )?);
            }
            ConstructorStep::Sequence { steps } => {
                actions.extend(render_constructor_vm_steps(
                    steps,
                    ledger_fields,
                    parameters,
                    witnesses,
                    circuits,
                    stateful_circuits,
                    next_loop,
                    next_temp,
                )?);
            }
            ConstructorStep::Assert { condition, message } => {
                let mut expression_steps = Vec::new();
                let mut query_effect = false;
                let (condition, actual, _witness_effect) = stateful::render_state_expression(
                    condition,
                    parameters,
                    witnesses,
                    &mut expression_steps,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if actual != Type::Boolean {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Boolean,
                        actual,
                    });
                }
                actions.extend(expression_steps);
                actions.push(syn::parse_quote! {
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                });
            }
            ConstructorStep::CellWrite {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.index != *index {
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                };
                let mut expression_steps = Vec::new();
                let mut query_effect = false;
                let (value, actual, _witness_effect) = stateful::render_state_expression(
                    value,
                    parameters,
                    witnesses,
                    &mut expression_steps,
                    next_temp,
                    circuits,
                    stateful_circuits,
                    ledger_fields,
                    &mut query_effect,
                )?;
                if actual != *ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let value = retained_value(value, ty);
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                actions.extend(expression_steps);
                let path = declaration.physical_path();
                let write: syn::Stmt = if path.len() == 1 {
                    syn::parse_quote!(let step = context.write_cell(#index, #value)?;)
                } else {
                    let path = path
                        .iter()
                        .map(|part| syn::LitInt::new(&part.to_string(), Span::call_site()))
                        .collect::<Vec<_>>();
                    syn::parse_quote!(let step = context.write_cell_at_path(&[#(#path),*], #value)?;)
                };
                actions.push(write);
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::CounterIncrement {
                field,
                index,
                amount,
            }
            | ConstructorStep::CounterDecrement {
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
                let amount: syn::Expr = match amount {
                    CounterAmount::Literal { value } => {
                        let value = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                        syn::parse_quote!(#value)
                    }
                    CounterAmount::Parameter { name } => {
                        let (ty, rust_name) = parameters
                            .get(name.as_str())
                            .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
                        let expected = Type::Unsigned {
                            max: "65535".into(),
                        };
                        if *ty != &expected {
                            return Err(RenderError::TypeMismatch {
                                expected,
                                actual: (*ty).clone(),
                            });
                        }
                        syn::parse_quote!(#rust_name.value() as u16)
                    }
                };
                let index = ledger_path_expr(declaration);
                let method = if matches!(step, ConstructorStep::CounterIncrement { .. }) {
                    syn::Ident::new("increment_counter", Span::call_site())
                } else {
                    syn::Ident::new("decrement_counter", Span::call_site())
                };
                actions.push(syn::parse_quote!(let step = context.#method(#index, #amount)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::CounterReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = ledger_path_expr(declaration);
                actions.push(syn::parse_quote!(let step = context.write_cell(#index, 0_u64)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::SetInsert {
                field,
                index,
                value,
            }
            | ConstructorStep::SetRemove {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.index != *index {
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(value) {
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let (value, actual) = expression_with_calls(value, parameters, &HashMap::new())?;
                if actual != *ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let value = retained_value(value, ty);
                let method = if matches!(step, ConstructorStep::SetInsert { .. }) {
                    syn::Ident::new("insert_set", Span::call_site())
                } else {
                    syn::Ident::new("remove_set", Span::call_site())
                };
                let index = ledger_path_expr(declaration);
                actions.push(syn::parse_quote!(let step = context.#method(#index, #value)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::SetReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = ledger_path_expr(declaration);
                actions.push(syn::parse_quote!(let step = context.reset_set(#index)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::ListPushFront {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.index != *index {
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::List { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(value) {
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let (value, actual) = expression_with_calls(value, parameters, &HashMap::new())?;
                if actual != *ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let value = retained_value(value, ty);
                let index = ledger_path_expr(declaration);
                actions
                    .push(syn::parse_quote!(let step = context.push_front_list(#index, #value)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::ListPopFront { field, index }
            | ConstructorStep::ListReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let method = if matches!(step, ConstructorStep::ListPopFront { .. }) {
                    syn::Ident::new("pop_front_list", Span::call_site())
                } else {
                    syn::Ident::new("reset_list", Span::call_site())
                };
                let index = ledger_path_expr(declaration);
                actions.push(syn::parse_quote!(let step = context.#method(#index)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::MapInsert {
                field,
                index,
                key,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.index != *index {
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(key) || !infallible_constructor_expr(value) {
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, parameters, &HashMap::new())?;
                if actual_key != *key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let (value, actual_value) =
                    expression_with_calls(value, parameters, &HashMap::new())?;
                if actual_value != *value_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: value_ty.clone(),
                        actual: actual_value,
                    });
                }
                let key = retained_value(key, key_ty);
                let value = retained_value(value, value_ty);
                let index = ledger_path_expr(declaration);
                actions
                    .push(syn::parse_quote!(let step = context.insert_map(#index, #key, #value)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::MapInsertDefault { field, index, key }
            | ConstructorStep::MapRemove { field, index, key } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.index != *index {
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(key) {
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let (key, actual_key) = expression_with_calls(key, parameters, &HashMap::new())?;
                if actual_key != *key_ty {
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let key = retained_value(key, key_ty);
                let index = ledger_path_expr(declaration);
                if matches!(step, ConstructorStep::MapInsertDefault { .. }) {
                    let value_ty = rust_type(value_ty)?;
                    actions.push(syn::parse_quote!(let step = context.insert_map(#index, #key, <#value_ty as Default>::default())?;));
                } else {
                    actions.push(syn::parse_quote!(let step = context.remove_map(#index, #key)?;));
                }
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::MapReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = ledger_path_expr(declaration);
                actions.push(syn::parse_quote!(let step = context.reset_map(#index)?;));
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::ForEach {
                binding,
                values,
                steps,
            } => {
                let loop_index = *next_loop;
                *next_loop += 1;
                let item = syn::Ident::new(
                    &format!("__compact_constructor_item_{loop_index}"),
                    Span::call_site(),
                );
                let iterable = syn::Ident::new(
                    &format!("__compact_constructor_values_{loop_index}"),
                    Span::call_site(),
                );
                let mut rendered_values = Vec::new();
                for value in values {
                    if !infallible_constructor_expr(value) {
                        return Err(RenderError::InvalidConstructorInitializer(
                            binding.name.clone(),
                        ));
                    }
                    let (value, actual) =
                        expression_with_calls(value, parameters, &HashMap::new())?;
                    if actual != binding.ty {
                        return Err(RenderError::TypeMismatch {
                            expected: binding.ty.clone(),
                            actual,
                        });
                    }
                    rendered_values.push(value);
                }
                let mut body_parameters = parameters.clone();
                body_parameters.insert(binding.name.as_str(), (&binding.ty, item.clone()));
                let body = render_constructor_vm_steps(
                    steps,
                    ledger_fields,
                    &body_parameters,
                    witnesses,
                    circuits,
                    stateful_circuits,
                    next_loop,
                    next_temp,
                )?;
                let ty = rust_type(&binding.ty)?;
                let len = syn::LitInt::new(&values.len().to_string(), Span::call_site());
                let values = rendered_values
                    .into_iter()
                    .map(|value| retained_value(value, &binding.ty))
                    .collect::<Vec<_>>();
                actions.push(syn::parse_quote!(let #iterable: [#ty; #len] = [#(#values),*];));
                actions.push(syn::parse_quote!(for #item in #iterable { #(#body)* }));
            }
            ConstructorStep::ForEachVector {
                binding,
                source,
                length,
                steps,
            } => {
                let loop_index = *next_loop;
                *next_loop += 1;
                let item = syn::Ident::new(
                    &format!("__compact_constructor_item_{loop_index}"),
                    Span::call_site(),
                );
                let Expr::Parameter { name } = source else {
                    return Err(RenderError::InvalidConstructorInitializer(
                        binding.name.clone(),
                    ));
                };
                let (actual, iterable) = parameters
                    .get(name.as_str())
                    .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
                let expected = Type::Vector {
                    element: Box::new(binding.ty.clone()),
                    length: *length,
                };
                if **actual != expected {
                    return Err(RenderError::TypeMismatch {
                        expected,
                        actual: (*actual).clone(),
                    });
                }
                let mut body_parameters = parameters.clone();
                body_parameters.insert(binding.name.as_str(), (&binding.ty, item.clone()));
                let body = render_constructor_vm_steps(
                    steps,
                    ledger_fields,
                    &body_parameters,
                    witnesses,
                    circuits,
                    stateful_circuits,
                    next_loop,
                    next_temp,
                )?;
                actions.push(syn::parse_quote!(for #item in #iterable.0.iter() { #(#body)* }));
            }
            ConstructorStep::If {
                condition,
                then_steps,
                otherwise_steps,
            } => {
                if !infallible_constructor_expr(condition) {
                    return Err(RenderError::InvalidConstructorInitializer("if".into()));
                }
                let (condition, actual) =
                    expression_with_calls(condition, parameters, &HashMap::new())?;
                if actual != Type::Boolean {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Boolean,
                        actual,
                    });
                }
                let then_body = render_constructor_vm_steps(
                    then_steps,
                    ledger_fields,
                    parameters,
                    witnesses,
                    circuits,
                    stateful_circuits,
                    next_loop,
                    next_temp,
                )?;
                let otherwise_body = render_constructor_vm_steps(
                    otherwise_steps,
                    ledger_fields,
                    parameters,
                    witnesses,
                    circuits,
                    stateful_circuits,
                    next_loop,
                    next_temp,
                )?;
                if otherwise_body.is_empty() {
                    actions.push(syn::parse_quote!(if #condition { #(#then_body)* }));
                } else {
                    actions.push(syn::parse_quote!(if #condition { #(#then_body)* } else { #(#otherwise_body)* }));
                }
            }
        }
    }
    Ok(actions)
}

/// Mirror the compiler's 15-wide public-ledger batching, including the
/// leading remainder group used when the field count is not divisible by 15.
fn expected_ledger_paths(field_count: usize) -> Vec<Vec<u8>> {
    const SEGMENT: usize = 15;
    if field_count <= SEGMENT {
        return (0..field_count).map(|index| vec![index as u8]).collect();
    }
    let remainder = field_count % SEGMENT;
    let mut group_sizes = Vec::new();
    if remainder != 0 {
        group_sizes.push(remainder);
    }
    group_sizes.extend(std::iter::repeat_n(SEGMENT, field_count / SEGMENT));
    expected_ledger_paths(group_sizes.len())
        .into_iter()
        .zip(group_sizes)
        .flat_map(|(prefix, size)| {
            (0..size).map(move |index| {
                let mut path = prefix.clone();
                path.push(index as u8);
                path
            })
        })
        .collect()
}

/// The compiler gives distinct instantiations of a Compact struct names such
/// as `MerkleTreePath` and `MerkleTreePathCompact1`.
fn is_compact_struct_instantiation(name: &str, source_name: &str) -> bool {
    name == source_name
        || name
            .strip_prefix(source_name)
            .and_then(|suffix| suffix.strip_prefix("Compact"))
            .is_some_and(|index| {
                !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
            })
}

/// Preserve the compiler-assigned concrete name of a `Maybe<T>` instantiation
/// while checking the exact List element shape. A source can contain several
/// `Maybe<T>` types, so only the first one is necessarily named `Maybe`.
fn list_head_result_type(element: &Type, actual: &Type) -> Type {
    let name = match actual {
        Type::Struct { name, .. } if is_compact_struct_instantiation(name, "Maybe") => name.clone(),
        _ => "Maybe".into(),
    };
    Type::Struct {
        name,
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: element.clone(),
            },
        ],
    }
}

pub fn render(contract: &Contract) -> Result<String, RenderError> {
    Ok(render_with_capabilities(contract)?.source)
}

/// Render the publishable schema-3 capability report using compiler proof metadata.
/// `render_with_capabilities` exposes the schema-2 lowering draft for callers
/// that need to inspect reasons before a compiler contract-info file exists.
pub fn render_with_proof_capabilities(
    contract: &Contract,
    contract_info: &Value,
) -> Result<RenderedContract, RenderError> {
    let mut rendered = render_with_capabilities(contract)?;
    rendered
        .capabilities
        .apply_contract_info(contract_info)
        .map_err(RenderError::ProofApplicability)?;
    Ok(rendered)
}

pub fn render_with_capabilities(contract: &Contract) -> Result<RenderedContract, RenderError> {
    if contract.schema_version != SCHEMA_VERSION {
        return Err(RenderError::SchemaVersion(contract.schema_version));
    }

    // Build callable lookup maps only after checking their keys. Otherwise a
    // later declaration silently replaces an earlier one in a HashMap, and a
    // caller can fail with a misleading type error before we reach the
    // duplicate declaration and its Compact source location.
    let mut names = HashSet::new();
    for (name, source) in contract
        .circuits
        .iter()
        .map(|circuit| (&circuit.name, circuit.source.as_ref()))
        .chain(
            contract
                .stateful_circuits
                .iter()
                .map(|circuit| (&circuit.name, circuit.source.as_ref())),
        )
    {
        located(source, || {
            ident(name)?;
            if !names.insert(name.as_str()) {
                return Err(RenderError::DuplicateCircuit(name.clone()));
            }
            Ok(())
        })?;
    }
    // Pure functions and ledger methods live in distinct generated Rust
    // namespaces. Check each independently after raw-name duplicates, since
    // Compact `$` and Rust keywords can normalize distinct source names to
    // the same emitted function identifier.
    validate_circuit_function_namespace(
        contract
            .circuits
            .iter()
            .map(|circuit| (circuit.name.as_str(), circuit.source.as_ref())),
    )?;
    validate_circuit_function_namespace(
        contract
            .stateful_circuits
            .iter()
            .map(|circuit| (circuit.name.as_str(), circuit.source.as_ref())),
    )?;

    let mut struct_definitions = BTreeMap::new();
    let mut enum_definitions = BTreeMap::new();
    if let Some(constructor) = &contract.constructor {
        located(constructor.source.as_ref(), || {
            for parameter in &constructor.parameters {
                collect_named_types(
                    &parameter.ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                )?;
            }
            for step in &constructor.steps {
                collect_constructor_step_types(
                    step,
                    &mut struct_definitions,
                    &mut enum_definitions,
                )?;
            }
            Ok(())
        })?;
    }
    for circuit in &contract.circuits {
        located(circuit.source.as_ref(), || {
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
            Ok(())
        })?;
    }
    for witness in &contract.witnesses {
        located(witness.source.as_ref(), || {
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
            Ok(())
        })?;
    }
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
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
            if let ir::StateReturn::Expression { value }
            | ir::StateReturn::HistoricMerkleCheckRoot { root: value, .. }
            | ir::StateReturn::MerkleCheckRoot { root: value, .. } = &circuit.return_value
            {
                collect_expression_types(value, &mut struct_definitions, &mut enum_definitions)?;
            }
            if let ir::StateReturn::Effectful { body } = &circuit.return_value {
                collect_return_plan_types(body, &mut struct_definitions, &mut enum_definitions)?;
            }
            for action in &circuit.actions {
                collect_action_types(action, &mut struct_definitions, &mut enum_definitions)?;
            }
            Ok(())
        })?;
    }
    for field in &contract.ledger_fields {
        match &field.declaration {
            LedgerFieldKind::Cell { ty }
            | LedgerFieldKind::Set { ty }
            | LedgerFieldKind::List { ty } => {
                collect_named_types(ty, &mut struct_definitions, &mut enum_definitions)
                    .map_err(|error| error.at(field.source.as_ref()))?;
            }
            LedgerFieldKind::Map { key, value } => {
                collect_named_types(key, &mut struct_definitions, &mut enum_definitions)
                    .map_err(|error| error.at(field.source.as_ref()))?;
                collect_named_types(value, &mut struct_definitions, &mut enum_definitions)
                    .map_err(|error| error.at(field.source.as_ref()))?;
            }
            LedgerFieldKind::MerkleTree { ty, .. }
            | LedgerFieldKind::HistoricMerkleTree { ty, .. } => {
                collect_named_types(ty, &mut struct_definitions, &mut enum_definitions)
                    .map_err(|error| error.at(field.source.as_ref()))?;
            }
            LedgerFieldKind::Counter => {}
        }
    }
    let mut ledger_fields = HashMap::new();
    let mut ordered_fields = contract.ledger_fields.iter().collect::<Vec<_>>();
    ordered_fields.sort_by_key(|field| field.physical_path());
    for (expected_path, field) in expected_ledger_paths(ordered_fields.len())
        .iter()
        .zip(&ordered_fields)
    {
        let path = field.physical_path();
        if &path != expected_path || path.first() != Some(&field.index) {
            if field.path.is_empty() {
                return Err(RenderError::InvalidLedgerIndex(field.index).at(field.source.as_ref()));
            }
            return Err(RenderError::InvalidLedgerPath(path).at(field.source.as_ref()));
        }
        if path.len() > 2 {
            return Err(RenderError::UnsupportedLedgerPath(path).at(field.source.as_ref()));
        }
        if let LedgerFieldKind::MerkleTree { depth, .. }
        | LedgerFieldKind::HistoricMerkleTree { depth, .. } = field.declaration
            && !(2..=32).contains(&depth)
        {
            return Err(RenderError::InvalidMerkleTreeDepth(depth).at(field.source.as_ref()));
        }
        if ledger_fields.insert(field.id.as_str(), *field).is_some() {
            return Err(
                RenderError::DuplicateLedgerField(field.id.clone()).at(field.source.as_ref())
            );
        }
    }
    let callable_circuits: HashMap<&str, &PureCircuit> = contract
        .circuits
        .iter()
        .map(|circuit| (circuit.name.as_str(), circuit))
        .collect();
    let witness_syntax = witness::build(&contract.witnesses, &ordered_fields)?;
    let mut items = Vec::new();
    for circuit in &contract.circuits {
        let item = located(circuit.source.as_ref(), || {
            let name = ident(&circuit.name)?;
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
            let (body, actual) =
                expression_with_calls(&circuit.body, &parameters, &callable_circuits)?;
            if actual != circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual,
                });
            }
            let result = rust_type(&circuit.result)?;
            let unit_body = if actual == Type::Unit {
                unit_statements(&circuit.body, &parameters, &callable_circuits)?
            } else {
                Vec::new()
            };
            let value_tail: syn::Expr = if let syn::Expr::Try(fallible) = &body {
                (*fallible.expr).clone()
            } else {
                syn::parse_quote!(Ok(#body))
            };
            let item: syn::Item = if circuit.internal && actual == Type::Unit {
                syn::parse_quote! {
                    pub(crate) fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                        #(#unit_body)*
                        Ok(())
                    }
                }
            } else if actual == Type::Unit {
                syn::parse_quote! {
                    pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                        #(#unit_body)*
                        Ok(())
                    }
                }
            } else if circuit.internal {
                syn::parse_quote! {
                    pub(crate) fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                        #value_tail
                    }
                }
            } else {
                syn::parse_quote! {
                    pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                        #value_tail
                    }
                }
            };
            Ok(item)
        })?;
        items.push(item);
    }

    let callable_stateful_circuits: HashMap<&str, &StatefulCircuit> = contract
        .stateful_circuits
        .iter()
        .map(|circuit| (circuit.name.as_str(), circuit))
        .collect();
    let mut stateful_items = Vec::new();
    let mut contract_methods = Vec::new();
    let mut recorded_methods = Vec::new();
    let mut borrowed_recorded_methods = Vec::new();
    let mut witnessed_recorded = false;
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
            stateful_items.push(stateful::render_stateful_circuit(
                circuit,
                &ledger_fields,
                &witness_syntax.declarations,
                &callable_circuits,
                &callable_stateful_circuits,
            )?);
            if let Some(method) =
                stateful::render_contract_method(circuit, &callable_stateful_circuits)?
            {
                contract_methods.push(method);
            }
            Ok(())
        })?;
    }
    let (shared_recorded_callees, mut recorded_items) = recorded::plan_recorded_helpers(
        &contract.stateful_circuits,
        &ledger_fields,
        &witness_syntax.declarations,
        &callable_circuits,
        &callable_stateful_circuits,
    )?;
    let mut proof_capabilities = Vec::new();
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
            let recorded = recorded::render_recorded_circuit(
                circuit,
                &ledger_fields,
                &witness_syntax.declarations,
                &callable_circuits,
                &callable_stateful_circuits,
                &shared_recorded_callees,
            )?;
            let call_name = format!("{}_call", circuit.name);
            let recording_unavailable = recorded.gap().cloned();
            let observed_call = recorded.is_supported()
                && !contract
                    .stateful_circuits
                    .iter()
                    .any(|other| !other.internal && other.name == call_name);
            if !circuit.internal {
                proof_capabilities.push(RustCircuitCapability {
                    name: circuit.name.clone(),
                    source: circuit.source.clone(),
                    recorded: recorded.is_supported(),
                    observed_call,
                    proof_required: None,
                    recording_status: None,
                    recording_unavailable: recording_unavailable.clone(),
                    observed_call_unavailable: if observed_call {
                        None
                    } else if let Some(gap) = recording_unavailable {
                        Some(recorded::RecordingGap::recording_dependency(gap))
                    } else {
                        Some(recorded::RecordingGap::name_collision(&call_name))
                    },
                });
            }
            if let recorded::RecordingOutcome::Supported(item) = recorded {
                recorded_items.push(item);
                let uses_witness = stateful::circuit_uses_witness(
                    circuit,
                    &callable_stateful_circuits,
                    &mut HashSet::new(),
                )?;
                if uses_witness {
                    witnessed_recorded = true;
                } else {
                    recorded_methods.push(recorded::render_recorded_contract_method(circuit)?);
                }
                borrowed_recorded_methods.push(recorded::render_borrowed_recorded_contract_method(
                    circuit,
                    uses_witness,
                )?);
                if observed_call {
                    let observed = recorded::render_observed_call_method(circuit, uses_witness)?;
                    if !uses_witness {
                        recorded_methods.push(observed.clone());
                    }
                    borrowed_recorded_methods.push(observed);
                }
            }
            Ok(())
        })?;
    }

    let mut constructor_args = Vec::<syn::FnArg>::new();
    let mut constructor_parameters = HashMap::new();
    let mut constructor_values = HashMap::<&str, syn::Expr>::new();
    let constructor_uses_witness = if let Some(constructor) = &contract.constructor {
        let mut uses_witness = false;
        for step in &constructor.steps {
            uses_witness |= constructor_step_uses_witness(step, &callable_stateful_circuits)
                .map_err(|error| error.at(constructor.source.as_ref()))?;
        }
        uses_witness
    } else {
        false
    };
    let constructor_uses_vm = constructor_uses_witness
        || contract.constructor.as_ref().is_some_and(|constructor| {
            let mut seen_cells = HashSet::new();
            constructor.steps.iter().any(|step| match step {
                ConstructorStep::CellWrite { field, value, .. } => {
                    !seen_cells.insert(field.as_str())
                        || stateful::expression_contains_stateful_call(
                            value,
                            &callable_stateful_circuits,
                        )
                }
                ConstructorStep::Expression { .. }
                | ConstructorStep::Let { .. }
                | ConstructorStep::Sequence { .. }
                | ConstructorStep::Assert { .. }
                | ConstructorStep::CounterIncrement { .. }
                | ConstructorStep::CounterDecrement { .. }
                | ConstructorStep::CounterReset { .. }
                | ConstructorStep::SetInsert { .. }
                | ConstructorStep::SetRemove { .. }
                | ConstructorStep::SetReset { .. }
                | ConstructorStep::ListPushFront { .. }
                | ConstructorStep::ListPopFront { .. }
                | ConstructorStep::ListReset { .. }
                | ConstructorStep::MapInsert { .. }
                | ConstructorStep::MapInsertDefault { .. }
                | ConstructorStep::MapRemove { .. }
                | ConstructorStep::MapReset { .. }
                | ConstructorStep::ForEach { .. }
                | ConstructorStep::ForEachVector { .. }
                | ConstructorStep::If { .. } => true,
            })
        });
    let mut constructor_actions = Vec::<syn::Stmt>::new();
    let mut constructor_preparations = Vec::<syn::Stmt>::new();
    if let Some(constructor) = &contract.constructor {
        located(constructor.source.as_ref(), || {
            for (index, parameter) in constructor.parameters.iter().enumerate() {
                ident(&parameter.name)?;
                let name = syn::Ident::new(
                    &format!("__compact_constructor_param_{index}"),
                    Span::call_site(),
                );
                if constructor_parameters
                    .insert(parameter.name.as_str(), (&parameter.ty, name.clone()))
                    .is_some()
                {
                    return Err(RenderError::DuplicateParameter(parameter.name.clone()));
                }
                let ty = rust_type(&parameter.ty)?;
                constructor_args.push(syn::parse_quote!(#name: #ty));
            }
            if constructor_uses_vm {
                constructor_actions = render_constructor_vm_steps(
                    &constructor.steps,
                    &ledger_fields,
                    &constructor_parameters,
                    &witness_syntax.declarations,
                    &callable_circuits,
                    &callable_stateful_circuits,
                    &mut 0,
                    &mut 0,
                )?;
            } else {
                for step in &constructor.steps {
                    let ConstructorStep::CellWrite {
                        field,
                        index,
                        value,
                    } = step
                    else {
                        unreachable!("VM constructor selection includes effects and loops")
                    };
                    let declaration = ledger_fields
                        .get(field.as_str())
                        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                    if declaration.index != *index {
                        return Err(RenderError::InvalidLedgerIndex(*index));
                    }
                    let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                        return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                    };
                    let (value, actual) =
                        expression_with_calls(value, &constructor_parameters, &callable_circuits)?;
                    if actual != *ty {
                        return Err(RenderError::TypeMismatch {
                            expected: ty.clone(),
                            actual,
                        });
                    }
                    let name = syn::Ident::new(
                        &format!(
                            "__compact_constructor_value_{}",
                            constructor_preparations.len()
                        ),
                        Span::call_site(),
                    );
                    constructor_preparations.push(syn::parse_quote!(let #name = #value;));
                    constructor_values.insert(field.as_str(), syn::parse_quote!(#name));
                }
            }
            Ok(())
        })?;
    }
    let constructor_fields = ordered_fields.iter().map(|field| match &field.declaration {
        LedgerFieldKind::Counter => Ok(syn::parse_quote!(runtime::ledger::constructor_counter())),
        LedgerFieldKind::Set { .. } => Ok(syn::parse_quote!(runtime::ledger::constructor_set())),
        LedgerFieldKind::List { .. } => Ok(syn::parse_quote!(runtime::ledger::constructor_list())),
        LedgerFieldKind::Map { .. } => Ok(syn::parse_quote!(runtime::ledger::constructor_map())),
        LedgerFieldKind::MerkleTree { depth, .. } => {
            let depth = syn::LitInt::new(&format!("{depth}u8"), Span::call_site());
            Ok(syn::parse_quote!(runtime::ledger::constructor_merkle_tree(#depth)))
        }
        LedgerFieldKind::HistoricMerkleTree { depth, .. } => {
            let depth = syn::LitInt::new(&format!("{depth}u8"), Span::call_site());
            Ok(syn::parse_quote!(runtime::ledger::constructor_historic_merkle_tree(#depth)))
        }
        LedgerFieldKind::Cell { ty } => {
            if !matches!(ty, Type::Boolean | Type::Field | Type::JubjubPoint | Type::OpaqueString | Type::OpaqueBytes | Type::Unsigned { .. } | Type::Bytes { .. } | Type::Struct { .. } | Type::Enum { .. } | Type::Vector { .. } | Type::Tuple { .. } | Type::Unit) {
                return Err(RenderError::UnsupportedLedgerCellType(ty.clone()));
            }
            let value = constructor_values
                .get(field.id.as_str())
                .map(|value| retained_value(value.clone(), ty))
                .unwrap_or_else(|| syn::parse_quote!(Default::default()));
            let ty = rust_type(ty)?;
            Ok(syn::parse_quote!(runtime::ledger::constructor_cell::<#ty, runtime::ledger::DefaultDB>(#value)))
        }
    }).collect::<Result<Vec<syn::Expr>, RenderError>>()?;

    let ledger_view_methods = &witness_syntax.ledger_view_methods;
    let witness_methods = &witness_syntax.trait_methods;
    let constructor_return: syn::Expr = if constructor_uses_vm {
        let transcript_init: Option<syn::Stmt> = constructor_uses_witness
            .then(|| syn::parse_quote!(let mut private_transcript_outputs = Vec::new();));
        let transcript_finish: Option<syn::Stmt> = constructor_uses_witness
            .then(|| syn::parse_quote!(let _ = private_transcript_outputs;));
        syn::parse_quote!({
            let mut context = runtime::context::ConstructorResult::new(__compact_context, state)
                .into_circuit_context(runtime::ledger::ContractAddress::default());
            let mut total_cost = runtime::context::RunningCost::default();
            #transcript_init
            #(#constructor_actions)*
            let _ = total_cost;
            #transcript_finish
            Ok(context.into_constructor_result())
        })
    } else {
        syn::parse_quote!(Ok(runtime::context::ConstructorResult::new(
            __compact_context,
            state
        )))
    };

    let runtime_abi = syn::LitInt::new(&RUNTIME_ABI_VERSION.to_string(), Span::call_site());
    let initial_state: syn::Item = if constructor_uses_witness {
        syn::parse_quote! {
            pub fn initial_state<Private, W: TryWitnesses<Private>>(
                __compact_context: runtime::context::ConstructorContext<Private>,
                witnesses: &W,
                #(#constructor_args),*
            ) -> Result<runtime::context::ConstructorResult<Private>, runtime::CompactError> {
                #(#constructor_preparations)*
                let state = runtime::ledger::contract_state(vec![#(#constructor_fields),*]);
                #constructor_return
            }
        }
    } else {
        syn::parse_quote! {
            pub fn initial_state<Private>(
                __compact_context: runtime::context::ConstructorContext<Private>,
                #(#constructor_args),*
            ) -> Result<runtime::context::ConstructorResult<Private>, runtime::CompactError> {
                #(#constructor_preparations)*
                let state = runtime::ledger::contract_state(vec![#(#constructor_fields),*]);
                #constructor_return
            }
        }
    };
    let borrowed_recorded_handle = witnessed_recorded.then(|| {
        quote! {
            /// A recording handle with access to the contract's witnesses.
            pub struct BorrowedContract<'a, W> {
                pub(super) witnesses: &'a W,
            }
            impl<W> BorrowedContract<'_, W> {
                #(#borrowed_recorded_methods)*
            }
        }
    });
    let recorded_module: Option<syn::Item> = if recorded_items.is_empty() {
        None
    } else {
        Some(syn::parse_quote! {
            /// Circuits with a replayable ordered ledger program.
            pub mod recorded {
                use midnight_compact_runtime as runtime;
                #(#recorded_items)*
                /// Typed handle for circuits with a complete recorded trace.
                pub struct Contract;
                impl Contract {
                    #(#recorded_methods)*
                }
                #borrowed_recorded_handle
            }
        })
    };
    let recording_field =
        (!recorded_items.is_empty()).then(|| quote!(pub recording: recorded::Contract,));
    let recording_init =
        (!recorded_items.is_empty()).then(|| quote!(recording: recorded::Contract,));
    let recording_name_is_exported = contract
        .stateful_circuits
        .iter()
        .any(|circuit| !circuit.internal && circuit.name == "recording");
    let recording_method: Option<syn::ImplItemFn> = if witnessed_recorded {
        Some(syn::parse_quote! {
            /// Borrow the contract's witnesses for a replayable circuit call.
            pub fn recording(&self) -> recorded::BorrowedContract<'_, W> {
                recorded::BorrowedContract { witnesses: &self.witnesses }
            }
        })
    } else if !recorded_items.is_empty() && !recording_name_is_exported {
        Some(syn::parse_quote! {
            /// Access replayable circuit calls for this contract.
            pub fn recording(&self) -> &recorded::Contract {
                &self.recording
            }
        })
    } else {
        None
    };
    let mut slot_items = Vec::<syn::Item>::new();
    let mut public_state_getters = Vec::<syn::ImplItemFn>::new();
    for field in &contract.ledger_fields {
        let name = ident(&field.id)?;
        let path = field.physical_path();
        match &field.declaration {
            LedgerFieldKind::Cell { ty } => {
                let ty = rust_type(ty)?;
                public_state_getters.push(syn::parse_quote! {
                    /// Decode this declared Cell from the borrowed public state.
                    pub fn #name(&self) -> Result<#ty, runtime::CompactError> {
                        crate::ledger_slots::#name.inspect(self.state)
                    }
                });
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::CellSlot<#ty> =
                        runtime::slots::CellSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::Counter => {
                public_state_getters.push(syn::parse_quote! {
                    /// Decode this declared Counter as Compact Uint<64>.
                    pub fn #name(&self) -> Result<runtime::BoundedUint<{u64::MAX as u128}>, runtime::CompactError> {
                        let value = crate::ledger_slots::#name.inspect(self.state)?;
                        Ok(runtime::BoundedUint::<{u64::MAX as u128}>::new(value as u128)
                            .expect("ledger Counter fits Uint<64>"))
                    }
                });
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::CounterSlot =
                        runtime::slots::CounterSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::Set { ty } => {
                let ty = rust_type(ty)?;
                public_state_getters.push(syn::parse_quote! {
                    /// Inspect this declared Set in the borrowed public state.
                    pub fn #name(&self) -> Result<runtime::ledger::SetView<'a, #ty, D>, runtime::CompactError> {
                        crate::ledger_slots::#name.inspect(self.state)
                    }
                });
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::SetSlot<#ty> =
                        runtime::slots::SetSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::List { ty } => {
                let ty = rust_type(ty)?;
                public_state_getters.push(syn::parse_quote! {
                    /// Inspect this declared List in the borrowed public state.
                    pub fn #name(&self) -> Result<runtime::ledger::ListView<'a, #ty, D>, runtime::CompactError> {
                        crate::ledger_slots::#name.inspect(self.state)
                    }
                });
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::ListSlot<#ty> =
                        runtime::slots::ListSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::Map { key, value } => {
                // Nested Map values are structural markers, not CellValue.
                let has_cell_value = !matches!(value, Type::LedgerMap { .. });
                let Some((key, value)) = map_slot_types(key, value)? else {
                    continue;
                };
                if has_cell_value {
                    public_state_getters.push(syn::parse_quote! {
                        /// Inspect this declared Map in the borrowed public state.
                        pub fn #name(&self) -> Result<runtime::ledger::MapView<'a, #key, #value, D>, runtime::CompactError> {
                            crate::ledger_slots::#name.inspect(self.state)
                        }
                    });
                }
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::MapSlot<#key, #value> =
                        runtime::slots::MapSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::MerkleTree { ty, depth }
            | LedgerFieldKind::HistoricMerkleTree { ty, depth } => {
                let leaf = rust_type(ty)?;
                let historic = matches!(
                    &field.declaration,
                    LedgerFieldKind::HistoricMerkleTree { .. }
                );
                if historic {
                    public_state_getters.push(syn::parse_quote! {
                        /// Inspect this declared historic Merkle tree in the borrowed public state.
                        pub fn #name(&self) -> Result<runtime::slots::HistoricMerkleStateView<'a, #leaf, #depth, D>, runtime::CompactError> {
                            crate::ledger_slots::#name.inspect(self.state)
                        }
                    });
                } else {
                    public_state_getters.push(syn::parse_quote! {
                        /// Inspect this declared plain Merkle tree in the borrowed public state.
                        pub fn #name(&self) -> Result<runtime::slots::PlainMerkleStateView<'a, #leaf, #depth, D>, runtime::CompactError> {
                            crate::ledger_slots::#name.inspect(self.state)
                        }
                    });
                }
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::MerkleSlot<#leaf, #depth, #historic> =
                        runtime::slots::MerkleSlot::new(&[#(#path),*]);
                });
            }
        }
    }
    let slots_module: Option<syn::Item> = (!slot_items.is_empty()).then(|| {
        syn::parse_quote! {
            /// Typed descriptors for Compact ledger declarations.
            #[allow(non_upper_case_globals)]
            pub mod ledger_slots {
                use midnight_compact_runtime as runtime;
                #(#slot_items)*
            }
        }
    });
    let type_declarations::TypeDeclarations {
        module: types_module,
        alias_exports,
    } = type_declarations::render(
        &struct_definitions,
        &enum_definitions,
        &contract.type_aliases,
        slots_module.is_some(),
    )?;
    let public_state_view = (!public_state_getters.is_empty()).then(|| {
        quote! {
            /// Read-only projection of an existing ledger-8 public state.
            pub struct PublicStateView<'a, D: runtime::ledger::DB = runtime::ledger::DefaultDB> {
                state: &'a runtime::ledger::StateValue<D>,
            }
            impl<'a, D: runtime::ledger::DB> PublicStateView<'a, D> {
                #(#public_state_getters)*
            }
            impl<'a, S: runtime::public_state::PublicStateSource>
                From<&'a S> for PublicStateView<'a, S::Database> {
                fn from(source: &'a S) -> Self {
                    Self { state: source.public_state() }
                }
            }
        }
    });
    let ledger_module: Option<syn::Item> = if contract.ledger_fields.is_empty()
        && contract.constructor.is_none()
        && contract.stateful_circuits.is_empty()
        && contract.witnesses.is_empty()
    {
        None
    } else {
        Some(syn::parse_quote! {
            #[allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables)]
            pub mod ledger_contract {
                    use midnight_compact_runtime as runtime;
                    const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
                    pub struct LedgerView<'a> {
                        #[allow(dead_code)]
                        state: &'a runtime::ledger::StateValue<runtime::ledger::DefaultDB>,
                        #[allow(dead_code)]
                        meter: &'a runtime::context::WitnessReadMeter<'a>,
                    }
                    impl<'a> LedgerView<'a> {
                        #(#ledger_view_methods)*
                    }
                    #public_state_view
                    /// Implement for infallible callbacks; use TryWitnesses for fallible ledger reads.
                    #[runtime::compact_witness_bridge]
                    pub trait Witnesses<Private> {
                    #(#witness_methods)*
                }
                #initial_state
                #(#stateful_items)*
                #recorded_module
                /// Groups the contract's exported circuits for Rust consumers.
                pub struct Contract<W> {
                    #[allow(dead_code)]
                    witnesses: W,
                    #recording_field
                }
                impl<W> From<W> for Contract<W> {
                    fn from(witnesses: W) -> Self {
                        Self { witnesses, #recording_init }
                    }
                }
                impl Default for Contract<()> {
                    fn default() -> Self {
                        Self { witnesses: (), #recording_init }
                    }
                }
                impl<W> Contract<W> {
                    #(#contract_methods)*
                    #recording_method
                }
            }
        })
    };
    let mut file: syn::File = syn::parse2(quote! {
        /// The matching Midnight Compact Rust runtime used by this generated crate.
        pub use midnight_compact_runtime as runtime;
        #types_module
        #alias_exports
        #[allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables)]
        pub mod pure_circuits {
            use midnight_compact_runtime as runtime;
            const _: () = assert!(runtime::RUST_RUNTIME_ABI == #runtime_abi);
            #(#items)*
        }
        #slots_module
        #ledger_module
    })
    .expect("typed renderer constructed invalid Rust syntax");
    CompactSignatureLint.visit_file_mut(&mut file);
    CompactBooleanLiterals.visit_file_mut(&mut file);
    Ok(RenderedContract {
        source: format!(
            "{GENERATED_HEADER}// Generated by compactc. Do not edit.\n\n{}",
            prettyplease::unparse(&file)
        ),
        capabilities: RustCapabilityReport {
            // The compiler finalizes the report from contract-info.json before publication.
            schema_version: 2,
            circuits: proof_capabilities,
        },
    })
}
