// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Explicit refusal controls for compiler tests. Their domains are intentionally
//! different: direct VectorFoldCall is source-valid but not yet recorded directly;
//! opaque persistent commitments are native-IR-only source-policy controls.
use crate::ir::{Contract, Expr, Parameter, PureCircuit, Type};

pub(crate) fn fixed_map_input() -> Expr {
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

/// Genuine source-qualified direct fold. Each complete contract must include
/// the exact declaration below; a missing callee is not a refusal control.
const FOLD_STEP: &str = "recording_refusal_fold_step";
pub(crate) fn unsupported_fixed_input() -> Expr {
    Expr::VectorFoldCall {
        name: FOLD_STEP.into(),
        initial: Box::new(Expr::FieldLiteral { value: "0".into() }),
        source: Box::new(Expr::Tuple {
            elements: vec![Expr::FieldLiteral { value: "1".into() }],
        }),
        accumulator: Type::Field,
        element: Type::Field,
        length: 1,
    }
}
pub(crate) fn register_unsupported_fold(contract: &mut Contract) {
    assert!(!contract.circuits.iter().any(|c| c.name == FOLD_STEP));
    contract.circuits.push(PureCircuit {
        source: None,
        name: FOLD_STEP.into(),
        internal: true,
        parameters: ["acc", "item"]
            .into_iter()
            .map(|name| Parameter {
                name: name.into(),
                ty: Type::Field,
            })
            .collect(),
        result: Type::Field,
        body: Expr::Add {
            left: Box::new(Expr::Parameter { name: "acc".into() }),
            right: Box::new(Expr::Parameter {
                name: "item".into(),
            }),
        },
    });
}
pub(crate) fn with_unsupported_fold(mut contract: Contract) -> Contract {
    register_unsupported_fold(&mut contract);
    contract
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
    register_unsupported_fold(&mut contract);
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
    let Err(super::effect_plan::PlanError::Unsupported(gap)) = super::effect_plan::check_with_calls(
        root,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        &contract
            .circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect(),
        &std::collections::HashMap::new(),
    ) else {
        panic!("direct VectorFoldCall must remain explicitly unsupported")
    };
    assert_eq!(gap.ir_node, "Expr::VectorFoldCall");
    assert_eq!(gap.path, "return_value.value.value");
}
