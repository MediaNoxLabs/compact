// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Frozen coercible from signed 05217556; original body SHA256 1ee10b559848602062a2bffa7f1788fdaec44f361f1713045b24a2ff973acace.
use super::{Type, unsigned_bound, value_type};

pub(super) fn coercible(source: &Type, target: &Type) -> bool {
    if !value_type(source) || !value_type(target) {
        return false;
    }
    if source == target {
        return true;
    }
    match (source, target) {
        (Type::Unsigned { .. }, Type::Field) => unsigned_bound(source).is_some(),
        (Type::Unsigned { .. }, Type::Unsigned { .. }) => {
            fn limbs(bound: crate::UnsignedMaximum) -> (u128, u128) {
                match bound {
                    crate::UnsignedMaximum::Small(low) => (0, low),
                    crate::UnsignedMaximum::Wide { high, low } => (high, low),
                }
            }
            matches!((unsigned_bound(source),unsigned_bound(target)),(Some(a),Some(b)) if limbs(a)<=limbs(b))
        }
        (
            Type::Vector {
                element: a,
                length: n,
            },
            Type::Vector {
                element: b,
                length: m,
            },
        ) => n == m && coercible(a, b),
        (Type::Tuple { elements: a }, Type::Tuple { elements: b }) => {
            !a.is_empty() && a.len() == b.len() && a.iter().zip(b).all(|(a, b)| coercible(a, b))
        }
        (Type::Tuple { elements }, Type::Vector { element, length }) => {
            !elements.is_empty()
                && elements.len() == *length
                && elements.iter().all(|source| coercible(source, element))
        }
        (Type::Vector { element, length }, Type::Tuple { elements }) => {
            !elements.is_empty()
                && elements.len() == *length
                && elements.iter().all(|target| coercible(element, target))
        }
        _ => false,
    }
}
