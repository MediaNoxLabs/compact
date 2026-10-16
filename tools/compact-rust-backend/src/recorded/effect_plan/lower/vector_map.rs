// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Vector maps lower once, with a lexical formal and SSA captures.
use super::*;

impl Lowerer<'_> {
    pub(super) fn vector_map(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } = expression
        else {
            return Err(self.unsupported_expression(expression, path));
        };
        let source_path = format!("{path}.source");
        let start = bindings.len();
        let source = self.expression(source, scope, bindings, &source_path)?;
        let source_type = Type::Vector {
            element: Box::new(parameter.ty.clone()),
            length: *length,
        };
        let source = self.coerce(source, &source_type, bindings, &source_path)?;
        if !super::super::context_free::bindings(&bindings[start..]) {
            return Err(invalid(
                &source_path,
                "VectorMap source must be context-free",
            ));
        }
        let result_type = Type::Vector {
            element: Box::new(result.clone()),
            length: *length,
        };
        self.declarations.types().validate_at(&result_type, path)?;
        if !super::super::validate::value_type(&result_type) {
            return Err(invalid(path, "unsupported VectorMap result type"));
        }
        self.spend(path)?;
        let formal = self.fresh(parameter.ty.clone());
        let mut local = scope.clone();
        local.insert(parameter.name.clone(), formal.clone());
        let body_path = format!("{path}.body");
        let mut body_bindings = Vec::new();
        let value = self.expression(body, &local, &mut body_bindings, &body_path)?;
        if value.ty != *result {
            return Err(invalid(
                &body_path,
                "VectorMap body differs from declared result",
            ));
        }
        if !super::super::context_free::bindings(&body_bindings) {
            return Err(invalid(&body_path, "VectorMap body must be context-free"));
        }
        Ok((
            Operation::VectorMap {
                source,
                parameter: formal,
                body: Block {
                    bindings: body_bindings,
                    result: value,
                },
            },
            result_type,
        ))
    }
}
