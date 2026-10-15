// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete-family native/neutral differential outcomes and meaningful controls.
//! Worker gates are added by the parent after the author source handoff.
use super::*;
use crate::ir::ComparisonOperator;
use crate::pure_semantics::numeric::ArithmeticKind;
use protocol::compare;
mod old_unsigned;
mod protocol;
mod unsigned_output;
mod workers;

const WIDE: &str = "340282366920938463463374607431768211456";
const MAX_248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const TOO_WIDE: &str =
    "452312848583266388373324160190187140051835877600158453279131187530910662656";
fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn field(kind: ArithmeticKind, left: Expr, right: Expr) -> Expr {
    let left = Box::new(left);
    let right = Box::new(right);
    match kind {
        ArithmeticKind::Add => Expr::Add { left, right },
        ArithmeticKind::Subtract => Expr::Subtract { left, right },
        ArithmeticKind::Multiply => Expr::Multiply { left, right },
    }
}
fn unsigned(kind: ArithmeticKind, max: &str, left: Expr, right: Expr) -> Expr {
    let max = max.into();
    let left = Box::new(left);
    let right = Box::new(right);
    match kind {
        ArithmeticKind::Add => Expr::UnsignedAdd { max, left, right },
        ArithmeticKind::Subtract => Expr::UnsignedSubtract { max, left, right },
        ArithmeticKind::Multiply => Expr::UnsignedMultiply { max, left, right },
    }
}
fn equality(negated: bool, left: Expr, right: Expr) -> Expr {
    let left = Box::new(left);
    let right = Box::new(right);
    if negated {
        Expr::NotEqual { left, right }
    } else {
        Expr::Equal { left, right }
    }
}
fn ordered(operator: ComparisonOperator, left: Expr, right: Expr) -> Expr {
    Expr::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn literal(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}

mod child_order;
mod equality;
mod nested;
mod nested_equality;
mod numeric_outcomes;

#[test]
#[should_panic(expected = "private pure-expression dispatcher selects this family")]
fn wrong_family_private_arithmetic_adapter_retains_misuse_contract() {
    let _ = crate::pure_arithmetic_expression(&Expr::Unit, &HashMap::new(), &HashMap::new());
}
