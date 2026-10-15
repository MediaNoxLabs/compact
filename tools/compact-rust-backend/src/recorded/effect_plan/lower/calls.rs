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

impl<'source> CallSite<'source> {
    pub(super) fn source(self) -> RefusalSite<'source> {
        match self {
            Self::Expression(source) => RefusalSite::node(Owner::Expr(source)),
            Self::Statement(source) => RefusalSite::node(Owner::Action(source)),
        }
    }

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
            refused(self.observer, site.source());
            return Err(self.concrete_failure(site.missing(path)));
        };
        if matches!(site, CallSite::Statement(_)) && signature.kind != CallKind::Stateful {
            refused(self.observer, site.source());
            return Err(self.concrete_failure(site.missing(path)));
        }
        if signature.kind == CallKind::Pure {
            return self
                .pure_call((declaration, site), name, arguments, scope, bindings, path)
                .inspect_err(|_| {
                    if let Some(observer) = self.observer {
                        observer.related(match site {
                            CallSite::Expression(source) => RefusalSite::node(Owner::Expr(source)),
                            CallSite::Statement(source) => RefusalSite::node(Owner::Action(source)),
                        });
                    }
                });
        }
        // Borrow the original body. The immutable signature snapshot is checked
        // against it before binding any formals; no recursive IR clone occurs.
        let callee = self.circuits.get(name).copied().ok_or_else(|| {
            refused(self.observer, site.source());
            invalid(path, "stateful call body is unavailable")
        })?;
        for (result, original) in [(&signature.result, false), (&callee.result, true)] {
            self.validate_call_annotation(result, path, original)?;
        }
        for (parameter, original) in signature
            .parameters
            .iter()
            .map(|p| (p, false))
            .chain(callee.parameters.iter().map(|p| (p, true)))
        {
            self.validate_call_annotation(&parameter.ty, path, original)?;
        }
        if signature.name != callee.name
            || signature.parameters != callee.parameters
            || signature.result != callee.result
        {
            refused(self.observer, RefusalSite::unavailable());
            return Err(invalid(path, "call declaration and body signature differ"));
        }
        if arguments.len() != callee.parameters.len() {
            refused(self.observer, site.source());
            return Err(invalid(
                path,
                "call argument count differs from declared parameters",
            ));
        }
        if !stateful_call_type(&callee.result)
            || callee.parameters.iter().any(|p| !stateful_call_type(&p.ty))
        {
            refused(self.observer, RefusalSite::node(Owner::Stateful(callee)));
            return Err(invalid(path, "unsupported stateful call signature"));
        }
        if self.call_stack.iter().any(|active| active == name) {
            refused(self.observer, site.source());
            return Err(invalid(path, "recursive stateful call"));
        }
        if self.call_stack.len() >= self.max_call_depth {
            refused(self.observer, site.source());
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
            let original_argument = argument;
            let value = self.expression(argument, scope, bindings, &argument_path)?;
            let value = match site {
                CallSite::Expression(_) => self
                    .coerce(value, &parameter.ty, bindings, &argument_path)
                    .inspect_err(|_| {
                        refused(
                            self.observer,
                            RefusalSite::node(Owner::Expr(original_argument)),
                        )
                    })?,
                CallSite::Statement(_) if value.ty == parameter.ty => value,
                CallSite::Statement(_) => {
                    refused(
                        self.observer,
                        RefusalSite::node(Owner::Expr(original_argument)),
                    );
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
                refused(
                    self.observer,
                    RefusalSite::field(Owner::Parameter(parameter), "name"),
                );
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
        let result = result.inspect_err(|_| {
            if let Some(observer) = self.observer {
                observer.related(match site {
                    CallSite::Expression(source) => RefusalSite::node(Owner::Expr(source)),
                    CallSite::Statement(source) => RefusalSite::node(Owner::Action(source)),
                });
            }
        })?;
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

impl Lowerer<'_> {
    // This flag comes only from the literal original/snapshot iterator above.
    // It is not inferred through pointer equality, names or Type equality.
    fn validate_call_annotation(
        &self,
        ty: &Type,
        path: &str,
        original: bool,
    ) -> Result<(), RecordingGap> {
        let observer = if original { self.observer } else { None };
        self.declarations
            .types()
            .validate_at_observed(ty, path, observer)
            .inspect_err(|_| {
                refused(
                    self.observer,
                    if original {
                        RefusalSite::node(Owner::Type(ty))
                    } else {
                        RefusalSite::unavailable()
                    },
                );
            })
    }
}

#[cfg(test)]
mod diagnostic_tests;

#[cfg(test)]
mod guard_tests;
