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

/// Deliberate CoPS030 diagnostic migration: preserve a complete historical
/// refusal and check the actual public failure's code, node, path and dependency.
pub(crate) fn assert_checked_fold_refusal(
    contract: &Contract,
    root: &crate::ir::StatefulCircuit,
    historical: crate::recorded::RecordingGap,
    expected_path: &str,
) {
    use crate::recorded::{RecordingGap, RecordingOutcome, effect_plan, generic_reference};
    use std::collections::{HashMap, HashSet};
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<HashMap<_, _>>();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let calls = contract
        .stateful_circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let old = generic_reference::render_recorded_circuit(
        root,
        &ledger,
        &witnesses,
        &pure,
        &calls,
        &HashSet::new(),
    )
    .unwrap();
    assert!(matches!(old, RecordingOutcome::Unsupported(gap) if gap == historical));
    let Err(effect_plan::DiagnosticPlanError::Unsupported(rejection)) =
        effect_plan::lower_with_calls_diagnostic(root, &ledger, &witnesses, &pure, &calls)
    else {
        panic!("expected checked Fold refusal")
    };
    let expected = rejection.into_checked_gap();
    assert_eq!(
        expected.code,
        crate::recorded::RecordingGapCode::UnsupportedExpression
    );
    assert_eq!(expected.ir_node, "Expr::VectorFoldCall");
    assert_eq!(expected.path, expected_path);
    let output = crate::render_with_capabilities(contract).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == root.name)
        .unwrap();
    assert!(!cap.recorded && !cap.observed_call);
    assert_eq!(cap.recording_unavailable.as_ref(), Some(&expected));
    let dependency: &RecordingGap = cap.observed_call_unavailable.as_ref().unwrap();
    assert_eq!(dependency.path, expected.path);
    assert_eq!(dependency.ir_node, expected.ir_node);
    assert_eq!(dependency.resource, expected.resource);
}
