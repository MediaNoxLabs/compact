// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Explicit refusal controls for compiler tests. Their domains are intentionally
//! different: VectorMap is a valid source construct not yet recorded directly;
//! opaque persistent commitments are native-IR-only source-policy controls.
use crate::ir::{Expr, Parameter, Type};

pub(crate) fn unsupported_fixed_input() -> Expr {
    Expr::VectorMap {
        parameter: Parameter {
            name: "sentinel_item".into(),
            ty: Type::Field,
        },
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![Expr::FieldLiteral { value: "1".into() }],
        }),
        body: Box::new(Expr::Parameter {
            name: "sentinel_item".into(),
        }),
        result: Type::Field,
        length: 1,
    }
}

pub(crate) fn unsupported_commit() -> Expr {
    Expr::PersistentCommit {
        value: Box::new(unsupported_fixed_input()),
        opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
    }
}

/// Keeps PersistentCommit's concrete typed-policy provenance. The Compact
/// frontend rejects opaque persistent hashing; never call this a source gap.
pub(crate) fn opaque_commit_policy() -> Expr {
    Expr::PersistentCommit {
        value: Box::new(Expr::Default {
            ty: Type::OpaqueString,
        }),
        opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
    }
}

#[test]
fn unsupported_control_is_native_valid_and_checked_refused_at_the_actual_child() {
    let mut contract: crate::ir::Contract = serde_json::from_str(include_str!(
        "../../tests/opaque-string-map-schema20-ir.json"
    ))
    .unwrap();
    let root = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "ensure")
        .unwrap();
    root.actions.clear();
    root.result = Type::Bytes { length: 32 };
    root.return_value = crate::ir::StateReturn::Expression {
        value: unsupported_commit(),
    };
    // A whole native render is required: a standalone checker refusal alone
    // cannot establish a legitimate diagnostic fallback control.
    let output = crate::render_with_capabilities(&contract).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "ensure")
            .unwrap()
            .recorded
    );
    let root = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == "ensure")
        .unwrap();
    let Err(super::effect_plan::PlanError::Unsupported(gap)) = super::effect_plan::check(
        root,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    ) else {
        panic!("direct VectorMap must remain explicitly unsupported")
    };
    assert_eq!(gap.ir_node, "Expr::VectorMap");
    assert_eq!(gap.path, "return_value.value.value");
}
