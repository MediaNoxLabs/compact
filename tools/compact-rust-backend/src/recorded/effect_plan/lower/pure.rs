// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pure functions reuse their generated implementation, after certification.
use super::*;

impl Lowerer<'_> {
    pub(super) fn pure_call(
        &mut self,
        declaration: CallId,
        name: &str,
        arguments: &[Expr],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let signature = self
            .declarations
            .call(declaration)
            .ok_or_else(|| invalid(path, "unknown pure call declaration"))?;
        if arguments.len() != signature.parameters.len() {
            return Err(invalid(
                path,
                "pure argument count differs from declaration",
            ));
        }
        if !surface_type(&signature.result)
            || signature.parameters.iter().any(|p| !surface_type(&p.ty))
        {
            return Err(invalid(path, "unsupported pure call interface"));
        }
        let mut values = Vec::new();
        for (i, (argument, parameter)) in arguments.iter().zip(&signature.parameters).enumerate() {
            let argument_path = format!("{path}.arguments[{i}]");
            let value = self.expression(argument, scope, bindings, &argument_path)?;
            // Native pure calls require exact argument types; an explicit IR
            // Coerce is handled by expression lowering before this boundary.
            if value.ty != parameter.ty {
                return Err(invalid(
                    &argument_path,
                    "pure argument type differs from declaration",
                ));
            }
            values.push(value);
        }
        self.pure.certify(name, path)?;
        let certified = self
            .pure
            .get(name)
            .ok_or_else(|| invalid(path, "pure certificate was not retained"))?;
        if self.call_stack.len().saturating_add(certified.call_depth()) > self.max_call_depth {
            return Err(invalid(
                path,
                "combined recording/pure call depth limit exceeded",
            ));
        }
        self.expanded_steps = self
            .expanded_steps
            .saturating_add(certified.expanded_work());
        if self.expanded_steps > self.max_expanded_steps {
            return Err(invalid(
                path,
                "combined recording/pure expanded work limit exceeded",
            ));
        }
        Ok(self.emit(
            Operation::PureCall {
                declaration,
                arguments: values,
            },
            signature.result.clone(),
            bindings,
            path,
        ))
    }
}
