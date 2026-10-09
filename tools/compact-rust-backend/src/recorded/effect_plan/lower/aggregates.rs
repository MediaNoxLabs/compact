// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rich values preserve member order and keep declarations separate from syntax.
use super::super::validate;
use super::*;

impl Lowerer<'_> {
    pub(super) fn aggregate(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        match expression {
            Expr::BytesLiteral { bytes } => {
                let ty = Type::Bytes {
                    length: bytes.len(),
                };
                if !validate::surface_type(&ty) {
                    return Err(invalid(path, "unsupported fixed-byte length"));
                }
                Ok((Operation::Literal(Literal::Bytes(bytes.clone())), ty))
            }
            Expr::EnumVariant { ty, variant } => {
                self.declarations.types().validate_at(ty, path)?;
                let Type::Enum { variants, .. } = ty else {
                    return Err(invalid(path, "enum literal requires an enum type"));
                };
                if !validate::surface_type(ty) || !variants.contains(variant) {
                    return Err(invalid(path, "unknown or unsupported enum variant"));
                }
                Ok((
                    Operation::EnumVariant {
                        variant: variant.clone(),
                    },
                    ty.clone(),
                ))
            }
            Expr::Default { ty } => {
                self.declarations.types().validate_at(ty, path)?;
                if !matches!(ty, Type::Struct { .. }) || !validate::surface_type(ty) {
                    return Err(invalid(path, "default requires a supported struct type"));
                }
                Ok((Operation::Default, ty.clone()))
            }
            Expr::StructLiteral { ty, fields } => self.structure(ty, fields, scope, bindings, path),
            Expr::StructField {
                value,
                field,
                index,
            } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                let Type::Struct { fields, .. } = &value.ty else {
                    return Err(invalid(path, "projection requires a struct value"));
                };
                let member = fields
                    .get(*index)
                    .filter(|member| member.name == *field)
                    .ok_or_else(|| invalid(path, "unknown struct member or inconsistent index"))?;
                let ty = member.ty.clone();
                Ok((
                    Operation::StructProject {
                        value,
                        field: field.clone(),
                        index: *index,
                    },
                    ty,
                ))
            }
            Expr::Tuple { elements } => {
                let mut values = Vec::new();
                for (index, element) in elements.iter().enumerate() {
                    values.push(self.expression(
                        element,
                        scope,
                        bindings,
                        &format!("{path}.elements[{index}]"),
                    )?);
                }
                let ty = Type::Tuple {
                    elements: values.iter().map(|value| value.ty.clone()).collect(),
                };
                Ok((Operation::Tuple { elements: values }, ty))
            }
            _ => unreachable!("aggregate dispatcher selects this expression family"),
        }
    }

    fn structure(
        &mut self,
        ty: &Type,
        expressions: &[Expr],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        self.declarations.types().validate_at(ty, path)?;
        let Type::Struct { fields, .. } = ty else {
            return Err(invalid(path, "constructor requires a struct type"));
        };
        if !validate::surface_type(ty) || fields.len() != expressions.len() {
            return Err(invalid(
                path,
                "unsupported struct type or member count mismatch",
            ));
        }
        let mut values = Vec::new();
        for (index, (member, expression)) in fields.iter().zip(expressions).enumerate() {
            let member_path = format!("{path}.fields[{index}]");
            let value = self.expression(expression, scope, bindings, &member_path)?;
            if value.ty != member.ty {
                return Err(invalid(
                    &member_path,
                    "struct member type differs from declaration",
                ));
            }
            values.push(value);
        }
        Ok((Operation::StructConstruct { fields: values }, ty.clone()))
    }
}
