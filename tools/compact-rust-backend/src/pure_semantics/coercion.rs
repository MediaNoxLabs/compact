// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Bounded implicit conversion decisions, independent of Rust syntax.
//!
//! Identity precedes conversion shape and bound checks. Borrowed annotation
//! scans precede equality, owned refusal cloning and plan allocation. Work units
//! charge one per plan/equality frame, one per aggregate capacity slot or Box,
//! and the maximum byte length of compared text (or numeric text parsed).
//! These are conservative compiler work units, not measured CPU instructions.

use equality::equivalent;

use super::numeric::{
    UnsignedCastStrategy, UnsignedMaximum, cast_strategy_from_bounds, unsigned_maximum,
};
use crate::{RenderError, ir::Type, resource_limits};

/// Constructed only by the checked relation. No source Type or Expr is cloned
/// into a successful plan; aggregate children borrow source pairs while checking.
#[derive(Debug)]
pub(crate) struct CoercionPlan {
    kind: CoercionKind,
    may_fail: bool,
}

#[derive(Debug)]
pub(crate) enum CoercionKind {
    Identity,
    UnsignedToField {
        wide: bool,
    },
    UnsignedWiden {
        cast: UnsignedCastStrategy,
    },
    Vector {
        length: usize,
        element: Box<CoercionPlan>,
    },
    Aggregate {
        source: AggregateCarrier,
        target: AggregateCarrier,
        elements: Vec<CoercionPlan>,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum AggregateCarrier {
    Tuple,
    Vector,
}

impl CoercionPlan {
    pub(crate) fn kind(&self) -> &CoercionKind {
        &self.kind
    }
    pub(crate) fn may_fail(&self) -> bool {
        self.may_fail
    }
}

pub(crate) fn plan_coercion(actual: &Type, target: &Type) -> Result<CoercionPlan, RenderError> {
    preflight(actual, target)?;
    plan_bounded(actual, target, &mut Work::new())
}

fn preflight(actual: &Type, target: &Type) -> Result<(), RenderError> {
    drop(resource_limits::recording_type_nodes(actual)?);
    drop(resource_limits::recording_type_nodes(target)?);
    Ok(())
}

struct Work {
    observed: usize,
    limit: usize,
    pending_limit: usize,
}
impl Work {
    fn new() -> Self {
        Self {
            observed: 0,
            limit: resource_limits::coercion_work_limit(),
            pending_limit: resource_limits::coercion_pending_limit(),
        }
    }
    fn charge(&mut self, units: usize) -> Result<(), RenderError> {
        self.observed = self.observed.saturating_add(units);
        if self.observed > self.limit {
            return Err(RenderError::ResourceLimit {
                resource: "coercion_work",
                observed: self.observed,
                limit: self.limit,
            });
        }
        Ok(())
    }
    fn text_equal(&mut self, left: &str, right: &str) -> Result<bool, RenderError> {
        self.charge(left.len().max(right.len()))?;
        Ok(left == right)
    }
    fn maximum(&mut self, text: &str) -> Result<UnsignedMaximum, RenderError> {
        self.charge(text.len())?;
        unsigned_maximum(text)
    }
}

fn mismatch(actual: &Type, target: &Type) -> RenderError {
    RenderError::TypeMismatch {
        expected: target.clone(),
        actual: actual.clone(),
    }
}

fn limbs(maximum: UnsignedMaximum) -> (u128, u128) {
    match maximum {
        UnsignedMaximum::Small(low) => (0, low),
        UnsignedMaximum::Wide { high, low } => (high, low),
    }
}

fn plan_bounded(
    actual: &Type,
    target: &Type,
    work: &mut Work,
) -> Result<CoercionPlan, RenderError> {
    work.charge(1)?;
    if equivalent(actual, target, work)? {
        return Ok(CoercionPlan {
            kind: CoercionKind::Identity,
            may_fail: false,
        });
    }
    let (kind, may_fail) = match (actual, target) {
        (Type::Unsigned { max }, Type::Field) => (
            CoercionKind::UnsignedToField {
                wide: matches!(work.maximum(max)?, UnsignedMaximum::Wide { .. }),
            },
            false,
        ),
        (Type::Unsigned { max: source_max }, Type::Unsigned { max: target_max }) => {
            let source = work.maximum(source_max)?;
            let target_bound = work.maximum(target_max)?;
            if limbs(source) > limbs(target_bound) {
                return Err(mismatch(actual, target));
            }
            (
                CoercionKind::UnsignedWiden {
                    cast: cast_strategy_from_bounds(source, target_bound),
                },
                true,
            )
        }
        (
            Type::Vector {
                element: source,
                length: source_length,
            },
            Type::Vector {
                element: target,
                length: target_length,
            },
        ) if source_length == target_length => {
            let element = plan_bounded(source, target, work)?;
            let may_fail = element.may_fail;
            work.charge(1)?;
            (
                CoercionKind::Vector {
                    length: *source_length,
                    element: Box::new(element),
                },
                may_fail,
            )
        }
        (Type::Tuple { elements: sources }, Type::Tuple { elements: targets })
            if sources.len() == targets.len() && !sources.is_empty() =>
        {
            return aggregate(
                AggregateCarrier::Tuple,
                AggregateCarrier::Tuple,
                sources.iter().zip(targets),
                sources.len(),
                work,
            );
        }
        (
            Type::Tuple { elements: sources },
            Type::Vector {
                element: target,
                length,
            },
        ) if sources.len() == *length && !sources.is_empty() => {
            return aggregate(
                AggregateCarrier::Tuple,
                AggregateCarrier::Vector,
                sources.iter().map(|source| (source, target.as_ref())),
                sources.len(),
                work,
            );
        }
        (
            Type::Vector {
                element: source,
                length,
            },
            Type::Tuple { elements: targets },
        ) if *length == targets.len() && !targets.is_empty() => {
            return aggregate(
                AggregateCarrier::Vector,
                AggregateCarrier::Tuple,
                targets.iter().map(|target| (source.as_ref(), target)),
                targets.len(),
                work,
            );
        }
        _ => return Err(mismatch(actual, target)),
    };
    Ok(CoercionPlan { kind, may_fail })
}

fn aggregate<'a>(
    source: AggregateCarrier,
    target: AggregateCarrier,
    pairs: impl Iterator<Item = (&'a Type, &'a Type)>,
    count: usize,
    work: &mut Work,
) -> Result<CoercionPlan, RenderError> {
    // Capacity is charged before allocation, even if a later member refuses.
    work.charge(count)?;
    let mut elements = Vec::with_capacity(count);
    let mut may_fail = false;
    for (source, target) in pairs {
        let element = plan_bounded(source, target, work)?;
        may_fail |= element.may_fail;
        elements.push(element);
    }
    Ok(CoercionPlan {
        kind: CoercionKind::Aggregate {
            source,
            target,
            elements,
        },
        may_fail,
    })
}

mod equality;

#[cfg(test)]
mod tests;
