// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! All sixteen source variants really execute through a no-syntax consumer.
use super::*;

#[test]
fn all_sixteen_operations_return_checked_types_and_visit_children_once() {
    let left = parameter("left");
    let right = parameter("right");
    for (source, lt, rt, expected, binary) in [
        (
            CryptoSource::TransientHash(&left),
            Type::Field,
            Type::Unit,
            Type::Field,
            false,
        ),
        (
            CryptoSource::TransientCommit(&left, &right),
            Type::OpaqueString,
            Type::Field,
            Type::Field,
            true,
        ),
        (
            CryptoSource::PersistentHash(&left),
            Type::Field,
            Type::Unit,
            Type::Bytes { length: 32 },
            false,
        ),
        (
            CryptoSource::PersistentCommit(&left, &right),
            Type::Field,
            Type::Bytes { length: 32 },
            Type::Bytes { length: 32 },
            true,
        ),
        (
            CryptoSource::Keccak256(&left),
            Type::Field,
            Type::Unit,
            Type::Bytes { length: 32 },
            false,
        ),
        (
            CryptoSource::DegradeToTransient(&left),
            Type::Bytes { length: 32 },
            Type::Unit,
            Type::Field,
            false,
        ),
        (
            CryptoSource::UpgradeFromTransient(&left),
            Type::Field,
            Type::Unit,
            Type::Bytes { length: 32 },
            false,
        ),
        (
            CryptoSource::HashToCurve(&left),
            Type::OpaqueBytes,
            Type::Unit,
            Type::JubjubPoint,
            false,
        ),
        (
            CryptoSource::JubjubPointX(&left),
            Type::JubjubPoint,
            Type::Unit,
            Type::Field,
            false,
        ),
        (
            CryptoSource::JubjubPointY(&left),
            Type::JubjubPoint,
            Type::Unit,
            Type::Field,
            false,
        ),
        (
            CryptoSource::ConstructJubjubPoint(&left, &right),
            Type::Field,
            Type::Field,
            Type::JubjubPoint,
            true,
        ),
        (
            CryptoSource::EcAdd(&left, &right),
            Type::JubjubPoint,
            Type::JubjubPoint,
            Type::JubjubPoint,
            true,
        ),
        (
            CryptoSource::EcNeg(&left),
            Type::JubjubPoint,
            Type::Unit,
            Type::JubjubPoint,
            false,
        ),
        (
            CryptoSource::EcMul(&left, &right),
            Type::JubjubPoint,
            Type::Field,
            Type::JubjubPoint,
            true,
        ),
        (
            CryptoSource::EcMulGenerator(&left),
            Type::Field,
            Type::Unit,
            Type::JubjubPoint,
            false,
        ),
        (
            CryptoSource::JubjubScalarFromNative(&left),
            Type::Field,
            Type::Unit,
            Type::Field,
            false,
        ),
    ] {
        let mut children = LeafChildren {
            bindings: HashMap::from([("left", (&lt, ())), ("right", (&rt, ()))]),
            visits: vec![],
        };
        let mut sink = Consumer::default();
        assert_eq!(
            fold_crypto(source, &mut children, &mut sink).unwrap().ty,
            expected
        );
        let mut addresses = vec![std::ptr::from_ref(&left) as usize];
        if binary {
            addresses.push(std::ptr::from_ref(&right) as usize);
        }
        assert_eq!(children.visits, addresses);
        assert_eq!(sink.outputs.len(), 1);
    }
}
