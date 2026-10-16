// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Default compositional admission and native-validation boundaries for ADR0371.
use compact_rust_backend::{ir::Contract, render_with_capabilities};

fn fixture() -> Contract {
    serde_json::from_str(include_str!("effect-plan-prototype-schema20-ir.json")).unwrap()
}

#[test]
fn two_real_source_compositions_have_default_recording_admission() {
    let rendered = render_with_capabilities(&fixture()).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 2);
    for circuit in rendered.capabilities.circuits {
        assert!(circuit.recorded);
        assert!(circuit.observed_call);
        assert_eq!(circuit.recording_unavailable.is_none(), circuit.recorded);
    }
}

#[test]
fn invalid_ledger_declarations_fail_before_recording_admission() {
    let mut contract = fixture();
    contract.ledger_fields[0].index = 1;
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn invalid_witness_signature_is_never_hidden_by_profile_fallback() {
    let mut contract = fixture();
    contract.witnesses[0]
        .parameters
        .push(compact_rust_backend::ir::Parameter {
            name: "missing".into(),
            ty: compact_rust_backend::ir::Type::Field,
        });
    assert!(render_with_capabilities(&contract).is_err());
}

use compact_rust_backend::{RenderError, ir::*};

fn single(body: ReturnPlan) -> Contract {
    let mut contract: Contract =
        serde_json::from_str(include_str!("effectful-return-schema20-ir.json")).unwrap();
    let circuit = &mut contract.stateful_circuits[0];
    circuit.source = None;
    circuit.name = "sample".into();
    circuit.parameters = vec![Parameter {
        name: "x".into(),
        ty: Type::Field,
    }];
    circuit.return_value = StateReturn::Effectful { body };
    contract
}
fn value(value: Expr) -> ReturnPlan {
    ReturnPlan::Value { value }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(value: u64) -> Expr {
    Expr::FieldLiteral {
        value: value.to_string(),
    }
}
fn read() -> Expr {
    Expr::CellRead {
        field: "state".into(),
        index: 0,
    }
}
fn root_error(mut error: RenderError) -> RenderError {
    while let RenderError::Located { error: inner, .. } = error {
        error = *inner;
    }
    error
}

#[test]
fn malformed_types_and_escaped_references_are_hard_native_errors() {
    let bodies = [
        value(Expr::Boolean { value: true }),
        ReturnPlan::Let {
            bindings: vec![LocalBinding {
                name: "bad".into(),
                ty: Type::Field,
                value: Expr::Boolean { value: true },
            }],
            result: Box::new(value(read())),
        },
        ReturnPlan::Sequence {
            actions: vec![StateAction::CellWrite {
                field: "state".into(),
                index: 0,
                value: Expr::Boolean { value: true },
            }],
            result: Box::new(value(read())),
        },
    ];
    for body in bodies {
        let error = render_with_capabilities(&single(body))
            .err()
            .expect("native type error must not become an unavailable capability");
        assert!(matches!(
            root_error(error),
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean
            }
        ));
    }
    let escaped = ReturnPlan::Sequence {
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "branch_only".into(),
                ty: Type::Field,
                value: field(1),
            }],
            action: Box::new(StateAction::CellWrite {
                field: "state".into(),
                index: 0,
                value: parameter("branch_only"),
            }),
        }],
        result: Box::new(value(parameter("branch_only"))),
    };
    let dead = ReturnPlan::Conditional {
        condition: Expr::Boolean { value: true },
        then: Box::new(value(read())),
        otherwise: Box::new(value(parameter("branch_only"))),
    };
    let unused = ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "unused".into(),
            ty: Type::Field,
            value: parameter("branch_only"),
        }],
        result: Box::new(value(read())),
    };
    for body in [escaped, dead, unused] {
        let error = render_with_capabilities(&single(body))
            .err()
            .expect("native scope error must not become an unavailable capability");
        assert!(
            matches!(root_error(error),RenderError::UnknownParameter(name) if name == "branch_only")
        );
    }
    let wrong_field = single(value(Expr::CellRead {
        field: "missing".into(),
        index: 0,
    }));
    let error = render_with_capabilities(&wrong_field)
        .err()
        .expect("unknown field is a native error");
    assert!(matches!(root_error(error),RenderError::UnknownLedgerField(name) if name == "missing"));
}

#[test]
fn certified_pure_helper_is_available_through_public_recording() {
    let mut contract = single(value(Expr::Call {
        name: "identity".into(),
        arguments: vec![parameter("x")],
    }));
    contract.circuits.push(PureCircuit {
        source: None,
        name: "identity".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: parameter("value"),
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    let row = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "sample")
        .unwrap();
    assert!(row.recorded && row.observed_call);
    assert!(row.recording_unavailable.is_none());
}

#[test]
fn direct_bytes_temporaries_now_compose_with_recorded_cell_reads() {
    let contract = single(ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "unused_bytes".into(),
            ty: Type::Bytes { length: 1 },
            value: Expr::BytesLiteral { bytes: vec![0] },
        }],
        result: Box::new(value(read())),
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    let row = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "sample")
        .unwrap();
    assert!(row.recorded && row.observed_call);
    assert!(row.recording_unavailable.is_none());
}

#[test]
fn a_legacy_supported_wide_binding_still_records_with_either_feature_setting() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("effectful-return-schema20-ir.json")).unwrap();
    let StateReturn::Effectful { body } = &mut contract.stateful_circuits[0].return_value else {
        panic!("fixture return form")
    };
    let max = "340282366920938463463374607431768211456";
    *body = ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "unused_wide".into(),
            ty: Type::Unsigned { max: max.into() },
            value: Expr::UnsignedLiteral {
                value: "1".into(),
                max: max.into(),
            },
        }],
        result: Box::new(body.clone()),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered.capabilities.circuits[0]
            .recording_unavailable
            .is_none()
    );
}

#[test]
fn legacy_and_experiment_both_admit_a_potentially_empty_selected_branch() {
    // Recorded API availability does not guarantee a nonempty VM transcript on
    // every execution. The transaction adapter owns EmptyTranscript rejection.
    let contract = single(ReturnPlan::Conditional {
        condition: Expr::Equal {
            left: Box::new(parameter("x")),
            right: Box::new(field(0)),
        },
        then: Box::new(ReturnPlan::Sequence {
            actions: vec![StateAction::CellWrite {
                field: "state".into(),
                index: 0,
                value: Expr::Add {
                    left: Box::new(read()),
                    right: Box::new(field(1)),
                },
            }],
            result: Box::new(value(read())),
        }),
        otherwise: Box::new(value(field(0))),
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
}
