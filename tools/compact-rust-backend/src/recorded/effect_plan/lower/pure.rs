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
            refused(self.observer, RefusalSite::node(Owner::Action(action)));
            return Err(invalid(
                path,
                "pure Unit statement requires a pure Unit declaration",
            ));
        }
        self.pure_call(
            (declaration, calls::CallSite::Statement(action)),
            name,
            arguments,
            scope,
            bindings,
            path,
        )
        .inspect_err(|_| {
            if let Some(observer) = self.observer {
                observer.related(RefusalSite::node(Owner::Action(action)));
            }
        })?;
        Ok(())
    }

    pub(super) fn pure_call(
        &mut self,
        call: (CallId, calls::CallSite<'_>),
        name: &str,
        arguments: &[Expr],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let (declaration, site) = call;
        let signature = self.declarations.call(declaration).ok_or_else(|| {
            refused(self.observer, site.source());
            invalid(path, "unknown pure call declaration")
        })?;
        if arguments.len() != signature.parameters.len() {
            refused(self.observer, site.source());
            return Err(invalid(
                path,
                "pure argument count differs from declaration",
            ));
        }
        self.declarations
            .types()
            .validate_at(&signature.result, path)
            .inspect_err(|_| refused(self.observer, RefusalSite::unavailable()))?;
        for parameter in &signature.parameters {
            self.declarations
                .types()
                .validate_at(&parameter.ty, path)
                .inspect_err(|_| refused(self.observer, RefusalSite::unavailable()))?;
        }
        if !super::super::validate::pure_interface_type(&signature.result)
            || signature
                .parameters
                .iter()
                .any(|p| !super::super::validate::pure_interface_type(&p.ty))
        {
            refused(self.observer, site.source());
            return Err(invalid(path, "unsupported pure call interface"));
        }
        let mut values = Vec::new();
        for (i, (argument, parameter)) in arguments.iter().zip(&signature.parameters).enumerate() {
            let argument_path = format!("{path}.arguments[{i}]");
            let original_argument = argument;
            let value = self.expression(argument, scope, bindings, &argument_path)?;
            // Native pure calls require exact argument types; an explicit IR
            // Coerce is handled by expression lowering before this boundary.
            if value.ty != parameter.ty {
                refused(
                    self.observer,
                    RefusalSite::node(Owner::Expr(original_argument)),
                );
                return Err(invalid(
                    &argument_path,
                    "pure argument type differs from declaration",
                ));
            }
            values.push(value);
        }
        (if self.observer.is_some() {
            self.pure.certify_observed(name, path, self.observer)
        } else {
            self.pure.certify(name, path)
        })
        .inspect_err(|_| refused(self.observer, RefusalSite::unavailable()))?;
        let certified = self.pure.get(name).ok_or_else(|| {
            refused(self.observer, RefusalSite::unavailable());
            invalid(path, "pure certificate was not retained")
        })?;
        if self.call_stack.len().saturating_add(certified.call_depth()) > self.max_call_depth {
            refused(self.observer, RefusalSite::unavailable());
            return Err(invalid(
                path,
                "combined recording/pure call depth limit exceeded",
            ));
        }
        self.expanded_steps = self
            .expanded_steps
            .saturating_add(certified.expanded_work());
        if self.expanded_steps > self.max_expanded_steps {
            refused(self.observer, RefusalSite::unavailable());
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

#[cfg(test)]
mod guard_tests;
