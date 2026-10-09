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
        check_fixed_layouts(&types, path)?;
        for ty in types {
            match ty {
                Type::Unit
                | Type::Boolean
                | Type::Field
                | Type::JubjubPoint
                | Type::Bytes { .. }
                | Type::OpaqueString
                | Type::OpaqueBytes
                | Type::Vector { .. } => {}
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
                    if self.structs.get(name) != Some(fields) {
                        return Err(invalid(
                            path,
                            "struct definition is not canonical source type",
                        ));
                    }
                }
                Type::Tuple { .. } => {}
                Type::LedgerMap { .. } => {
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

/// Visit lists are parent-before-child, including multiple source annotations.
/// Charge fixed layout rather than syntax: a Vector type has one child annotation
/// but its defaults, clones and generated array layout contain N children. Opaque
/// leaves contribute one slot; their dynamic payload length is not bounded here.
/// These per-type limits do not claim total runtime RAM or repeated-call bounds.
pub(super) fn check_fixed_layouts(types: &[&Type], path: &str) -> Result<(), RecordingGap> {
    #[derive(Clone, Copy)]
    struct Layout {
        bytes: usize,
        slots: usize,
    }
    let mut layouts = HashMap::<*const Type, Layout>::new();
    let byte_limit = crate::resource_limits::recording_byte_payload_limit();
    let slot_limit = crate::resource_limits::recording_fixed_layout_limit();
    for &ty in types.iter().rev() {
        let get = |child: &Type| layouts[&(child as *const Type)];
        let sum = |children: &mut dyn Iterator<Item = &Type>| {
            children.fold(Layout { bytes: 0, slots: 1 }, |total, child| {
                let child = get(child);
                Layout {
                    bytes: total.bytes.saturating_add(child.bytes),
                    slots: total.slots.saturating_add(child.slots),
                }
            })
        };
        let layout = match ty {
            Type::Bytes { length } => Layout {
                bytes: *length,
                slots: 1,
            },
            Type::Struct { fields, .. } => sum(&mut fields.iter().map(|f| &f.ty)),
            Type::Tuple { elements } => sum(&mut elements.iter()),
            Type::Vector { element, length } => {
                let child = get(element);
                Layout {
                    bytes: child.bytes.saturating_mul(*length),
                    slots: 1usize.saturating_add(child.slots.saturating_mul(*length)),
                }
            }
            _ => Layout { bytes: 0, slots: 1 },
        };
        if layout.bytes > byte_limit {
            return Err(invalid(
                path,
                format!("fixed-byte aggregate payload exceeds {byte_limit} bytes"),
            ));
        }
        if layout.slots > slot_limit {
            return Err(invalid(
                path,
                format!("fixed aggregate layout exceeds {slot_limit} slots"),
            ));
        }
        layouts.insert(ty as *const Type, layout);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "experimental-recording-plan"))]
mod resource_tests;
