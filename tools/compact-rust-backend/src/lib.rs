//! Rust syntax construction from Compact's typed backend IR.

mod capabilities;
mod circuit_analysis;
mod coin_shapes;
pub mod compatibility;
pub mod ir;
mod resource_limits;
pub use capabilities::{
    RUST_CAPABILITY_SCHEMA_VERSION, RecordingStatus, RustCapabilityReport, RustCircuitCapability,
};
mod constructor_vm;
mod hash_input;
mod implicit_types;
mod jubjub_value;
mod naming;
mod native_frame;
mod native_header;
mod native_representation;
mod origins;
mod source_diagnostics;
pub use origins::diagnostic::{SourceDiagnostic, SourcePosition};
pub use origins::frontend::render::{
    FrontendRender, NativeCompileFailure, NativeFailureCause, RecordingRequirement, RenderOptions,
};
pub use origins::frontend::{
    ConfiguredFrontend, FrontendArtifacts, FrontendError, FrontendRequest, InvocationFingerprint,
};
pub use origins::recording::RecordingDiagnostic;
pub use origins::status::{
    BindingSummary, MetadataProblem, MetadataStatus, MetadataUnavailable, SourceSnapshotFingerprint,
};
mod pure_native;
mod pure_semantics;
mod recorded;
pub use recorded::{
    RecordingGap, RecordingGapCode, RecordingResource, RecordingResourceKind,
    RecordingResourcePhase,
};
mod stateful;
mod type_declarations;
mod value_lowering;
mod witness;

const RUNTIME_ABI_VERSION: u32 = 52;

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
    ConstructorStep, Contract, CounterAmount, Expr, LedgerFieldKind, PureCircuit, SCHEMA_VERSION,
    StateAction, StatefulCircuit, StructField, Type,
};
use naming::{ident, semantic_identifier, validate_circuit_function_namespace};
use proc_macro2::Span;
use pure_native::fragment::NativeFragment;
use quote::quote;
use serde_json::Value;
use source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use syn::visit_mut::{self, VisitMut};
// Preserve the internal entry points while typed-value rules have one owner.
use value_lowering::{
    UnsignedMaximum, coerce_expression, field_literal_bytes, field_to_bytes_32_syntax,
    map_slot_types, retained_value, rust_type, unsigned_arithmetic_syntax, unsigned_cast_syntax,
    unsigned_maximum,
};
#[cfg(test)]
use value_lowering::{copy_type, wide_uint_type};

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

/// A rendering refusal. Downstream matches must retain a fallback arm so new
/// diagnostics can be added within this backend compatibility line. Display
/// text is intended for people, not a stable machine-readable protocol.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderError {
    /// Bounded compiler support policy was exceeded. The resource label and
    /// Display text are human diagnostics, not a machine-readable protocol.
    ResourceLimit {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
    Located {
        location: ir::SourceLocation,
        error: Box<RenderError>,
    },
    RecordingPlanInvariant {
        path: String,
        detail: String,
    },
    SchemaVersion(u32),
    ProofApplicability(String),
    InvalidIdentifier(String),
    InvalidUnsignedMaximum(String),
    /// The bound is valid, but this operation is not supported by the wide carrier.
    UnsupportedWideUnsignedOperation {
        operation: &'static str,
        max: String,
    },
    /// An operation requires a trait the pinned Rust tuple carrier cannot provide.
    UnsupportedTupleOperation {
        operation: &'static str,
        arity: usize,
        limit: usize,
        required_trait: &'static str,
    },
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
    /// A pure call graph supplied through private IR contains a cycle.
    RecursivePureCall(String),
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
            Self::ResourceLimit {
                resource,
                limit,
                observed,
            } => write!(
                f,
                "compiler resource {resource} exceeds {limit} (observed {observed})"
            ),
            Self::Located { location, error } => write!(
                f,
                "{} line {} char {}: {}",
                location.file, location.line, location.column, error
            ),
            Self::RecordingPlanInvariant { path, detail } => {
                write!(f, "recording plan invariant at {path}: {detail}")
            }
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
                    "invalid Compact Uint maximum {max:?}; expected a canonical decimal integer from 0 through 2^248 - 1"
                )
            }
            Self::UnsupportedWideUnsignedOperation { operation, max } => write!(
                f,
                "unsupported wide Compact Uint {operation} for maximum {max:?}"
            ),
            Self::UnsupportedTupleOperation {
                operation,
                arity,
                limit,
                required_trait,
            } => write!(
                f,
                "{operation} requires {required_trait}; tuple arity {arity} exceeds the supported maximum {limit}"
            ),
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
            Self::RecursivePureCall(name) => {
                write!(f, "recursive pure circuit call involving {name:?}")
            }
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

/// Name public wrapper arguments after their Compact parameters. Wrapper
/// locals and names that normalize to the same Rust identifier need stable
/// positional fallbacks; the underlying circuit still receives arguments in
/// declaration order.
pub(crate) fn public_parameter_idents(parameters: &[ir::Parameter]) -> Vec<syn::Ident> {
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
    naming::parameter_idents(parameters, &reserved)
}

#[cfg(test)]
mod public_parameter_name_tests {
    use super::{ir, public_parameter_idents};

    fn allocated(names: &[&str]) -> Vec<String> {
        let parameters = names
            .iter()
            .map(|name| ir::Parameter {
                name: (*name).into(),
                ty: ir::Type::Field,
            })
            .collect::<Vec<_>>();
        public_parameter_idents(&parameters)
            .into_iter()
            .map(|name| name.to_string())
            .collect()
    }

    #[test]
    fn raw_and_plain_parameters_share_one_rust_binding_namespace() {
        for names in [
            ["r#foo", "foo"],
            ["foo", "r#foo"],
            ["type", "r#type"],
            ["r#type", "type"],
        ] {
            let expected_first = if names[0] == "type" {
                "r#type"
            } else {
                names[0]
            };
            assert_eq!(
                allocated(&names),
                [expected_first, "__compact_param_1"],
                "{names:?}"
            );
        }
    }

    #[test]
    fn raw_fallback_names_and_reserved_parameters_cannot_alias() {
        assert_eq!(
            allocated(&[
                "r#__compact_param_1",
                "context",
                "__compact_param_1_1",
                "r#context"
            ]),
            [
                "r#__compact_param_1",
                "__compact_param_1_1",
                "__compact_param_2",
                "__compact_param_3"
            ]
        );
    }

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

// Repeated references to one raw source type are valid. Distinct source names
// must not collapse into the same Rust type when `$`/raw spellings normalize.
fn has_named_type_collision(
    name: &str,
    structs: &BTreeMap<String, Vec<StructField>>,
    enums: &BTreeMap<String, Vec<String>>,
) -> Result<bool, RenderError> {
    let emitted = semantic_identifier(name)?;
    for other in structs.keys().chain(enums.keys()) {
        if other != name && semantic_identifier(other)? == emitted {
            return Ok(true);
        }
    }
    Ok(false)
}

// Originless compatibility entry used by synthesized and recorded type environments.
fn collect_named_types(
    ty: &Type,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), RenderError> {
    collect_named_types_observed(ty, structs, enums, None, None)
}

fn collect_named_types_with_origins<'ir>(
    ty: &'ir Type,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: Option<&mut type_declarations::DefinitionOrigins<'ir>>,
) -> Result<(), RenderError> {
    collect_named_types_observed(ty, structs, enums, None, origins)
}
fn named_collection_refused(observer: Option<&dyn RefusalObserver>, site: RefusalSite<'_>) {
    if let Some(observer) = observer {
        observer.refused(site);
    }
}

fn collect_named_types_observed<'ir>(
    ty: &'ir Type,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    observer: Option<&dyn RefusalObserver>,
    mut origins: Option<&mut type_declarations::DefinitionOrigins<'ir>>,
) -> Result<(), RenderError> {
    match ty {
        Type::Struct { name, fields } => {
            ident(name).inspect_err(|_| {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(Owner::Type(ty), "name"));
                }
            })?;
            if enums.contains_key(name) {
                named_collection_refused(observer, RefusalSite::node(Owner::Type(ty)));
                return Err(RenderError::ConflictingStruct(name.clone()));
            }
            if let Some(existing) = structs.get(name) {
                if existing != fields {
                    named_collection_refused(observer, RefusalSite::node(Owner::Type(ty)));
                    return Err(RenderError::ConflictingStruct(name.clone()));
                }
            } else {
                if has_named_type_collision(name, structs, enums)? {
                    named_collection_refused(observer, RefusalSite::node(Owner::Type(ty)));
                    return Err(RenderError::ConflictingStruct(name.clone()));
                }
                let mut names = HashSet::new();
                for field in fields {
                    ident(&field.name).inspect_err(|_| {
                        if let Some(observer) = observer {
                            observer.refused(RefusalSite::field(Owner::StructField(field), "name"));
                        }
                    })?;
                    if !names.insert(field.name.as_str()) {
                        named_collection_refused(
                            observer,
                            RefusalSite::field(Owner::StructField(field), "name"),
                        );
                        return Err(RenderError::DuplicateStructField(field.name.clone()));
                    }
                }
                structs.insert(name.clone(), fields.clone());
                if let Some(origins) = origins.as_deref_mut() {
                    origins.insert(name.clone(), ty);
                }
            }
            for field in fields {
                collect_named_types_observed(
                    &field.ty,
                    structs,
                    enums,
                    observer,
                    origins.as_deref_mut(),
                )?;
            }
        }
        Type::Enum { name, variants } => {
            ident(name).inspect_err(|_| {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(Owner::Type(ty), "name"));
                }
            })?;
            if structs.contains_key(name) {
                named_collection_refused(observer, RefusalSite::node(Owner::Type(ty)));
                return Err(RenderError::ConflictingEnum(name.clone()));
            }
            if variants.is_empty() {
                named_collection_refused(observer, RefusalSite::field(Owner::Type(ty), "variants"));
                return Err(RenderError::EmptyEnum(name.clone()));
            }
            let mut names = HashSet::new();
            for (index, variant) in variants.iter().enumerate() {
                ident(variant).inspect_err(|_| {
                    if let Some(observer) = observer {
                        observer.refused(RefusalSite::element(Owner::Type(ty), "variants", index));
                    }
                })?;
                if !names.insert(variant.as_str()) {
                    named_collection_refused(
                        observer,
                        RefusalSite::element(Owner::Type(ty), "variants", index),
                    );
                    return Err(RenderError::DuplicateEnumVariant(variant.clone()));
                }
            }
            if let Some(existing) = enums.get(name) {
                if existing != variants {
                    named_collection_refused(observer, RefusalSite::node(Owner::Type(ty)));
                    return Err(RenderError::ConflictingEnum(name.clone()));
                }
            } else {
                if has_named_type_collision(name, structs, enums)? {
                    named_collection_refused(observer, RefusalSite::node(Owner::Type(ty)));
                    return Err(RenderError::ConflictingEnum(name.clone()));
                }
                enums.insert(name.clone(), variants.clone());
                if let Some(origins) = origins.as_deref_mut() {
                    origins.insert(name.clone(), ty);
                }
            }
        }
        Type::Tuple { elements } => {
            for element in elements {
                collect_named_types_observed(
                    element,
                    structs,
                    enums,
                    observer,
                    origins.as_deref_mut(),
                )?;
            }
        }
        Type::Vector { element, .. } => {
            collect_named_types_observed(element, structs, enums, observer, origins.as_deref_mut())?
        }
        Type::LedgerMap { key, value } => {
            collect_named_types_observed(key, structs, enums, observer, origins.as_deref_mut())?;
            collect_named_types_observed(value, structs, enums, observer, origins)?;
        }
        _ => {}
    }
    Ok(())
}

fn collect_expression_types<'ir>(
    expr: &'ir Expr,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), RenderError> {
    match expr {
        Expr::Default { ty } | Expr::EnumVariant { ty, .. } => {
            collect_named_types_observed(ty, structs, enums, observer, Some(origins))?
        }
        Expr::StructField { value, .. } | Expr::TupleIndex { value, .. } => {
            collect_expression_types(value, structs, enums, origins, observer)?
        }
        Expr::StructLiteral { ty, fields } => {
            collect_named_types_observed(ty, structs, enums, observer, Some(origins))?;
            for field in fields {
                collect_expression_types(field, structs, enums, origins, observer)?;
            }
        }
        Expr::Tuple { elements } => {
            for element in elements {
                collect_expression_types(element, structs, enums, origins, observer)?;
            }
        }
        Expr::Vector { element, elements } => {
            collect_named_types_observed(element, structs, enums, observer, Some(origins))?;
            for value in elements {
                collect_expression_types(value, structs, enums, origins, observer)?;
            }
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            ..
        } => {
            collect_named_types_observed(&parameter.ty, structs, enums, observer, Some(origins))?;
            collect_named_types_observed(result, structs, enums, observer, Some(origins))?;
            collect_expression_types(source, structs, enums, origins, observer)?;
            collect_expression_types(body, structs, enums, origins, observer)?;
        }
        Expr::VectorFoldCall {
            initial,
            source,
            accumulator,
            element,
            ..
        } => {
            collect_named_types_observed(accumulator, structs, enums, observer, Some(origins))?;
            collect_named_types_observed(element, structs, enums, observer, Some(origins))?;
            collect_expression_types(initial, structs, enums, origins, observer)?;
            collect_expression_types(source, structs, enums, origins, observer)?;
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums, origins, observer)?;
            collect_expression_types(then, structs, enums, origins, observer)?;
            collect_expression_types(otherwise, structs, enums, origins, observer)?;
        }
        Expr::Let { bindings, body } => {
            for binding in bindings {
                collect_named_types_observed(&binding.ty, structs, enums, observer, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins, observer)?;
            }
            collect_expression_types(body, structs, enums, origins, observer)?;
        }
        Expr::Sequence { steps, value } => {
            for step in steps {
                collect_expression_types(step, structs, enums, origins, observer)?;
            }
            collect_expression_types(value, structs, enums, origins, observer)?;
        }
        Expr::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums, origins, observer)?;
        }
        Expr::SetMember { value, .. }
        | Expr::CounterLessThan {
            threshold: value, ..
        } => collect_expression_types(value, structs, enums, origins, observer)?,
        Expr::MapMember { key, .. } | Expr::MapLookup { key, .. } => {
            collect_expression_types(key, structs, enums, origins, observer)?
        }
        Expr::MerkleCheckRoot { root, .. } | Expr::HistoricMerkleCheckRoot { root, .. } => {
            collect_expression_types(root, structs, enums, origins, observer)?
        }
        Expr::Call { arguments, .. } | Expr::WitnessCall { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums, origins, observer)?;
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
            collect_expression_types(value, structs, enums, origins, observer)?
        }
        Expr::EcMulGenerator { scalar } => {
            collect_expression_types(scalar, structs, enums, origins, observer)?
        }
        Expr::EcMul { point, scalar } => {
            collect_expression_types(point, structs, enums, origins, observer)?;
            collect_expression_types(scalar, structs, enums, origins, observer)?;
        }
        Expr::ConstructJubjubPoint { x, y } => {
            collect_expression_types(x, structs, enums, origins, observer)?;
            collect_expression_types(y, structs, enums, origins, observer)?;
        }
        Expr::TransientCommit { value, opening } | Expr::PersistentCommit { value, opening } => {
            collect_expression_types(value, structs, enums, origins, observer)?;
            collect_expression_types(opening, structs, enums, origins, observer)?;
        }
        Expr::UnsignedCast { value, .. }
        | Expr::FieldCast { value }
        | Expr::FieldToBytes32 { value } => {
            collect_expression_types(value, structs, enums, origins, observer)?
        }
        Expr::Coerce { value, ty } => {
            collect_expression_types(value, structs, enums, origins, observer)?;
            collect_named_types_observed(ty, structs, enums, observer, Some(origins))?;
        }
        Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => {
            collect_expression_types(left, structs, enums, origins, observer)?;
            collect_expression_types(right, structs, enums, origins, observer)?;
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::Equal { left, right }
        | Expr::NotEqual { left, right }
        | Expr::Compare { left, right, .. }
        | Expr::EcAdd { left, right } => {
            collect_expression_types(left, structs, enums, origins, observer)?;
            collect_expression_types(right, structs, enums, origins, observer)?;
        }
        Expr::Unit
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::Parameter { .. } => {}
        Expr::KernelClaim { value, .. } => {
            collect_expression_types(value, structs, enums, origins, observer)?
        }
        Expr::KernelMintShielded { domain, amount } => {
            collect_expression_types(domain, structs, enums, origins, observer)?;
            collect_expression_types(amount, structs, enums, origins, observer)?;
        }
        Expr::CreateZswapInput { coin } => {
            collect_expression_types(coin, structs, enums, origins, observer)?
        }
        Expr::CreateZswapOutput { coin, recipient } => {
            collect_expression_types(coin, structs, enums, origins, observer)?;
            collect_expression_types(recipient, structs, enums, origins, observer)?;
        }
        Expr::NativeWitnessCall { builtin } => {
            collect_named_types_with_origins(&builtin.result_type(), structs, enums, None)
                .inspect_err(|_| named_collection_refused(observer, RefusalSite::unavailable()))?;
        }
        Expr::KernelSelf { ty } | Expr::ListHead { ty, .. } => {
            collect_named_types_observed(ty, structs, enums, observer, Some(origins))?
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

fn collect_action_types<'ir>(
    action: &'ir StateAction,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), RenderError> {
    match action {
        StateAction::Sequence { actions } => {
            for action in actions {
                collect_action_types(action, structs, enums, origins, observer)?;
            }
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums, origins, observer)?;
            collect_action_types(then, structs, enums, origins, observer)?;
            collect_action_types(otherwise, structs, enums, origins, observer)?;
        }
        StateAction::PureCall { arguments, .. } | StateAction::CircuitCall { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums, origins, observer)?;
            }
        }
        StateAction::Expression { value } => {
            collect_expression_types(value, structs, enums, origins, observer)?;
        }
        StateAction::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums, origins, observer)?;
        }
        StateAction::Let { bindings, action } => {
            for binding in bindings {
                collect_named_types_observed(&binding.ty, structs, enums, observer, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins, observer)?;
            }
            collect_action_types(action, structs, enums, origins, observer)?;
        }
        StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. } => {
            collect_expression_types(value, structs, enums, origins, observer)?;
        }
        StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        } => {
            collect_expression_types(coin, structs, enums, origins, observer)?;
            collect_expression_types(recipient, structs, enums, origins, observer)?;
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
            collect_expression_types(value, structs, enums, origins, observer)?;
            collect_expression_types(position, structs, enums, origins, observer)?;
        }
        StateAction::MapInsert { key, value, .. } => {
            collect_expression_types(key, structs, enums, origins, observer)?;
            collect_expression_types(value, structs, enums, origins, observer)?;
        }
        StateAction::MapInsertDefault { key, .. } | StateAction::MapRemove { key, .. } => {
            collect_expression_types(key, structs, enums, origins, observer)?;
        }
        StateAction::MerkleInsertIndexDefault { position, .. }
        | StateAction::HistoricMerkleInsertIndexDefault { position, .. } => {
            collect_expression_types(position, structs, enums, origins, observer)?;
        }
        StateAction::NativeWitnessCall { builtin } => {
            collect_named_types_with_origins(&builtin.result_type(), structs, enums, None)
                .inspect_err(|_| named_collection_refused(observer, RefusalSite::unavailable()))?;
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

fn collect_return_plan_types<'ir>(
    plan: &'ir ir::ReturnPlan,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), RenderError> {
    match plan {
        ir::ReturnPlan::Value { value } => {
            collect_expression_types(value, structs, enums, origins, observer)
        }
        ir::ReturnPlan::Sequence { actions, result } => {
            for action in actions {
                collect_action_types(action, structs, enums, origins, observer)?;
            }
            collect_return_plan_types(result, structs, enums, origins, observer)
        }
        ir::ReturnPlan::Let { bindings, result } => {
            for binding in bindings {
                collect_named_types_observed(&binding.ty, structs, enums, observer, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins, observer)?;
            }
            collect_return_plan_types(result, structs, enums, origins, observer)
        }
        ir::ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums, origins, observer)?;
            collect_return_plan_types(then, structs, enums, origins, observer)?;
            collect_return_plan_types(otherwise, structs, enums, origins, observer)
        }
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

/// Complete generic fold; native consumers retain expression and Unit views.
fn native_fragment_with_calls(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(NativeFragment, Type), RenderError> {
    let typed = pure_semantics::expression::fold_expression(
        expr,
        parameters,
        circuits,
        &mut pure_native::sink::NativeSink::new(),
    )?;
    Ok((typed.output, typed.ty))
}

/// Only configured frontend artifacts supply this borrowed pure-body observer.
/// Recursive traversal remains the same generic fold and native consumer.
fn native_fragment_with_refusal(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
    callback: &mut pure_native::sink::RefusalCallback<'_>,
) -> Result<(NativeFragment, Type), RenderError> {
    let typed = pure_semantics::expression::fold_expression(
        expr,
        parameters,
        circuits,
        &mut pure_native::sink::NativeSink::with_refusal_callback(callback),
    )?;
    Ok((typed.output, typed.ty))
}

fn expression_with_calls(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    let typed = pure_semantics::expression::fold_expression(
        expr,
        parameters,
        circuits,
        &mut pure_native::sink::NativeSink::new(),
    )?;
    Ok((typed.output.into_expression(), typed.ty))
}

#[cfg(test)]
fn pure_aggregate_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    use pure_semantics::aggregate::{AggregateSource, fold_aggregate};
    let source = match expr {
        Expr::StructField {
            value,
            field,
            index,
        } => AggregateSource::StructField {
            value,
            field,
            index: *index,
        },
        Expr::TupleIndex { value, index } => AggregateSource::TupleIndex {
            value,
            index: *index,
        },
        Expr::StructLiteral { ty, fields } => AggregateSource::StructLiteral { ty, fields },
        Expr::Tuple { elements } => AggregateSource::Tuple { elements },
        Expr::Vector { element, elements } => AggregateSource::Vector { element, elements },
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => AggregateSource::VectorMap {
            parameter,
            source,
            body,
            result,
            length: *length,
        },
        Expr::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => AggregateSource::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length: *length,
        },
        Expr::Call { name, arguments } => AggregateSource::Call { name, arguments },
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    };
    let typed = fold_aggregate(
        source,
        &mut pure_native::children::NativeChildren::new(parameters, circuits),
        circuits,
        &mut pure_native::leaf::NativeLeafSink::new(),
    )?;
    Ok((typed.output, typed.ty))
}

#[cfg(test)]
fn pure_conversion_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    use pure_semantics::conversion::{ConversionSource, fold_conversion};
    let source = match expr {
        Expr::FieldCast { value } => ConversionSource::FieldCast { value },
        Expr::FieldToBytes32 { value } => ConversionSource::FieldToBytes32 { value },
        Expr::Coerce { value, ty } => ConversionSource::Coerce { value, ty },
        Expr::UnsignedCast { max, value } => ConversionSource::UnsignedCast { max, value },
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    };
    let typed = fold_conversion(
        source,
        &mut pure_native::children::NativeChildren::new(parameters, circuits),
        &mut pure_native::leaf::NativeLeafSink::new(),
    )?;
    Ok((typed.output, typed.ty))
}

#[cfg(test)]
fn pure_arithmetic_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    use pure_semantics::{
        arithmetic::{ArithmeticSource, fold_arithmetic},
        numeric::ArithmeticKind,
    };
    let source = match expr {
        Expr::UnsignedAdd { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Add,
            max,
            left,
            right,
        },
        Expr::UnsignedSubtract { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Subtract,
            max,
            left,
            right,
        },
        Expr::UnsignedMultiply { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Multiply,
            max,
            left,
            right,
        },
        Expr::Add { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Add,
            left,
            right,
        },
        Expr::Subtract { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Subtract,
            left,
            right,
        },
        Expr::Multiply { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Multiply,
            left,
            right,
        },
        Expr::Equal { left, right } => ArithmeticSource::Equality {
            negated: false,
            left,
            right,
        },
        Expr::NotEqual { left, right } => ArithmeticSource::Equality {
            negated: true,
            left,
            right,
        },
        Expr::Compare {
            operator,
            left,
            right,
        } => ArithmeticSource::Ordered {
            operator: *operator,
            left,
            right,
        },
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    };
    let typed = fold_arithmetic(
        source,
        &mut pure_native::children::NativeChildren::new(parameters, circuits),
        &mut pure_native::leaf::NativeLeafSink::new(),
    )?;
    Ok((typed.output, typed.ty))
}

#[cfg(test)]
fn pure_crypto_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    use pure_semantics::crypto::{CryptoSource, fold_crypto};
    let source = match expr {
        Expr::TransientHash { value } => CryptoSource::TransientHash(value),
        Expr::TransientCommit { value, opening } => CryptoSource::TransientCommit(value, opening),
        Expr::PersistentHash { value } => CryptoSource::PersistentHash(value),
        Expr::PersistentCommit { value, opening } => CryptoSource::PersistentCommit(value, opening),
        Expr::Keccak256 { value } => CryptoSource::Keccak256(value),
        Expr::DegradeToTransient { value } => CryptoSource::DegradeToTransient(value),
        Expr::UpgradeFromTransient { value } => CryptoSource::UpgradeFromTransient(value),
        Expr::HashToCurve { value } => CryptoSource::HashToCurve(value),
        Expr::JubjubPointX { value } => CryptoSource::JubjubPointX(value),
        Expr::JubjubPointY { value } => CryptoSource::JubjubPointY(value),
        Expr::ConstructJubjubPoint { x, y } => CryptoSource::ConstructJubjubPoint(x, y),
        Expr::EcAdd { left, right } => CryptoSource::EcAdd(left, right),
        Expr::EcNeg { value } => CryptoSource::EcNeg(value),
        Expr::EcMul { point, scalar } => CryptoSource::EcMul(point, scalar),
        Expr::EcMulGenerator { scalar } => CryptoSource::EcMulGenerator(scalar),
        Expr::JubjubScalarFromNative { value } => CryptoSource::JubjubScalarFromNative(value),
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    };
    let typed = fold_crypto(
        source,
        &mut pure_native::children::NativeChildren::new(parameters, circuits),
        &mut pure_native::leaf::NativeLeafSink::new(),
    )?;
    Ok((typed.output, typed.ty))
}

#[cfg(test)]
fn pure_leaf_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    _circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    use pure_semantics::leaf::{LeafSource, fold_leaf};
    let source = match expr {
        Expr::Unit => LeafSource::Unit,
        Expr::Default { ty } => LeafSource::Default(ty),
        Expr::Boolean { value } => LeafSource::Boolean(*value),
        Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
        Expr::BytesLiteral { bytes } => LeafSource::BytesLiteral(bytes),
        Expr::UnsignedLiteral { value, max } => LeafSource::UnsignedLiteral { value, max },
        Expr::Parameter { name } => LeafSource::Parameter(name),
        Expr::EnumVariant { ty, variant } => LeafSource::EnumVariant { ty, variant },
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
        | Expr::KernelSelf { .. } => pure_semantics::leaf::LeafSource::Effectful,
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    };
    let typed = fold_leaf(
        source,
        parameters,
        &mut pure_native::leaf::NativeLeafSink::new(),
    )?;
    Ok((typed.output, typed.ty))
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

fn collect_constructor_step_types<'ir>(
    step: &'ir ConstructorStep,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(), RenderError> {
    match step {
        ConstructorStep::Expression { value } => {
            collect_expression_types(value, structs, enums, origins, observer)?;
        }
        ConstructorStep::Let { bindings, step } => {
            for binding in bindings {
                collect_named_types_observed(&binding.ty, structs, enums, observer, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins, observer)?;
            }
            collect_constructor_step_types(step, structs, enums, origins, observer)?;
        }
        ConstructorStep::Sequence { steps } => {
            for step in steps {
                collect_constructor_step_types(step, structs, enums, origins, observer)?;
            }
        }
        ConstructorStep::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums, origins, observer)?
        }
        ConstructorStep::CellWrite { value, .. }
        | ConstructorStep::SetInsert { value, .. }
        | ConstructorStep::SetRemove { value, .. }
        | ConstructorStep::ListPushFront { value, .. } => {
            collect_expression_types(value, structs, enums, origins, observer)?
        }
        ConstructorStep::MapInsert { key, value, .. } => {
            collect_expression_types(key, structs, enums, origins, observer)?;
            collect_expression_types(value, structs, enums, origins, observer)?;
        }
        ConstructorStep::MapInsertDefault { key, .. } | ConstructorStep::MapRemove { key, .. } => {
            collect_expression_types(key, structs, enums, origins, observer)?
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
            collect_named_types_observed(&binding.ty, structs, enums, observer, Some(origins))?;
            for value in values {
                collect_expression_types(value, structs, enums, origins, observer)?;
            }
            for step in steps {
                collect_constructor_step_types(step, structs, enums, origins, observer)?;
            }
        }
        ConstructorStep::ForEachVector {
            binding,
            source,
            steps,
            ..
        } => {
            collect_named_types_observed(&binding.ty, structs, enums, observer, Some(origins))?;
            collect_expression_types(source, structs, enums, origins, observer)?;
            for step in steps {
                collect_constructor_step_types(step, structs, enums, origins, observer)?;
            }
        }
        ConstructorStep::If {
            condition,
            then_steps,
            otherwise_steps,
        } => {
            collect_expression_types(condition, structs, enums, origins, observer)?;
            for step in then_steps.iter().chain(otherwise_steps) {
                collect_constructor_step_types(step, structs, enums, origins, observer)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
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

/// Keep the original witness-first constructor selection; observe only propagated graph failures.
fn constructor_step_uses_witness_observed(
    step: &ConstructorStep,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<bool, RenderError> {
    let requires = |value: &Expr| {
        stateful::expression_requires_witness_observed(value, stateful_circuits, observer)
    };
    match step {
        ConstructorStep::Expression { value } => requires(value),
        ConstructorStep::Let { bindings, step } => {
            for binding in bindings {
                if requires(&binding.value)? {
                    return Ok(true);
                }
            }
            constructor_step_uses_witness_observed(step, stateful_circuits, observer)
        }
        ConstructorStep::Sequence { steps } => {
            for step in steps {
                if constructor_step_uses_witness_observed(step, stateful_circuits, observer)? {
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
                if constructor_step_uses_witness_observed(step, stateful_circuits, observer)? {
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
                if constructor_step_uses_witness_observed(step, stateful_circuits, observer)? {
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
                if constructor_step_uses_witness_observed(step, stateful_circuits, observer)? {
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
    let environment = stateful::NativeExpressionContext {
        witnesses,
        circuits,
        stateful_circuits,
        ledger_fields,
        observer: None,
    };
    render_constructor_vm_steps_observed(
        steps,
        parameters,
        &environment,
        constructor_vm::Output {
            next_loop,
            next_temp,
        },
    )
}

/// Preserve source-step recursion while observing existing operand/name failures.
fn render_constructor_vm_steps_observed<'a>(
    steps: &'a [ConstructorStep],
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    environment: &stateful::NativeExpressionContext<'_, '_>,
    output: constructor_vm::Output<'_>,
) -> Result<Vec<syn::Stmt>, RenderError> {
    let constructor_vm::Output {
        next_loop,
        next_temp,
    } = output;
    let ledger_fields = environment.ledger_fields;
    let mut actions = Vec::new();
    for step in steps {
        match step {
            ConstructorStep::Expression { value } => {
                let mut expression_steps = Vec::new();
                let mut query_effect = false;
                let (value, ty, _) = stateful::render_state_expression_observed(
                    value,
                    parameters,
                    &mut expression_steps,
                    next_temp,
                    &mut query_effect,
                    environment,
                )?;
                actions.extend(expression_steps);
                actions.extend(discard_expression(value, &ty));
            }
            ConstructorStep::Let { bindings, step } => {
                let mut locals = parameters.clone();
                for binding in bindings {
                    native_header::identifier(
                        &binding.name,
                        RefusalSite::field(Owner::Binding(binding), "name"),
                        environment.observer,
                    )?;
                    let mut expression_steps = Vec::new();
                    let mut query_effect = false;
                    let (value, actual, _witness_effect) =
                        stateful::render_state_expression_observed(
                            &binding.value,
                            &locals,
                            &mut expression_steps,
                            next_temp,
                            &mut query_effect,
                            environment,
                        )?;
                    if actual != binding.ty {
                        constructor_vm::refused(
                            RefusalSite::node(Owner::Expr(&binding.value)),
                            environment.observer,
                        );
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
                    let ty = native_header::carrier(&binding.ty, environment.observer)?;
                    actions.push(syn::parse_quote!(let #local_name: #ty = #value;));
                    locals.insert(binding.name.as_str(), (&binding.ty, local_name));
                }
                actions.extend(render_constructor_vm_steps_observed(
                    std::slice::from_ref(step.as_ref()),
                    &locals,
                    environment,
                    constructor_vm::Output {
                        next_loop,
                        next_temp,
                    },
                )?);
            }
            ConstructorStep::Sequence { steps } => {
                actions.extend(render_constructor_vm_steps_observed(
                    steps,
                    parameters,
                    environment,
                    constructor_vm::Output {
                        next_loop,
                        next_temp,
                    },
                )?);
            }
            ConstructorStep::Assert { condition, message } => {
                let original_condition = condition;
                let mut expression_steps = Vec::new();
                let mut query_effect = false;
                let (condition, actual, _witness_effect) =
                    stateful::render_state_expression_observed(
                        condition,
                        parameters,
                        &mut expression_steps,
                        next_temp,
                        &mut query_effect,
                        environment,
                    )?;
                if actual != Type::Boolean {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_condition)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.index != *index {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "index"),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                };
                let mut expression_steps = Vec::new();
                let mut query_effect = false;
                let original_value = value;
                let (value, actual, _witness_effect) = stateful::render_state_expression_observed(
                    value,
                    parameters,
                    &mut expression_steps,
                    next_temp,
                    &mut query_effect,
                    environment,
                )?;
                if actual != *ty {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_value)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::ConstructorStep(step)),
                        environment.observer,
                    );
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let amount: syn::Expr = match amount {
                    CounterAmount::Literal { value } => {
                        let value = syn::LitInt::new(&format!("{value}u16"), Span::call_site());
                        syn::parse_quote!(#value)
                    }
                    CounterAmount::Parameter { name } => {
                        let (ty, rust_name) = parameters.get(name.as_str()).ok_or_else(|| {
                            constructor_vm::refused(
                                RefusalSite::field(Owner::CounterAmount(amount), "name"),
                                environment.observer,
                            );
                            RenderError::UnknownParameter(name.clone())
                        })?;
                        let expected = Type::Unsigned {
                            max: "65535".into(),
                        };
                        if *ty != &expected {
                            constructor_vm::refused(
                                RefusalSite::field(Owner::CounterAmount(amount), "name"),
                                environment.observer,
                            );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::ConstructorStep(step)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.index != *index {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "index"),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(value) {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(value)),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let original_value = value;
                let (value, actual) =
                    constructor_vm::pure_expression(value, parameters, environment.observer)?;
                if actual != *ty {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_value)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::ConstructorStep(step)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.index != *index {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "index"),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::List { ty } = &declaration.declaration else {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(value) {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(value)),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let original_value = value;
                let (value, actual) =
                    constructor_vm::pure_expression(value, parameters, environment.observer)?;
                if actual != *ty {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_value)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if !matches!(declaration.declaration, LedgerFieldKind::List { .. })
                    || declaration.index != *index
                {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::ConstructorStep(step)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.index != *index {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "index"),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(key) || !infallible_constructor_expr(value) {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::ConstructorStep(step)),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let original_key = key;
                let (key, actual_key) =
                    constructor_vm::pure_expression(key, parameters, environment.observer)?;
                if actual_key != *key_ty {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_key)),
                        environment.observer,
                    );
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let original_value = value;
                let (value, actual_value) =
                    constructor_vm::pure_expression(value, parameters, environment.observer)?;
                if actual_value != *value_ty {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_value)),
                        environment.observer,
                    );
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
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if declaration.index != *index {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "index"),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidLedgerIndex(*index));
                }
                let LedgerFieldKind::Map {
                    key: key_ty,
                    value: value_ty,
                } = &declaration.declaration
                else {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if !infallible_constructor_expr(key) {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(key)),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                }
                let original_key = key;
                let (key, actual_key) =
                    constructor_vm::pure_expression(key, parameters, environment.observer)?;
                if actual_key != *key_ty {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_key)),
                        environment.observer,
                    );
                    return Err(RenderError::TypeMismatch {
                        expected: key_ty.clone(),
                        actual: actual_key,
                    });
                }
                let key = retained_value(key, key_ty);
                let index = ledger_path_expr(declaration);
                if matches!(step, ConstructorStep::MapInsertDefault { .. }) {
                    let value_ty = native_header::carrier(value_ty, environment.observer)?;
                    actions.push(syn::parse_quote!(let step = context.insert_map(#index, #key, <#value_ty as Default>::default())?;));
                } else {
                    actions.push(syn::parse_quote!(let step = context.remove_map(#index, #key)?;));
                }
                actions.push(syn::parse_quote!(context = step.context;));
            }
            ConstructorStep::MapReset { field, index } => {
                let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::field(Owner::ConstructorStep(step), "field"),
                        environment.observer,
                    );
                    RenderError::UnknownLedgerField(field.clone())
                })?;
                if !matches!(declaration.declaration, LedgerFieldKind::Map { .. })
                    || declaration.index != *index
                {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::ConstructorStep(step)),
                        environment.observer,
                    );
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
                        constructor_vm::refused(
                            RefusalSite::node(Owner::Expr(value)),
                            environment.observer,
                        );
                        return Err(RenderError::InvalidConstructorInitializer(
                            binding.name.clone(),
                        ));
                    }
                    let original_value = value;
                    let (value, actual) =
                        constructor_vm::pure_expression(value, parameters, environment.observer)?;
                    if actual != binding.ty {
                        constructor_vm::refused(
                            RefusalSite::node(Owner::Expr(original_value)),
                            environment.observer,
                        );
                        return Err(RenderError::TypeMismatch {
                            expected: binding.ty.clone(),
                            actual,
                        });
                    }
                    rendered_values.push(value);
                }
                let mut body_parameters = parameters.clone();
                body_parameters.insert(binding.name.as_str(), (&binding.ty, item.clone()));
                let body = render_constructor_vm_steps_observed(
                    steps,
                    &body_parameters,
                    environment,
                    constructor_vm::Output {
                        next_loop,
                        next_temp,
                    },
                )?;
                let ty = native_header::carrier(&binding.ty, environment.observer)?;
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
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(source)),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer(
                        binding.name.clone(),
                    ));
                };
                let (actual, iterable) = parameters.get(name.as_str()).ok_or_else(|| {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(source)),
                        environment.observer,
                    );
                    RenderError::UnknownParameter(name.clone())
                })?;
                let expected = Type::Vector {
                    element: Box::new(binding.ty.clone()),
                    length: *length,
                };
                if **actual != expected {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(source)),
                        environment.observer,
                    );
                    return Err(RenderError::TypeMismatch {
                        expected,
                        actual: (*actual).clone(),
                    });
                }
                let mut body_parameters = parameters.clone();
                body_parameters.insert(binding.name.as_str(), (&binding.ty, item.clone()));
                let body = render_constructor_vm_steps_observed(
                    steps,
                    &body_parameters,
                    environment,
                    constructor_vm::Output {
                        next_loop,
                        next_temp,
                    },
                )?;
                actions.push(syn::parse_quote!(for #item in #iterable.0.iter() { #(#body)* }));
            }
            ConstructorStep::If {
                condition,
                then_steps,
                otherwise_steps,
            } => {
                if !infallible_constructor_expr(condition) {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(condition)),
                        environment.observer,
                    );
                    return Err(RenderError::InvalidConstructorInitializer("if".into()));
                }
                let original_condition = condition;
                let (condition, actual) =
                    constructor_vm::pure_expression(condition, parameters, environment.observer)?;
                if actual != Type::Boolean {
                    constructor_vm::refused(
                        RefusalSite::node(Owner::Expr(original_condition)),
                        environment.observer,
                    );
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Boolean,
                        actual,
                    });
                }
                let then_body = render_constructor_vm_steps_observed(
                    then_steps,
                    parameters,
                    environment,
                    constructor_vm::Output {
                        next_loop,
                        next_temp,
                    },
                )?;
                let otherwise_body = render_constructor_vm_steps_observed(
                    otherwise_steps,
                    parameters,
                    environment,
                    constructor_vm::Output {
                        next_loop,
                        next_temp,
                    },
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
    render_with_native_refusal(contract, None, None)
}

/// Private common renderer. Originless public APIs never discover metadata.
fn render_with_native_refusal(
    contract: &Contract,
    refusal_callback: Option<&mut pure_native::sink::RefusalCallback<'_>>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<RenderedContract, RenderError> {
    render_with_refusals(contract, refusal_callback, observer, None)
}
fn render_with_refusals(
    contract: &Contract,
    mut refusal_callback: Option<&mut pure_native::sink::RefusalCallback<'_>>,
    observer: Option<&dyn RefusalObserver>,
    recording_observer: Option<&dyn source_diagnostics::RefusalObserver>,
) -> Result<RenderedContract, RenderError> {
    if contract.schema_version != SCHEMA_VERSION {
        return Err(RenderError::SchemaVersion(contract.schema_version));
    }

    let preflight = match observer {
        Some(observer) => resource_limits::validate_with_observer(contract, Some(observer))?,
        None => resource_limits::validate(contract)?,
    };

    // Build callable lookup maps only after checking their keys. Otherwise a
    // later declaration silently replaces an earlier one in a HashMap, and a
    // caller can fail with a misleading type error before we reach the
    // duplicate declaration and its Compact source location.
    let mut names = HashSet::new();
    for (name, source, owner) in contract
        .circuits
        .iter()
        .map(|circuit| (&circuit.name, circuit.source.as_ref(), Owner::Pure(circuit)))
        .chain(contract.stateful_circuits.iter().map(|circuit| {
            (
                &circuit.name,
                circuit.source.as_ref(),
                Owner::Stateful(circuit),
            )
        }))
    {
        located(source, || {
            ident(name).inspect_err(|_| {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(owner, "name"));
                }
            })?;
            if !names.insert(name.as_str()) {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(owner, "name"));
                }
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
    let mut definition_origins = type_declarations::DefinitionOrigins::new();
    if let Some(constructor) = &contract.constructor {
        located(constructor.source.as_ref(), || {
            for parameter in &constructor.parameters {
                collect_named_types_observed(
                    &parameter.ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )?;
            }
            for step in &constructor.steps {
                collect_constructor_step_types(
                    step,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    &mut definition_origins,
                    observer,
                )?;
            }
            Ok(())
        })?;
    }
    for circuit in &contract.circuits {
        located(circuit.source.as_ref(), || {
            for parameter in &circuit.parameters {
                collect_named_types_observed(
                    &parameter.ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )?;
            }
            collect_named_types_observed(
                &circuit.result,
                &mut struct_definitions,
                &mut enum_definitions,
                observer,
                Some(&mut definition_origins),
            )?;
            collect_expression_types(
                &circuit.body,
                &mut struct_definitions,
                &mut enum_definitions,
                &mut definition_origins,
                observer,
            )?;
            Ok(())
        })?;
    }
    for witness in &contract.witnesses {
        located(witness.source.as_ref(), || {
            for parameter in &witness.parameters {
                collect_named_types_observed(
                    &parameter.ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )?;
            }
            collect_named_types_observed(
                &witness.result,
                &mut struct_definitions,
                &mut enum_definitions,
                observer,
                Some(&mut definition_origins),
            )?;
            Ok(())
        })?;
    }
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
            for parameter in &circuit.parameters {
                collect_named_types_observed(
                    &parameter.ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )?;
            }
            collect_named_types_observed(
                &circuit.result,
                &mut struct_definitions,
                &mut enum_definitions,
                observer,
                Some(&mut definition_origins),
            )?;
            if let ir::StateReturn::Expression { value }
            | ir::StateReturn::HistoricMerkleCheckRoot { root: value, .. }
            | ir::StateReturn::MerkleCheckRoot { root: value, .. } = &circuit.return_value
            {
                collect_expression_types(
                    value,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    &mut definition_origins,
                    observer,
                )?;
            }
            if let ir::StateReturn::Effectful { body } = &circuit.return_value {
                collect_return_plan_types(
                    body,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    &mut definition_origins,
                    observer,
                )?;
            }
            for action in &circuit.actions {
                collect_action_types(
                    action,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    &mut definition_origins,
                    observer,
                )?;
            }
            Ok(())
        })?;
    }
    let has_merkle_witness_view = implicit_types::collect_witness_view_types(
        contract,
        &mut struct_definitions,
        &mut enum_definitions,
    )
    .inspect_err(|_| named_collection_refused(observer, RefusalSite::unavailable()))?;
    for field in &contract.ledger_fields {
        match &field.declaration {
            LedgerFieldKind::Cell { ty }
            | LedgerFieldKind::Set { ty }
            | LedgerFieldKind::List { ty } => {
                collect_named_types_observed(
                    ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )
                .map_err(|error| error.at(field.source.as_ref()))?;
            }
            LedgerFieldKind::Map { key, value } => {
                collect_named_types_observed(
                    key,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )
                .map_err(|error| error.at(field.source.as_ref()))?;
                collect_named_types_observed(
                    value,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )
                .map_err(|error| error.at(field.source.as_ref()))?;
            }
            LedgerFieldKind::MerkleTree { ty, .. }
            | LedgerFieldKind::HistoricMerkleTree { ty, .. } => {
                collect_named_types_observed(
                    ty,
                    &mut struct_definitions,
                    &mut enum_definitions,
                    observer,
                    Some(&mut definition_origins),
                )
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
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(Owner::Ledger(field), "index"));
                }
                return Err(RenderError::InvalidLedgerIndex(field.index).at(field.source.as_ref()));
            }
            if let Some(observer) = observer {
                observer.refused(RefusalSite::node(Owner::Ledger(field)));
            }
            return Err(RenderError::InvalidLedgerPath(path).at(field.source.as_ref()));
        }
        if path.len() > 2 {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(Owner::Ledger(field), "path"));
            }
            return Err(RenderError::UnsupportedLedgerPath(path).at(field.source.as_ref()));
        }
        if let LedgerFieldKind::MerkleTree { depth, .. }
        | LedgerFieldKind::HistoricMerkleTree { depth, .. } = field.declaration
            && !(2..=32).contains(&depth)
        {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(
                    Owner::LedgerKind(&field.declaration),
                    "depth",
                ));
            }
            return Err(RenderError::InvalidMerkleTreeDepth(depth).at(field.source.as_ref()));
        }
        if ledger_fields.insert(field.id.as_str(), *field).is_some() {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(Owner::Ledger(field), "id"));
            }
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
    let witness_syntax = witness::build(&contract.witnesses, &ordered_fields, observer)?;
    let mut items = Vec::new();
    for circuit in &contract.circuits {
        let item = located(circuit.source.as_ref(), || {
            let name = ident(&circuit.name)?;
            let mut parameters = HashMap::new();
            let mut args = Vec::<syn::FnArg>::new();
            for (parameter, arg_name) in circuit
                .parameters
                .iter()
                .zip(naming::parameter_idents(&circuit.parameters, &[]))
            {
                native_header::identifier(
                    &parameter.name,
                    RefusalSite::field(Owner::Parameter(parameter), "name"),
                    observer,
                )?;
                if parameters
                    .insert(parameter.name.as_str(), (&parameter.ty, arg_name.clone()))
                    .is_some()
                {
                    if let Some(observer) = observer {
                        observer.refused(RefusalSite::field(Owner::Parameter(parameter), "name"));
                    }
                    return Err(RenderError::DuplicateParameter(parameter.name.clone()));
                }
                let arg_ty = native_header::carrier(&parameter.ty, observer)?;
                let arg = pure_native::item::argument(arg_name, arg_ty);
                args.push(arg);
            }
            let mut observed_callback = |expression: &Expr, _: &RenderError| {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::node(Owner::Expr(expression)));
                }
            };
            let (body, actual) = match (observer, refusal_callback.as_mut()) {
                (Some(_), _) => native_fragment_with_refusal(
                    &circuit.body,
                    &parameters,
                    &callable_circuits,
                    &mut observed_callback,
                )?,
                (None, Some(callback)) => native_fragment_with_refusal(
                    &circuit.body,
                    &parameters,
                    &callable_circuits,
                    &mut **callback,
                )?,
                (None, None) => {
                    native_fragment_with_calls(&circuit.body, &parameters, &callable_circuits)?
                }
            };
            if actual != circuit.result {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(Owner::Pure(circuit), "result"));
                }
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual,
                });
            }
            let result = native_header::carrier(&circuit.result, observer)?;
            let item = pure_native::item::pure(
                name,
                circuit.internal,
                args,
                result,
                body,
                actual == Type::Unit,
            );
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
    // Optional facade helpers share the emitted Rust method namespace with
    // source exports, including Compact `$` and Rust raw-identifier spellings.
    let exported_method_names = contract
        .stateful_circuits
        .iter()
        .filter(|circuit| !circuit.internal)
        .map(|circuit| semantic_identifier(&circuit.name))
        .collect::<Result<HashSet<_>, _>>()?;
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
            stateful_items.push(stateful::render_stateful_circuit_observed(
                circuit,
                &ledger_fields,
                &witness_syntax.declarations,
                &callable_circuits,
                &callable_stateful_circuits,
                observer,
            )?);
            if let Some(method) =
                stateful::render_contract_method(circuit, &callable_stateful_circuits)?
            {
                contract_methods.push(method);
            }
            Ok(())
        })?;
    }
    let (shared_recorded_callees, recorded_helpers) = recorded::plan_recorded_helpers(
        &contract.stateful_circuits,
        &ledger_fields,
        &witness_syntax.declarations,
        &callable_circuits,
        &callable_stateful_circuits,
    )?;
    let mut recorded_items = Vec::new();
    let mut proof_capabilities = Vec::new();
    for circuit in &contract.stateful_circuits {
        located(circuit.source.as_ref(), || {
            let recorded = if recording_observer.is_some() {
                recorded::render_recorded_circuit_observed(
                    circuit,
                    &ledger_fields,
                    &witness_syntax.declarations,
                    &callable_circuits,
                    &callable_stateful_circuits,
                    &shared_recorded_callees,
                    recording_observer,
                )
            } else {
                recorded::render_recorded_circuit(
                    circuit,
                    &ledger_fields,
                    &witness_syntax.declarations,
                    &callable_circuits,
                    &callable_stateful_circuits,
                    &shared_recorded_callees,
                )
            }?;
            let call_name = format!("{}_call", circuit.name);
            let recording_unavailable = recorded.gap().cloned();
            let observed_call = recorded.is_supported()
                && !exported_method_names.contains(&semantic_identifier(&call_name)?);
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
            uses_witness |=
                constructor_step_uses_witness_observed(step, &callable_stateful_circuits, observer)
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
                native_header::identifier(
                    &parameter.name,
                    RefusalSite::field(Owner::Parameter(parameter), "name"),
                    observer,
                )?;
                let name = syn::Ident::new(
                    &format!("__compact_constructor_param_{index}"),
                    Span::call_site(),
                );
                if constructor_parameters
                    .insert(parameter.name.as_str(), (&parameter.ty, name.clone()))
                    .is_some()
                {
                    if let Some(observer) = observer {
                        observer.refused(RefusalSite::field(Owner::Parameter(parameter), "name"));
                    }
                    return Err(RenderError::DuplicateParameter(parameter.name.clone()));
                }
                let ty = native_header::carrier(&parameter.ty, observer)?;
                constructor_args.push(syn::parse_quote!(#name: #ty));
            }
            if constructor_uses_vm {
                constructor_actions = if observer.is_some() {
                    let environment = stateful::NativeExpressionContext {
                        witnesses: &witness_syntax.declarations,
                        circuits: &callable_circuits,
                        stateful_circuits: &callable_stateful_circuits,
                        ledger_fields: &ledger_fields,
                        observer,
                    };
                    render_constructor_vm_steps_observed(
                        &constructor.steps,
                        &constructor_parameters,
                        &environment,
                        constructor_vm::Output {
                            next_loop: &mut 0,
                            next_temp: &mut 0,
                        },
                    )
                } else {
                    render_constructor_vm_steps(
                        &constructor.steps,
                        &ledger_fields,
                        &constructor_parameters,
                        &witness_syntax.declarations,
                        &callable_circuits,
                        &callable_stateful_circuits,
                        &mut 0,
                        &mut 0,
                    )
                }?;
            } else {
                for (preparation_index, step) in constructor.steps.iter().enumerate() {
                    let ConstructorStep::CellWrite {
                        field,
                        index,
                        value,
                    } = step
                    else {
                        unreachable!("VM constructor selection includes effects and loops")
                    };
                    let declaration = ledger_fields.get(field.as_str()).ok_or_else(|| {
                        if let Some(observer) = observer {
                            observer
                                .refused(RefusalSite::field(Owner::ConstructorStep(step), "field"));
                        }
                        RenderError::UnknownLedgerField(field.clone())
                    })?;
                    if declaration.index != *index {
                        if let Some(observer) = observer {
                            observer
                                .refused(RefusalSite::field(Owner::ConstructorStep(step), "index"));
                        }
                        return Err(RenderError::InvalidLedgerIndex(*index));
                    }
                    let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                        if let Some(observer) = observer {
                            observer
                                .refused(RefusalSite::field(Owner::ConstructorStep(step), "field"));
                        }
                        return Err(RenderError::InvalidConstructorInitializer(field.clone()));
                    };
                    let original_value = value;
                    let (value, actual) = if let Some(observer) = observer {
                        let mut callback = |expression: &Expr, _: &RenderError| {
                            observer.refused(RefusalSite::node(Owner::Expr(expression)));
                        };
                        let (fragment, actual) = native_fragment_with_refusal(
                            value,
                            &constructor_parameters,
                            &callable_circuits,
                            &mut callback,
                        )?;
                        (fragment.into_expression(), actual)
                    } else {
                        expression_with_calls(value, &constructor_parameters, &callable_circuits)?
                    };
                    if actual != *ty {
                        if let Some(observer) = observer {
                            observer.refused(RefusalSite::node(Owner::Expr(original_value)));
                        }
                        return Err(RenderError::TypeMismatch {
                            expected: ty.clone(),
                            actual,
                        });
                    }
                    let name = syn::Ident::new(
                        &format!("__compact_constructor_value_{preparation_index}"),
                        Span::call_site(),
                    );
                    constructor_preparations.push(syn::parse_quote!(let #name = #value;));
                    constructor_preparations.push(syn::parse_quote!(
                        runtime::ledger::CellValue::validate_cell_value(&#name)?;
                    ));
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
                if let Some(observer) = observer { observer.refused(RefusalSite::node(Owner::Type(ty))); }
                return Err(RenderError::UnsupportedLedgerCellType(ty.clone()));
            }
            let value = constructor_values
                .get(field.id.as_str())
                .map(|value| retained_value(value.clone(), ty))
                .unwrap_or_else(|| syn::parse_quote!(Default::default()));
            let ty = native_header::carrier(ty, observer)?;
            Ok(syn::parse_quote!(runtime::ledger::try_constructor_cell::<#ty, runtime::ledger::DefaultDB>(#value)?))
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
        // A source circuit named `recording` owns that inherent method name.
        // Retain access to the witnessed handle through a standard trait, so
        // no additional source-level method name needs to be reserved.
        let borrowed_from_contract = exported_method_names.contains("recording").then(|| {
            quote! {
                impl<'a, W> ::core::convert::From<&'a super::Contract<W>> for BorrowedContract<'a, W> {
                    fn from(contract: &'a super::Contract<W>) -> Self {
                        Self { witnesses: &contract.witnesses }
                    }
                }
            }
        });
        quote! {
            /// A recording handle with access to the contract's witnesses.
            pub struct BorrowedContract<'a, W> {
                pub(super) witnesses: &'a W,
            }
            impl<W> BorrowedContract<'_, W> {
                #(#borrowed_recorded_methods)*
            }
            #borrowed_from_contract
        }
    });
    // All admission and helper validation has already completed. Prune only
    // private definitions absent from the final recorded Rust reference graph;
    // keep the names used during rendering and every public item unchanged.
    let mut reachable_helpers = recorded::retain_reachable_helpers(
        recorded_helpers,
        &mut recorded_items,
        &mut recorded_methods,
        &mut borrowed_recorded_methods,
    );
    reachable_helpers.append(&mut recorded_items);
    let recorded_items = reachable_helpers;
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
    let recording_method: Option<syn::ImplItemFn> = if exported_method_names.contains("recording") {
        None
    } else if witnessed_recorded {
        Some(syn::parse_quote! {
            /// Borrow the contract's witnesses for a replayable circuit call.
            pub fn recording(&self) -> recorded::BorrowedContract<'_, W> {
                recorded::BorrowedContract { witnesses: &self.witnesses }
            }
        })
    } else if !recorded_items.is_empty() {
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
        let name = native_header::identifier(
            &field.id,
            RefusalSite::field(Owner::Ledger(field), "id"),
            observer,
        )?;
        let path = field.physical_path();
        match &field.declaration {
            LedgerFieldKind::Cell { ty } => {
                let ty = native_header::carrier(ty, observer)?;
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
                let ty = native_header::carrier(ty, observer)?;
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
                let ty = native_header::carrier(ty, observer)?;
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
                let leaf = native_header::carrier(ty, observer)?;
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
    // Exported aliases can be the only path to a named type. Collect their
    // bodies before namespace admission so references cannot remain missing
    // or silently resolve to a different alias with the same Rust spelling.
    for alias in &contract.type_aliases {
        located(alias.source.as_ref(), || {
            collect_named_types_observed(
                &alias.ty,
                &mut struct_definitions,
                &mut enum_definitions,
                observer,
                Some(&mut definition_origins),
            )
        })?;
    }
    let type_declarations::TypeDeclarations {
        module: types_module,
        alias_exports,
    } = type_declarations::render(
        &struct_definitions,
        &enum_definitions,
        &contract.type_aliases,
        slots_module.is_some(),
        has_merkle_witness_view,
        &definition_origins,
        observer,
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
    // Preserve existing semantic diagnostics before refusing private-IR pure
    // recursion. This consumes facts from the bounded preflight, not a new walk.
    preflight.finish(contract)?;
    let mut file = pure_native::item::file(
        runtime_abi,
        types_module,
        alias_exports,
        items,
        slots_module,
        ledger_module,
    );
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

#[cfg(test)]
mod constructor_cell_observer_tests;

#[cfg(test)]
mod constructor_prepass;

#[cfg(test)]
mod named_collection_tests;
