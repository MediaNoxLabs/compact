// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Literal H197 old collection bodies. Only visibility headers are adjusted.
use crate::*;

mod actions;
mod constructors;
mod expressions;
pub(super) use actions::{collect_action_types, collect_return_plan_types};
pub(super) use constructors::collect_constructor_step_types;
pub(super) use expressions::collect_expression_types;

pub(super) fn has_named_type_collision(
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

pub(super) fn collect_named_types_with_origins<'ir>(
    ty: &'ir Type,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: Option<&mut type_declarations::DefinitionOrigins<'ir>>,
) -> Result<(), RenderError> {
    collect_named_types_observed(ty, structs, enums, None, origins)
}

pub(super) fn collect_named_types_observed<'ir>(
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
                return Err(RenderError::ConflictingStruct(name.clone()));
            }
            if let Some(existing) = structs.get(name) {
                if existing != fields {
                    return Err(RenderError::ConflictingStruct(name.clone()));
                }
            } else {
                if has_named_type_collision(name, structs, enums)? {
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
                return Err(RenderError::ConflictingEnum(name.clone()));
            }
            if variants.is_empty() {
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
                    return Err(RenderError::DuplicateEnumVariant(variant.clone()));
                }
            }
            if let Some(existing) = enums.get(name) {
                if existing != variants {
                    return Err(RenderError::ConflictingEnum(name.clone()));
                }
            } else {
                if has_named_type_collision(name, structs, enums)? {
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
