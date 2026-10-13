// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Checked control composition over the complete generic expression visitor.
//! Source order, lexical validation and Unit semantics are owned here.
use super::{
    carrier::fold_carrier,
    children::Children,
    declarations::PureDeclarations,
    identifier::check_name,
    scope::{LocalScope, Scope},
    sink::{ExpressionSink, Typed},
    visitor::ExpressionVisitor,
};
use crate::{
    RenderError,
    ir::{Expr, LocalBinding, Type},
};

pub(crate) enum ControlSource<'a> {
    Assert {
        condition: &'a Expr,
        message: &'a str,
    },
    Sequence {
        steps: &'a [Expr],
        value: &'a Expr,
    },
    If {
        condition: &'a Expr,
        then: &'a Expr,
        otherwise: &'a Expr,
    },
    Let {
        bindings: &'a [LocalBinding],
        body: &'a Expr,
    },
}
pub(crate) struct IfFacts {
    pub(crate) equal_arms: bool,
    pub(crate) evaluate_condition: bool,
    pub(crate) unit_result: bool,
}
pub(crate) struct CheckedLocal<V, B, C> {
    pub(crate) binding: B,
    pub(crate) carrier: C,
    pub(crate) value: V,
}
pub(crate) enum CheckedControl<'a, V, B, C> {
    Assert {
        condition: V,
        message: &'a str,
    },
    Sequence {
        steps: Vec<V>,
        value: V,
    },
    If {
        facts: IfFacts,
        condition: V,
        then: V,
        otherwise: V,
    },
    Let {
        bindings: Vec<CheckedLocal<V, B, C>>,
        body: V,
    },
}

pub(crate) fn fold_control<S: ExpressionSink, D: PureDeclarations>(
    source: ControlSource<'_>,
    bindings: &Scope<'_, S::Binding>,
    declarations: &D,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError>
where
    S::Binding: Clone,
{
    // Let has its own owned lexical session, never a mutable overlay of parent.
    if let ControlSource::Let {
        bindings: local_bindings,
        body,
    } = source
    {
        return lexical(local_bindings, body, bindings, declarations, sink);
    }
    let mut children = ExpressionVisitor::new(bindings, declarations);
    let checked = match source {
        ControlSource::Assert { condition, message } => {
            let condition = children.visit(condition, sink)?;
            expect(condition.ty, Type::Boolean)?;
            Typed {
                output: CheckedControl::Assert {
                    condition: condition.output,
                    message,
                },
                ty: Type::Unit,
            }
        }
        ControlSource::Sequence { steps, value } => {
            let mut values = Vec::new();
            for step in steps {
                let step = children.visit(step, sink)?;
                expect(step.ty, Type::Unit)?;
                values.push(step.output);
            }
            let value = children.visit(value, sink)?;
            Typed {
                output: CheckedControl::Sequence {
                    steps: values,
                    value: value.output,
                },
                ty: value.ty,
            }
        }
        ControlSource::If {
            condition,
            then,
            otherwise,
        } => {
            let condition_value = children.visit(condition, sink)?;
            expect(condition_value.ty, Type::Boolean)?;
            // Both distinct source occurrences check before any coalescing.
            let then_value = children.visit(then, sink)?;
            let otherwise_value = children.visit(otherwise, sink)?;
            if then_value.ty != otherwise_value.ty {
                return Err(RenderError::TypeMismatch {
                    expected: then_value.ty,
                    actual: otherwise_value.ty,
                });
            }
            let facts = IfFacts {
                equal_arms: then == otherwise,
                evaluate_condition: crate::condition_needs_statement(condition),
                unit_result: then_value.ty == Type::Unit,
            };
            Typed {
                output: CheckedControl::If {
                    facts,
                    condition: condition_value.output,
                    then: then_value.output,
                    otherwise: otherwise_value.output,
                },
                ty: then_value.ty,
            }
        }
        ControlSource::Let { .. } => unreachable!("lexical session selected above"),
    };
    Ok(Typed {
        output: sink.control(
            checked.output,
            bindings.iter().map(|(key, (_, binding))| (*key, binding)),
        ),
        ty: checked.ty,
    })
}

fn expect(actual: Type, expected: Type) -> Result<(), RenderError> {
    if actual != expected {
        return Err(RenderError::TypeMismatch { expected, actual });
    }
    Ok(())
}

fn lexical<S: ExpressionSink, D: PureDeclarations>(
    bindings: &[LocalBinding],
    body: &Expr,
    outer: &Scope<'_, S::Binding>,
    declarations: &D,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError>
where
    S::Binding: Clone,
{
    let mut scope = LocalScope::new(outer);
    let mut checked = Vec::new();
    for binding in bindings {
        let name = check_name(&binding.name)?;
        let value =
            ExpressionVisitor::new(scope.bindings(), declarations).visit(&binding.value, sink)?;
        if value.ty != binding.ty {
            return Err(RenderError::TypeMismatch {
                expected: binding.ty.clone(),
                actual: value.ty,
            });
        }
        let carrier = fold_carrier(&binding.ty, sink.carriers())?;
        let local = sink.local_binding(
            &name,
            &binding.name,
            scope
                .bindings()
                .iter()
                .map(|(key, (_, symbol))| (*key, symbol)),
        );
        checked.push(CheckedLocal {
            binding: local.clone(),
            carrier,
            value: value.output,
        });
        scope.insert(binding.name.as_str(), &binding.ty, local);
    }
    let body = ExpressionVisitor::new(scope.bindings(), declarations).visit(body, sink)?;
    Ok(Typed {
        output: sink.control(
            CheckedControl::Let {
                bindings: checked,
                body: body.output,
            },
            scope
                .bindings()
                .iter()
                .map(|(key, (_, symbol))| (*key, symbol)),
        ),
        ty: body.ty,
    })
}

#[cfg(test)]
mod tests;
