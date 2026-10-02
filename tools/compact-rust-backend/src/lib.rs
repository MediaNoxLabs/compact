//! Rust syntax construction from Compact's typed backend IR.

pub mod ir;
mod recorded;
mod stateful;
mod witness;

const RUNTIME_ABI_VERSION: u32 = 4;

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

pub(crate) fn scalar_map_slot_types(
    key: &Type,
    value: &Type,
) -> Result<Option<(syn::Type, syn::Type)>, RenderError> {
    let key = match rust_type(key) {
        Ok(ty) => ty,
        Err(RenderError::UnsupportedLedgerValueType(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let value = match rust_type(value) {
        Ok(ty) => ty,
        Err(RenderError::UnsupportedLedgerValueType(_)) => return Ok(None),
        Err(error) => return Err(error),
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
        Expr::SetMember { value, .. } => collect_expression_types(value, structs, enums)?,
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
        Expr::UnsignedCast { value, .. } | Expr::FieldCast { value } => {
            collect_expression_types(value, structs, enums)?
        }
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
        Expr::KernelSelf { ty } => collect_named_types(ty, structs, enums)?,
        Expr::SetIsEmpty { .. } | Expr::MapIsEmpty { .. } | Expr::CellRead { .. } => {}
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
        StateAction::CounterIncrement { .. }
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

fn copy_type(ty: &Type) -> bool {
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
            Ok((
                syn::parse_quote!((#value).#name.clone()),
                declaration.ty.clone(),
            ))
        }
        Expr::TupleIndex { value, index } => {
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            let Type::Tuple { elements } = ty else {
                return Err(RenderError::ExpectedTuple(ty));
            };
            let result = elements
                .get(*index)
                .ok_or_else(|| RenderError::InvalidTupleIndex(*index))?
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
            let mut field_names = Vec::with_capacity(fields.len());
            let mut field_values = Vec::with_capacity(fields.len());
            for (value, declaration) in fields.iter().zip(declarations) {
                let (rendered, actual) = expression_with_calls(value, parameters, circuits)?;
                if actual != declaration.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: declaration.ty.clone(),
                        actual,
                    });
                }
                field_names.push(ident(&declaration.name)?);
                field_values.push(rendered);
            }
            let name = ident(name)?;
            Ok((
                syn::parse_quote!(crate::types::#name { #(#field_names: #field_values),* }),
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
            Ok((
                syn::parse_quote!({
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
                let (rendered, actual) = expression_with_calls(step, parameters, circuits)?;
                if actual != Type::Unit {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Unit,
                        actual,
                    });
                }
                statements.push(syn::parse_quote!(#rendered;));
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
        | Expr::SetMember { .. }
        | Expr::MapMember { .. }
        | Expr::MapLookup { .. }
        | Expr::MerkleCheckRoot { .. }
        | Expr::HistoricMerkleCheckRoot { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::CellRead { .. }
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
                if !matches!(actual, Type::Unsigned { .. }) {
                    return Err(RenderError::ExpectedUnsigned(actual));
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
        ConstructorStep::CounterIncrement { .. }
        | ConstructorStep::CounterDecrement { .. }
        | ConstructorStep::CounterReset { .. }
        | ConstructorStep::SetReset { .. }
        | ConstructorStep::ListPopFront { .. }
        | ConstructorStep::ListReset { .. }
        | ConstructorStep::MapReset { .. } => Ok(false),
    }
}

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
                let (value, _, _) = stateful::render_state_expression(
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
                actions.push(syn::parse_quote!(let _ = #value;));
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
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                actions.extend(expression_steps);
                let path = declaration.physical_path();
                let write: syn::Stmt = if path.len() == 1 {
                    syn::parse_quote!(let step = context.write_cell(#index, (#value).clone())?;)
                } else {
                    let path = path
                        .iter()
                        .map(|part| syn::LitInt::new(&part.to_string(), Span::call_site()))
                        .collect::<Vec<_>>();
                    syn::parse_quote!(let step = context.write_cell_at_path(&[#(#path),*], (#value).clone())?;)
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
                let method = if matches!(step, ConstructorStep::SetInsert { .. }) {
                    syn::Ident::new("insert_set", Span::call_site())
                } else {
                    syn::Ident::new("remove_set", Span::call_site())
                };
                let index = ledger_path_expr(declaration);
                actions.push(
                    syn::parse_quote!(let step = context.#method(#index, (#value).clone())?;),
                );
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
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                actions.push(syn::parse_quote!(let step = context.push_front_list(#index, (#value).clone())?;));
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
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
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
                let index = ledger_path_expr(declaration);
                actions.push(syn::parse_quote!(let step = context.insert_map(#index, (#key).clone(), (#value).clone())?;));
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
                let index = ledger_path_expr(declaration);
                if matches!(step, ConstructorStep::MapInsertDefault { .. }) {
                    let value_ty = rust_type(value_ty)?;
                    actions.push(syn::parse_quote!(let step = context.insert_map(#index, (#key).clone(), <#value_ty as Default>::default())?;));
                } else {
                    actions.push(
                        syn::parse_quote!(let step = context.remove_map(#index, (#key).clone())?;),
                    );
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
                actions.push(syn::parse_quote!(let #iterable: [#ty; #len] = [#((#rendered_values).clone()),*];));
                actions.push(syn::parse_quote!(for #item in #iterable { #(#body)* }));
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

pub fn render(contract: &Contract) -> Result<String, RenderError> {
    if contract.schema_version != SCHEMA_VERSION {
        return Err(RenderError::SchemaVersion(contract.schema_version));
    }

    let mut names = HashSet::new();
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
        if path.len() > 2
            || (path.len() > 1 && matches!(field.declaration, LedgerFieldKind::List { .. }))
        {
            return Err(RenderError::UnsupportedLedgerPath(path).at(field.source.as_ref()));
        }
        if let LedgerFieldKind::MerkleTree { depth, .. }
        | LedgerFieldKind::HistoricMerkleTree { depth, .. } = field.declaration
        {
            if !(2..=32).contains(&depth) {
                return Err(RenderError::InvalidMerkleTreeDepth(depth).at(field.source.as_ref()));
            }
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
            let (body, actual) =
                expression_with_calls(&circuit.body, &parameters, &callable_circuits)?;
            if actual != circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual,
                });
            }
            let result = rust_type(&circuit.result)?;
            let item: syn::Item = if circuit.internal {
                syn::parse_quote! {
                    pub(crate) fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                        Ok(#body)
                    }
                }
            } else {
                syn::parse_quote! {
                    pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                        Ok(#body)
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
    let mut recorded_items = Vec::new();
    let mut recorded_methods = Vec::new();
    let mut borrowed_recorded_methods = Vec::new();
    let mut witnessed_recorded = false;
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
            if !names.insert(circuit.name.as_str()) {
                return Err(RenderError::DuplicateCircuit(circuit.name.clone()));
            }
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
            if let Some(item) = recorded::render_recorded_circuit(
                circuit,
                &ledger_fields,
                &witness_syntax.declarations,
                &callable_circuits,
                &callable_stateful_circuits,
            )? {
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
                | ConstructorStep::ForEach { .. } => true,
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
            let ty = rust_type(ty)?;
            let value: syn::Expr = constructor_values.get(field.id.as_str())
                .map(|value| syn::parse_quote!(#value.clone()))
                .unwrap_or_else(|| syn::parse_quote!(Default::default()));
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
    let mut struct_items = Vec::<syn::Item>::new();
    let has_merkle_path = struct_definitions
        .keys()
        .any(|name| is_compact_struct_instantiation(name, "MerkleTreePath"));
    for (name, fields) in &struct_definitions {
        let empty = fields.is_empty();
        let conversion = if has_merkle_path {
            if is_compact_struct_instantiation(name, "MerkleTreeDigest") {
                Some("CompactMerkleTreeDigest")
            } else if is_compact_struct_instantiation(name, "MerkleTreePathEntry") {
                Some("CompactMerklePathEntry")
            } else if is_compact_struct_instantiation(name, "MerkleTreePath") {
                Some("CompactMerklePath")
            } else {
                None
            }
        } else {
            None
        };
        let name = ident(name)?;
        let fields = fields
            .iter()
            .map(|field| {
                let field_name = ident(&field.name)?;
                let field_ty = rust_type(&field.ty)?;
                Ok(syn::parse_quote!(pub #field_name: #field_ty))
            })
            .collect::<Result<Vec<syn::Field>, RenderError>>()?;
        let mut item: syn::ItemStruct = if empty {
            syn::parse_quote! {
                #[derive(Clone, Debug, Default, PartialEq, Eq, CompactCellValue, BinaryHashRepr, FieldRepr)]
                pub struct #name {}
            }
        } else {
            syn::parse_quote! {
                #[derive(Clone, Debug, Default, PartialEq, Eq, CompactCellValue, BinaryHashRepr, FieldRepr, FromFieldRepr)]
                pub struct #name { #(#fields),* }
            }
        };
        if let Some(conversion) = conversion {
            let conversion = syn::Ident::new(conversion, Span::call_site());
            item.attrs
                .push(syn::parse_quote!(#[derive(runtime::#conversion)]));
        }
        struct_items.push(syn::Item::Struct(item));
        if empty {
            struct_items.push(syn::parse_quote! {
                impl FromFieldRepr for #name {
                    const FIELD_SIZE: usize = 0;

                    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
                        repr.is_empty().then_some(Self {})
                    }
                }
            });
        }
    }
    for (name, variants) in &enum_definitions {
        let name = ident(name)?;
        let variants = variants
            .iter()
            .map(|variant| ident(variant))
            .collect::<Result<Vec<_>, _>>()?;
        struct_items.push(syn::parse_quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue, CompactEnum)]
            pub enum #name { #(#variants),* }
        });
    }
    let mut alias_names = HashSet::new();
    let mut alias_reexports = Vec::new();
    for alias in &contract.type_aliases {
        located(alias.source.as_ref(), || {
            if !alias_names.insert(alias.name.as_str())
                || struct_definitions.contains_key(&alias.name)
                || enum_definitions.contains_key(&alias.name)
                || matches!(
                    alias.name.as_str(),
                    "runtime" | "types" | "pure_circuits" | "ledger_contract"
                )
            {
                return Err(RenderError::ConflictingTypeAlias(alias.name.clone()));
            }
            let name = ident(&alias.name)?;
            let ty = rust_type(&alias.ty)?;
            struct_items
                .push(syn::parse_quote!(#[allow(non_camel_case_types)] pub type #name = #ty;));
            alias_reexports.push(name);
            Ok(())
        })?;
    }
    let mut derive_imports = Vec::<syn::Item>::new();
    if !struct_definitions.is_empty() {
        // The upstream struct derives expand with unqualified Fr and MemWrite.
        derive_imports.push(syn::parse_quote!(
            use runtime::{BinaryHashRepr, CompactCellValue, FieldRepr, Fr, FromFieldRepr, MemWrite};
        ));
        if !enum_definitions.is_empty() {
            derive_imports.push(syn::parse_quote!(
                use runtime::CompactEnum;
            ));
        }
    } else if !enum_definitions.is_empty() {
        derive_imports.push(syn::parse_quote!(
            use runtime::{CompactCellValue, CompactEnum};
        ));
    }
    let types_module: Option<syn::Item> = if struct_items.is_empty() {
        None
    } else {
        Some(syn::parse_quote! {
            #[allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables)]
            pub mod types {
                use midnight_compact_runtime as runtime;
                #(#derive_imports)*
                #(#struct_items)*
            }
        })
    };
    let alias_exports: Option<syn::Item> = if alias_reexports.is_empty() {
        None
    } else {
        Some(syn::parse_quote!(pub use types::{#(#alias_reexports),*};))
    };
    let initial_state: syn::Item = if constructor_uses_witness {
        syn::parse_quote! {
            pub fn initial_state<Private, W: Witnesses<Private>>(
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
    let borrowed_recording_method: Option<syn::ImplItemFn> = witnessed_recorded.then(|| {
        syn::parse_quote! {
            /// Borrow the contract's witnesses for a replayable circuit call.
            pub fn recording(&self) -> recorded::BorrowedContract<'_, W> {
                recorded::BorrowedContract { witnesses: &self.witnesses }
            }
        }
    });
    let mut slot_items = Vec::<syn::Item>::new();
    for field in &contract.ledger_fields {
        let name = ident(&field.id)?;
        let path = field.physical_path();
        match &field.declaration {
            LedgerFieldKind::Cell { ty } => {
                let ty = rust_type(ty)?;
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::CellSlot<#ty> =
                        runtime::slots::CellSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::Counter => slot_items.push(syn::parse_quote! {
                pub const #name: runtime::slots::CounterSlot =
                    runtime::slots::CounterSlot::new(&[#(#path),*]);
            }),
            LedgerFieldKind::Set { ty } => {
                let ty = rust_type(ty)?;
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::SetSlot<#ty> =
                        runtime::slots::SetSlot::new(&[#(#path),*]);
                });
            }
            LedgerFieldKind::List { ty } => {
                let ty = rust_type(ty)?;
                let index = field.index;
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::ListSlot<#ty> =
                        runtime::slots::ListSlot::new(#index);
                });
            }
            LedgerFieldKind::Map { key, value } => {
                // Nested ledger maps have no CellValue representation yet.
                // Keep their constructor/native support without advertising
                // scalar MapSlot operations that cannot type-check.
                let Some((key, value)) = scalar_map_slot_types(key, value)? else {
                    continue;
                };
                slot_items.push(syn::parse_quote! {
                    pub const #name: runtime::slots::MapSlot<#key, #value> =
                        runtime::slots::MapSlot::new(&[#(#path),*]);
                });
            }
            _ => {}
        }
    }
    let slots_module: Option<syn::Item> = (!slot_items.is_empty()).then(|| {
        syn::parse_quote! {
            /// Typed descriptors for Compact Cell, Counter, Set, Map, and List declarations.
            #[allow(non_upper_case_globals)]
            pub mod ledger_slots {
                use midnight_compact_runtime as runtime;
                #(#slot_items)*
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
                    }
                    impl<'a> LedgerView<'a> {
                        #(#ledger_view_methods)*
                    }
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
                    #borrowed_recording_method
                }
            }
        })
    };
    let file: syn::File = syn::parse2(quote! {
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
    Ok(format!(
        "{GENERATED_HEADER}// Generated by compactc. Do not edit.\n\n{}",
        prettyplease::unparse(&file)
    ))
}
