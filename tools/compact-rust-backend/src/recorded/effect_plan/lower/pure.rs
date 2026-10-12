// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pure functions reuse their generated implementation, after certification.
use super::*;

impl Lowerer<'_> {
    /// A direct pure Unit statement uses the existing certified invocation.
    /// Stateful statement calls keep their separate declaration-kind boundary.
    pub(super) fn pure_statement(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let StateAction::PureCall { name, arguments } = action else {
            return Err(self.unsupported_action(action, path));
        };
        let Some((declaration, signature)) = self.declarations.resolve_call(name) else {
            return Err(self.unsupported_action(action, path));
        };
        if signature.kind != super::super::declarations::CallKind::Pure
            || signature.result != Type::Unit
        {
            return Err(invalid(
                path,
                "pure Unit statement requires a pure Unit declaration",
            ));
        }
        self.pure_call(declaration, name, arguments, scope, bindings, path)?;
        Ok(())
    }

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
        self.declarations
            .types()
            .validate_at(&signature.result, path)?;
        for parameter in &signature.parameters {
            self.declarations.types().validate_at(&parameter.ty, path)?;
        }
        if !super::super::validate::pure_interface_type(&signature.result)
            || signature
                .parameters
                .iter()
                .any(|p| !super::super::validate::pure_interface_type(&p.ty))
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
