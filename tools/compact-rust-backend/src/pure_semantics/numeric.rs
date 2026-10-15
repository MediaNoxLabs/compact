// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Canonical numeric bounds and checked runtime operation strategies.
//!
//! This owner decides applicability without creating Rust syntax. Syntax adapters
//! consume these facts; they do not repeat bound parsing or operation admission.
//! Callers must apply the existing source resource gate before reaching these
//! leaf policies. Work is linear in the already bounded decimal input length.

use crate::RenderError;

pub(crate) fn field_literal_bytes(value: &str) -> Result<[u8; 32], RenderError> {
    let invalid = || RenderError::InvalidFieldLiteral(value.to_owned());
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let mut bytes = [0_u8; 32];
    for digit in value.bytes() {
        let mut carry = u16::from(digit - b'0');
        for byte in &mut bytes {
            let expanded = u16::from(*byte) * 10 + carry;
            *byte = expanded as u8;
            carry = expanded >> 8;
        }
        if carry != 0 {
            return Err(invalid());
        }
    }
    if midnight_transient_crypto::curve::Fr::from_le_bytes(&bytes).is_none() {
        return Err(invalid());
    }
    Ok(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsignedMaximum {
    Small(u128),
    Wide { high: u128, low: u128 },
}

pub(crate) fn unsigned_maximum(value: &str) -> Result<UnsignedMaximum, RenderError> {
    let bytes = field_literal_bytes(value)
        .map_err(|_| RenderError::InvalidUnsignedMaximum(value.to_owned()))?;
    if bytes[31] != 0 {
        return Err(RenderError::InvalidUnsignedMaximum(value.to_owned()));
    }
    let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
    let high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
    if high == 0 {
        Ok(UnsignedMaximum::Small(low))
    } else {
        Ok(UnsignedMaximum::Wide { high, low })
    }
}

/// Compact unsigned arithmetic, independent of any output representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArithmeticKind {
    Add,
    Subtract,
    Multiply,
}

impl ArithmeticKind {
    fn name(self) -> &'static str {
        match self {
            Self::Add => "addition",
            Self::Subtract => "subtraction",
            Self::Multiply => "multiplication",
        }
    }
}

/// An explicit cast may narrow: the runtime remains responsible for checking
/// the actual value. Implicit widening is a separate coercion relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsignedCastStrategy {
    SmallToSmall { source: u128, target: u128 },
    SmallToWide { high: u128, low: u128 },
    WideToSmall { high: u128, low: u128, target: u128 },
    WideToWide { high: u128, low: u128 },
}

pub(crate) fn checked_unsigned_cast(
    source_max: &str,
    target_max: &str,
) -> Result<UnsignedCastStrategy, RenderError> {
    let source = unsigned_maximum(source_max)?;
    let target = unsigned_maximum(target_max)?;
    Ok(cast_strategy_from_bounds(source, target))
}

/// Derive output selection from already parsed bounds. Widening callers apply
/// their own ordered bound relation; explicit casts also allow narrowing.
pub(crate) fn cast_strategy_from_bounds(
    source: UnsignedMaximum,
    target: UnsignedMaximum,
) -> UnsignedCastStrategy {
    match (source, target) {
        (UnsignedMaximum::Small(source), UnsignedMaximum::Small(target)) => {
            UnsignedCastStrategy::SmallToSmall { source, target }
        }
        (UnsignedMaximum::Small(_), UnsignedMaximum::Wide { high, low }) => {
            UnsignedCastStrategy::SmallToWide { high, low }
        }
        (UnsignedMaximum::Wide { high, low }, UnsignedMaximum::Small(target)) => {
            UnsignedCastStrategy::WideToSmall { high, low, target }
        }
        (UnsignedMaximum::Wide { .. }, UnsignedMaximum::Wide { high, low }) => {
            UnsignedCastStrategy::WideToWide { high, low }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsignedArithmeticStrategy {
    Small {
        kind: ArithmeticKind,
        left: u128,
        right: u128,
        result: u128,
    },
    WideAdd {
        high: u128,
        low: u128,
    },
}

pub(crate) fn checked_unsigned_arithmetic(
    kind: ArithmeticKind,
    left_max: &str,
    right_max: &str,
    result_max: &str,
) -> Result<UnsignedArithmeticStrategy, RenderError> {
    // Parse all three bounds first, preserving malformed-bound precedence over
    // the later unsupported-wide-operation diagnostic.
    let left = unsigned_maximum(left_max)?;
    let right = unsigned_maximum(right_max)?;
    let result = unsigned_maximum(result_max)?;
    if kind == ArithmeticKind::Add
        && let UnsignedMaximum::Wide { high, low } = result
    {
        // Wide-result addition admits either width at either operand position.
        return Ok(UnsignedArithmeticStrategy::WideAdd { high, low });
    }
    for (bound, text) in [(left, left_max), (right, right_max), (result, result_max)] {
        if matches!(bound, UnsignedMaximum::Wide { .. }) {
            return Err(RenderError::UnsupportedWideUnsignedOperation {
                operation: kind.name(),
                max: text.to_owned(),
            });
        }
    }
    let (
        UnsignedMaximum::Small(left),
        UnsignedMaximum::Small(right),
        UnsignedMaximum::Small(result),
    ) = (left, right, result)
    else {
        unreachable!("wide bounds were refused above")
    };
    Ok(UnsignedArithmeticStrategy::Small {
        kind,
        left,
        right,
        result,
    })
}

#[cfg(test)]
mod tests;
