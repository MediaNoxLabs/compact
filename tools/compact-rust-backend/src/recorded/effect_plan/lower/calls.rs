// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Stateful calls evaluate arguments once, then enter an isolated formal scope.
use super::super::declarations::CallKind;
use super::super::validate::stateful_call_type;
use super::*;

#[derive(Clone, Copy)]
pub(super) enum CallSite<'a> {
    Expression(&'a Expr),
    Statement(&'a StateAction),
}

impl CallSite<'_> {
    fn missing(self, path: &str) -> RecordingGap {
        match self {
            Self::Expression(expression) => RecordingGap::expression(expression, path.into()),
            Self::Statement(action) => RecordingGap::action(action, path.into()),
        }
    }
}

impl Lowerer<'_> {
    pub(super) fn call(
        &mut self,
        site: CallSite<'_>,
        name: &str,
        arguments: &[Expr],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let Some((declaration, signature)) = self.declarations.resolve_call(name) else {
            return Err(site.missing(path));
        };
        if matches!(site, CallSite::Statement(_)) && signature.kind != CallKind::Stateful {
            return Err(site.missing(path));
        }
        if signature.kind == CallKind::Pure {
            return self.pure_call(declaration, name, arguments, scope, bindings, path);
        }
        // Borrow the original body. The immutable signature snapshot is checked
        // against it before binding any formals; no recursive IR clone occurs.
        let callee = self
            .circuits
            .get(name)
            .copied()
            .ok_or_else(|| invalid(path, "stateful call body is unavailable"))?;
        for result in [&signature.result, &callee.result] {
            self.declarations.types().validate_at(result, path)?;
        }
        for parameter in signature.parameters.iter().chain(&callee.parameters) {
            self.declarations.types().validate_at(&parameter.ty, path)?;
        }
        if signature.name != callee.name
            || signature.parameters != callee.parameters
            || signature.result != callee.result
        {
            return Err(invalid(path, "call declaration and body signature differ"));
        }
        if arguments.len() != callee.parameters.len() {
            return Err(invalid(
                path,
                "call argument count differs from declared parameters",
            ));
        }
        if !stateful_call_type(&callee.result)
            || callee.parameters.iter().any(|p| !stateful_call_type(&p.ty))
        {
            return Err(invalid(path, "unsupported stateful call signature"));
        }
        if self.call_stack.iter().any(|active| active == name) {
            return Err(invalid(path, "recursive stateful call"));
        }
        if self.call_stack.len() >= self.max_call_depth {
            return Err(invalid(
                path,
                format!("recording call depth exceeds {}", self.max_call_depth),
            ));
        }
        // Do not push the callee until its arguments finish. f(f(x)) is a
        // sequential pair of calls, not recursion.
        let mut args = Vec::new();
        for (i, (argument, parameter)) in arguments.iter().zip(&callee.parameters).enumerate() {
            let argument_path = format!("{path}.arguments[{i}]");
            let value = self.expression(argument, scope, bindings, &argument_path)?;
            let value = match site {
                CallSite::Expression(_) => {
                    self.coerce(value, &parameter.ty, bindings, &argument_path)?
                }
                CallSite::Statement(_) if value.ty == parameter.ty => value,
                CallSite::Statement(_) => {
                    return Err(invalid(
                        &argument_path,
                        "statement call argument type differs from declared parameter",
                    ));
                }
            };
            args.push(value);
        }
        let mut child_scope = Scope::new();
        let mut parameters = Vec::new();
        for (i, parameter) in callee.parameters.iter().enumerate() {
            self.spend(&format!("{path}.parameters[{i}]"))?;
            let value = self.fresh(parameter.ty.clone());
            if child_scope
                .insert(parameter.name.clone(), value.clone())
                .is_some()
            {
                return Err(invalid(path, "duplicate callee parameter"));
            }
            parameters.push(value);
        }
        self.call_stack.push(name.into());
        let mut child_bindings = Vec::new();
        let result = self.circuit_body(
            callee,
            &child_scope,
            &mut child_bindings,
            &format!("{path}.callee[{name}]"),
        );
        self.call_stack.pop();
        let result = result?;
        Ok(self.emit(
            Operation::Call {
                declaration,
                arguments: args,
                parameters,
                body: Block {
                    bindings: child_bindings,
                    result,
                },
            },
            callee.result.clone(),
            bindings,
            path,
        ))
    }
}
