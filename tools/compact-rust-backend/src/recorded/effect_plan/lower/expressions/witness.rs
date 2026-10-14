// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original witness bodies, without new checks or traversal.
use super::*;

impl Lowerer<'_> {
    pub(super) fn witness_operation(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::WitnessCall { name, arguments } => {
                let (witness_id, declaration) = self
                    .declarations
                    .resolve_witness(name)
                    .ok_or_else(|| invalid(path, "unknown witness"))?;
                self.declarations
                    .types()
                    .validate_at(&declaration.result, path)?;
                for parameter in &declaration.parameters {
                    self.declarations.types().validate_at(&parameter.ty, path)?;
                }
                if !validate::witness_result_type(&declaration.result)
                    || arguments.len() != declaration.parameters.len()
                {
                    return Err(invalid(
                        path,
                        "witness result must be supported and arguments must match arity",
                    ));
                }
                if declaration
                    .parameters
                    .iter()
                    .any(|p| !validate::witness_parameter_type(&p.ty))
                {
                    return Err(invalid(path, "unsupported witness parameter type"));
                }
                let result = declaration.result.clone();
                let mut args = Vec::new();
                for (i, (argument, parameter)) in
                    arguments.iter().zip(&declaration.parameters).enumerate()
                {
                    let path = format!("{path}.arguments[{i}]");
                    let value = self.expression(argument, scope, bindings, &path)?;
                    args.push(self.coerce(value, &parameter.ty, bindings, &path)?);
                }
                (
                    Operation::Witness {
                        declaration: witness_id,
                        arguments: args,
                    },
                    result,
                )
            }
            _ => unreachable!("expression dispatcher selects witness"),
        };
        Ok(value)
    }
}
