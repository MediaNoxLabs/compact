// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Canonical named definitions retained from bounded authoritative source IR.
//! Lowered annotations cannot create or change a named type. Native collectors
//! remain the definition authority; no generated syntax is retained here.
use super::{RecordingGap, invalid};
use crate::ir::{LedgerField, PureCircuit, StatefulCircuit, StructField, Type, WitnessDeclaration};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Default)]
pub(super) struct TypeEnvironment {
    structs: BTreeMap<String, Vec<StructField>>,
    enums: BTreeMap<String, Vec<String>>,
}

impl TypeEnvironment {
    pub(super) fn from_sources(
        root: &StatefulCircuit,
        ledger: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        pure: &HashMap<&str, &PureCircuit>,
        stateful: &HashMap<&str, &StatefulCircuit>,
    ) -> Result<Self, RecordingGap> {
        let types =
            crate::resource_limits::recording_source_types(root, ledger, witnesses, pure, stateful)
                .map_err(|error| invalid("types", error.to_string()))?;
        // No recursive collector, clone or equality runs until all source bodies
        // and types have passed the shared exhaustive borrowed preflight.
        // Definition collection does not apply the checked value byte policy:
        // unrelated/unsupported declarations cannot refuse a scalar root. Each
        // actively lowered type is checked before model validation and rendering.
        let mut environment = Self::default();
        for ty in types {
            if matches!(ty, Type::Struct { .. } | Type::Enum { .. }) {
                crate::collect_named_types(ty, &mut environment.structs, &mut environment.enums)
                    .map_err(|error| invalid("types", error.to_string()))?;
            }
        }
        // The native collector rejects duplicate raw names, but currently does
        // not reject member/variant names which normalize to the same Rust name.
        for fields in environment.structs.values() {
            unique_names(fields.iter().map(|f| f.name.as_str()))?;
        }
        for variants in environment.enums.values() {
            unique_names(variants.iter().map(String::as_str))?;
        }
        Ok(environment)
    }

    #[cfg(test)]
    pub(super) fn validate(&self, ty: &Type) -> Result<(), RecordingGap> {
        self.validate_at(ty, "types")
    }

    pub(super) fn validate_at(&self, ty: &Type, path: &str) -> Result<(), RecordingGap> {
        let types = crate::resource_limits::recording_type_nodes(ty)
            .map_err(|error| invalid(path, error.to_string()))?;
        check_byte_payloads(&types, path)?;
        for ty in types {
            match ty {
                Type::Unit
                | Type::Boolean
                | Type::Field
                | Type::JubjubPoint
                | Type::Bytes { .. } => {}
                Type::Unsigned { max } => {
                    crate::unsigned_maximum(max)
                        .map_err(|error| invalid(path, error.to_string()))?;
                }
                Type::Enum { name, variants } => {
                    if self.enums.get(name) != Some(variants) {
                        return Err(invalid(
                            path,
                            "enum definition is not canonical source type",
                        ));
                    }
                }
                Type::Struct { name, fields } => {
                    if fields.is_empty() || self.structs.get(name) != Some(fields) {
                        return Err(invalid(
                            path,
                            "struct definition is empty or not canonical source type",
                        ));
                    }
                }
                Type::Tuple { .. } => {}
                Type::Vector { .. }
                | Type::OpaqueString
                | Type::OpaqueBytes
                | Type::LedgerMap { .. } => {
                    return Err(invalid(
                        path,
                        "type is outside checked recording value domain",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn unique_names<'a>(names: impl Iterator<Item = &'a str>) -> Result<(), RecordingGap> {
    let mut emitted = HashSet::new();
    for name in names {
        let normalized = crate::semantic_identifier(name)
            .map_err(|error| invalid("types", error.to_string()))?;
        if !emitted.insert(normalized) {
            return Err(invalid(
                "types",
                "named members collide after Rust identifier normalization",
            ));
        }
    }
    Ok(())
}

/// Visit lists are parent-before-child. Compute each type's fixed-byte leaf sum
/// once, in reverse, before any canonical equality. This cap is per aggregate;
/// it is not a bound on all Field values, repeated calls, proving or runtime RAM.
fn check_byte_payloads(types: &[&Type], path: &str) -> Result<(), RecordingGap> {
    let mut payloads = HashMap::<*const Type, usize>::new();
    let limit = crate::resource_limits::recording_byte_payload_limit();
    for &ty in types.iter().rev() {
        let get = |child: &Type| payloads.get(&(child as *const Type)).copied().unwrap_or(0);
        let bytes = match ty {
            Type::Bytes { length } => *length,
            Type::Struct { fields, .. } => fields
                .iter()
                .fold(0usize, |sum, f| sum.saturating_add(get(&f.ty))),
            Type::Tuple { elements } => elements
                .iter()
                .fold(0usize, |sum, t| sum.saturating_add(get(t))),
            _ => 0,
        };
        if bytes > limit {
            return Err(invalid(
                path,
                format!("fixed-byte aggregate payload exceeds {limit} bytes"),
            ));
        }
        payloads.insert(ty as *const Type, bytes);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "experimental-recording-plan"))]
mod resource_tests;
