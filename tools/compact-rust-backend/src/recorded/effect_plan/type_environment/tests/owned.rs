// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Fixed layout is bounded separately from raw type syntax and dynamic payloads.
use super::*;
use crate::ir::LocalBinding;
use crate::{RecordingResourceKind, RecordingResourcePhase};

fn assert_resource(gap: &RecordingGap, kind: RecordingResourceKind, limit: usize, path: &str) {
    let resource = gap.resource.as_ref().expect("typed layout producer");
    assert_eq!(resource.kind, kind);
    assert_eq!(
        resource.phase,
        RecordingResourcePhase::FixedLayoutValidation
    );
    assert_eq!(resource.limit, limit);
    assert_eq!(gap.path, path);
    assert_eq!(gap.ir_node, "RecordedPlan");
}

fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}

#[test]
fn fixed_vectors_multiply_byte_payloads_before_canonical_equality() {
    let env = TypeEnvironment::default();
    let limit = crate::resource_limits::recording_byte_payload_limit();
    env.validate(&vector(Type::Bytes { length: limit / 2 }, 2))
        .unwrap();
    let excessive = vector(
        Type::Bytes {
            length: limit / 2 + 1,
        },
        2,
    );
    let gap = env.validate(&excessive).unwrap_err();
    assert_eq!(
        gap.detail,
        format!("fixed-byte aggregate payload exceeds {limit} bytes")
    );
    assert_resource(
        &gap,
        RecordingResourceKind::FixedBytePayload,
        limit,
        "types",
    );
    let overflow = vector(vector(Type::Bytes { length: 32 }, 64), usize::MAX);
    let gap = env.validate(&overflow).unwrap_err();
    assert_resource(
        &gap,
        RecordingResourceKind::FixedBytePayload,
        limit,
        "types",
    );
    env.validate(&vector(vector(Type::Bytes { length: 16 }, 8), 512))
        .unwrap();
    assert!(
        env.validate(&vector(vector(Type::Bytes { length: 16 }, 8), 513))
            .is_err()
    );
}

#[test]
fn zero_byte_and_opaque_elements_still_have_bounded_fixed_layout() {
    let env = TypeEnvironment::default();
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    for element in [
        Type::Unit,
        Type::Field,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Bytes { length: 0 },
    ] {
        env.validate(&vector(element.clone(), limit - 1)).unwrap();
        let gap = env.validate(&vector(element.clone(), limit)).unwrap_err();
        assert_eq!(
            gap.detail,
            format!("fixed aggregate layout exceeds {limit} slots")
        );
        assert_resource(
            &gap,
            RecordingResourceKind::FixedLayoutSlots,
            limit,
            "types",
        );
        assert!(env.validate(&vector(element, usize::MAX)).is_err());
    }
    // Every array/tuple node counts, even when all leaves have zero byte payload.
    env.validate(&vector(vector(Type::Unit, 126), 258)).unwrap(); // 32767 slots
    assert!(env.validate(&vector(vector(Type::Unit, 126), 259)).is_err());
    env.validate(&vector(Type::Field, 0)).unwrap();
    // A zero-length outer array does not hide an invalid element annotation.
    assert!(
        env.validate(&vector(Type::Unsigned { max: "bad".into() }, 0))
            .unwrap_err()
            .resource
            .is_none()
    );
    assert!(
        env.validate(&vector(vector(Type::Unit, usize::MAX), 0))
            .is_err()
    );
}

#[test]
fn vector_owned_structs_remain_bound_to_exact_source_definitions() {
    let owned = structure(
        "Owned",
        Type::Tuple {
            elements: vec![
                Type::OpaqueString,
                Type::OpaqueBytes,
                vector(enumeration("State"), 2),
            ],
        },
    );
    let ty = structure("Envelope", vector(owned.clone(), 3));
    let source = root(ty.clone());
    let env = environment(&source).unwrap();
    env.validate(&ty).unwrap();
    let mut forged = owned.clone();
    let Type::Struct { fields, .. } = &mut forged else {
        unreachable!()
    };
    fields[0].ty = Type::OpaqueString;
    assert!(env.validate(&vector(forged, 3)).is_err());
    assert!(
        TypeEnvironment::default()
            .validate(&vector(owned, 3))
            .is_err()
    );
    let conflicts = root(Type::Tuple {
        elements: vec![ty, vector(structure("Owned", Type::Field), 1)],
    });
    assert!(environment(&conflicts).is_err());
}

fn hidden_default(name: &str, ty: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![],
        result: Type::Unit,
        body: Expr::Let {
            bindings: vec![LocalBinding {
                name: "unused".into(),
                ty: ty.clone(),
                value: Expr::Default { ty },
            }],
            body: Box::new(Expr::Unit),
        },
    }
}

#[test]
fn reachable_pure_hidden_layout_is_bounded_before_native_typing() {
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    let accepted = hidden_default("bounded", vector(Type::Field, limit - 1));
    let excessive = hidden_default("excessive", vector(Type::Unit, limit));
    let caller = PureCircuit {
        source: None,
        name: "caller".into(),
        internal: true,
        parameters: vec![],
        result: Type::Unit,
        body: Expr::Call {
            name: "excessive".into(),
            arguments: vec![],
        },
    };
    let pure = HashMap::from([
        ("bounded", &accepted),
        ("excessive", &excessive),
        ("caller", &caller),
    ]);
    let stateful = HashMap::new();
    let mut certifier = super::super::super::pure_certification::Certifier::new(&pure, &stateful);
    certifier.certify("bounded", "test").unwrap();
    // Unreachable oversized declarations did not invalidate the bounded helper.
    let gap = certifier.certify("caller", "test").unwrap_err();
    assert_resource(
        &gap,
        RecordingResourceKind::FixedLayoutSlots,
        limit,
        "test.callee[excessive]",
    );
}

#[test]
fn unrelated_fixed_layout_is_not_an_admission_policy_for_scalar_roots() {
    let source = root(Type::Field);
    let unused = hidden_default("unused", vector(Type::Field, usize::MAX));
    let pure = HashMap::from([("unused", &unused)]);
    let env = TypeEnvironment::from_sources(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::new(),
    )
    .unwrap();
    env.validate(&Type::Field).unwrap();
    super::super::super::check_with_calls(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::new(),
    )
    .unwrap();
}
