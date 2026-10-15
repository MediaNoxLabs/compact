// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete arithmetic-family semantics over independently visited children.
//! Both operands are visited before any operand-type, maximum or trait checks.
//! Child visitors retain DEFAULT inferred-type bounds before derived recursive
//! equality; coercion's charged relation is deliberately separate.
//! The complete generic Expr/body checker remains a later checkpoint.
use super::{
    children::Children,
    numeric::{
        ArithmeticKind, UnsignedArithmeticStrategy, UnsignedMaximum, checked_unsigned_arithmetic,
        unsigned_maximum,
    },
    sink::{ArithmeticSink, Typed},
};
use crate::{
    RenderError,
    ir::{ComparisonOperator, Expr, Type},
    native_representation::{Operation, check_tuple_traits},
};

pub(crate) enum ArithmeticSource<'a> {
    Unsigned {
        kind: ArithmeticKind,
        max: &'a str,
        left: &'a Expr,
        right: &'a Expr,
    },
    Field {
        kind: ArithmeticKind,
        left: &'a Expr,
        right: &'a Expr,
    },
    Equality {
        negated: bool,
        left: &'a Expr,
        right: &'a Expr,
    },
    Ordered {
        operator: ComparisonOperator,
        left: &'a Expr,
        right: &'a Expr,
    },
}

impl<'a> ArithmeticSource<'a> {
    fn operands(&self) -> (&'a Expr, &'a Expr) {
        match self {
            Self::Unsigned { left, right, .. }
            | Self::Field { left, right, .. }
            | Self::Equality { left, right, .. }
            | Self::Ordered { left, right, .. } => (left, right),
        }
    }
}

pub(crate) enum ArithmeticFacts {
    Field(ArithmeticKind),
    Unsigned(UnsignedArithmeticStrategy),
    Equality { negated: bool },
    Ordered(ComparisonOperator),
}

pub(crate) fn fold_arithmetic<S: ArithmeticSink, C: Children<S>>(
    source: ArithmeticSource<'_>,
    children: &mut C,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError> {
    let (left_source, right_source) = source.operands();
    let left = children.visit(left_source, sink)?;
    let right = children.visit(right_source, sink)?;
    // One shared output slot. Child/semantic checks complete before formatting.
    let (facts, ty) = match source {
        ArithmeticSource::Unsigned { kind, max, .. } => {
            let Type::Unsigned { max: left_max } = left.ty else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned {
                        max: max.to_owned(),
                    },
                    actual: left.ty,
                });
            };
            let Type::Unsigned { max: right_max } = right.ty else {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unsigned {
                        max: max.to_owned(),
                    },
                    actual: right.ty,
                });
            };
            let strategy = checked_unsigned_arithmetic(kind, &left_max, &right_max, max)?;
            (
                ArithmeticFacts::Unsigned(strategy),
                Type::Unsigned {
                    max: max.to_owned(),
                },
            )
        }
        ArithmeticSource::Field { kind, .. } => {
            if left.ty != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: left.ty,
                });
            }
            if right.ty != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: right.ty,
                });
            }
            (ArithmeticFacts::Field(kind), Type::Field)
        }
        ArithmeticSource::Equality { negated, .. } => {
            // Children supply bounded inferred types. Preserve exact derived
            // equality; coercion's separately charged relation is not this rule.
            if right.ty != left.ty {
                return Err(RenderError::TypeMismatch {
                    expected: left.ty,
                    actual: right.ty,
                });
            }
            check_tuple_traits(
                if negated {
                    Operation::NotEqual
                } else {
                    Operation::Equal
                },
                &left.ty,
            )?;
            (ArithmeticFacts::Equality { negated }, Type::Boolean)
        }
        ArithmeticSource::Ordered { operator, .. } => {
            for actual in [left.ty, right.ty] {
                let Type::Unsigned { max } = &actual else {
                    return Err(RenderError::ExpectedUnsigned(actual));
                };
                if matches!(unsigned_maximum(max)?, UnsignedMaximum::Wide { .. }) {
                    return Err(RenderError::UnsupportedWideUnsignedOperation {
                        operation: "ordered comparison",
                        max: max.clone(),
                    });
                }
            }
            (ArithmeticFacts::Ordered(operator), Type::Boolean)
        }
    };
    Ok(Typed {
        output: sink.arithmetic(facts, left.output, right.output),
        ty,
    })
}

#[cfg(test)]
mod tests;
