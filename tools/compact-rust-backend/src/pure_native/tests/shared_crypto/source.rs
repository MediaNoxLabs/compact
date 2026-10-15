// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-case selection only; semantic assertions use independent outcomes.
use super::*;
use crate::pure_semantics::crypto::CryptoSource;
#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    TransientHash,
    TransientCommit,
    PersistentHash,
    PersistentCommit,
    Keccak256,
    Degrade,
    Upgrade,
    HashToCurve,
    X,
    Y,
    Construct,
    Add,
    Negate,
    Multiply,
    Generator,
    Reduce,
}
pub(super) const ALL: [Kind; 16] = [
    Kind::TransientHash,
    Kind::TransientCommit,
    Kind::PersistentHash,
    Kind::PersistentCommit,
    Kind::Keccak256,
    Kind::Degrade,
    Kind::Upgrade,
    Kind::HashToCurve,
    Kind::X,
    Kind::Y,
    Kind::Construct,
    Kind::Add,
    Kind::Negate,
    Kind::Multiply,
    Kind::Generator,
    Kind::Reduce,
];
impl Kind {
    pub(super) fn binary(self) -> bool {
        matches!(
            self,
            Self::TransientCommit
                | Self::PersistentCommit
                | Self::Construct
                | Self::Add
                | Self::Multiply
        )
    }
}
pub(super) fn operation(kind: Kind, left: Expr, right: Expr) -> Expr {
    let left = Box::new(left);
    let right = Box::new(right);
    match kind {
        Kind::TransientHash => Expr::TransientHash { value: left },
        Kind::TransientCommit => Expr::TransientCommit {
            value: left,
            opening: right,
        },
        Kind::PersistentHash => Expr::PersistentHash { value: left },
        Kind::PersistentCommit => Expr::PersistentCommit {
            value: left,
            opening: right,
        },
        Kind::Keccak256 => Expr::Keccak256 { value: left },
        Kind::Degrade => Expr::DegradeToTransient { value: left },
        Kind::Upgrade => Expr::UpgradeFromTransient { value: left },
        Kind::HashToCurve => Expr::HashToCurve { value: left },
        Kind::X => Expr::JubjubPointX { value: left },
        Kind::Y => Expr::JubjubPointY { value: left },
        Kind::Construct => Expr::ConstructJubjubPoint { x: left, y: right },
        Kind::Add => Expr::EcAdd { left, right },
        Kind::Negate => Expr::EcNeg { value: left },
        Kind::Multiply => Expr::EcMul {
            point: left,
            scalar: right,
        },
        Kind::Generator => Expr::EcMulGenerator { scalar: left },
        Kind::Reduce => Expr::JubjubScalarFromNative { value: left },
    }
}
pub(super) fn descriptor(source: &Expr) -> Option<CryptoSource<'_>> {
    Some(match source {
        Expr::TransientHash { value } => CryptoSource::TransientHash(value),
        Expr::TransientCommit { value, opening } => CryptoSource::TransientCommit(value, opening),
        Expr::PersistentHash { value } => CryptoSource::PersistentHash(value),
        Expr::PersistentCommit { value, opening } => CryptoSource::PersistentCommit(value, opening),
        Expr::Keccak256 { value } => CryptoSource::Keccak256(value),
        Expr::DegradeToTransient { value } => CryptoSource::DegradeToTransient(value),
        Expr::UpgradeFromTransient { value } => CryptoSource::UpgradeFromTransient(value),
        Expr::HashToCurve { value } => CryptoSource::HashToCurve(value),
        Expr::JubjubPointX { value } => CryptoSource::JubjubPointX(value),
        Expr::JubjubPointY { value } => CryptoSource::JubjubPointY(value),
        Expr::ConstructJubjubPoint { x, y } => CryptoSource::ConstructJubjubPoint(x, y),
        Expr::EcAdd { left, right } => CryptoSource::EcAdd(left, right),
        Expr::EcNeg { value } => CryptoSource::EcNeg(value),
        Expr::EcMul { point, scalar } => CryptoSource::EcMul(point, scalar),
        Expr::EcMulGenerator { scalar } => CryptoSource::EcMulGenerator(scalar),
        Expr::JubjubScalarFromNative { value } => CryptoSource::JubjubScalarFromNative(value),
        _ => return None,
    })
}
