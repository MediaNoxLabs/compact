// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Product operations are checked independently against canonical source types.
use super::*;

pub(super) fn validate(
    binding: &Binding,
    declarations: &Declarations,
    scope: &HashMap<usize, Type>,
) -> Result<bool, RecordingGap> {
    let ty = &binding.value.ty;
    let path = &binding.path;
    let use_value = |value: &Value| used(value, scope, declarations, path);
    Ok(match &binding.operation {
        Operation::EnumVariant { variant } => {
            matches!(ty, Type::Enum { variants, .. } if variants.contains(variant))
        }
        Operation::Default => matches!(ty, Type::Struct { .. }) && value_type(ty),
        Operation::StructConstruct { fields: values } => {
            let Type::Struct { fields, .. } = ty else {
                return Ok(false);
            };
            if fields.len() != values.len() {
                return Ok(false);
            }
            for (field, value) in fields.iter().zip(values) {
                use_value(value)?;
                if field.ty != value.ty {
                    return Ok(false);
                }
            }
            true
        }
        Operation::StructProject {
            value,
            field,
            index,
        } => {
            use_value(value)?;
            let Type::Struct { fields, .. } = &value.ty else {
                return Ok(false);
            };
            fields
                .get(*index)
                .is_some_and(|member| member.name == *field && member.ty == *ty)
        }
        Operation::TupleProject { value, index } => {
            use_value(value)?;
            let Type::Tuple { elements } = &value.ty else {
                return Ok(false);
            };
            elements.get(*index).is_some_and(|element| element == ty)
        }
        Operation::Vector { elements: values } => {
            let Type::Vector { element, length } = ty else {
                return Ok(false);
            };
            if *length != values.len() {
                return Ok(false);
            }
            for value in values {
                use_value(value)?;
                if value.ty != **element {
                    return Ok(false);
                }
            }
            true
        }
        Operation::Tuple { elements: values } => {
            let Type::Tuple { elements } = ty else {
                return Ok(false);
            };
            if elements.len() != values.len() {
                return Ok(false);
            }
            for (element, value) in elements.iter().zip(values) {
                use_value(value)?;
                if *element != value.ty {
                    return Ok(false);
                }
            }
            true
        }
        _ => return Err(invalid(path, "expected rich value operation")),
    })
}
