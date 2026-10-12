// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::ir::{Parameter, ReturnPlan};
use trace::{Capture, Outcome, Phase, Scope};
#[path = "trace_tests/fixtures.rs"]
mod fixtures;

fn capture(contract: &Contract) -> (crate::RenderedContract, Vec<trace::Invocation>) {
    let capture = Capture::begin();
    let rendered = crate::render_with_capabilities(contract).unwrap();
    let invocations = capture.finish();
    let joined = attribution::join(&rendered.capabilities, &inspect(contract), &invocations);
    assert!(joined.errors.is_empty(), "{:?}", joined.errors);
    (rendered, invocations)
}

#[test]
fn observation_preserves_eager_error_and_lazy_short_circuit_without_repeating_calls() {
    fn eager(calls: &Cell<usize>) -> Result<Option<u8>, RenderError> {
        Ok(record_option!("first", {
            calls.set(calls.get() + 1);
            Some(1_u8)
        })
        .or(record_option!("second", {
            calls.set(calls.get() + 1);
            Err::<Option<u8>, _>(RenderError::SchemaVersion(0))?
        })))
    }
    let capture = Capture::begin();
    let contract = fixture();
    let scope = Scope::begin(&contract.stateful_circuits[0], false);
    let calls = Cell::new(0);
    let result = eager(&calls).map(|_| RecordingOutcome::Supported(()));
    scope.finish(&result);
    let rows = capture.finish();
    assert_eq!(
        calls.get(),
        2,
        "later eager validation must remain observable"
    );
    assert_eq!(rows[0].body_owner, Some("first"));
    assert!(matches!(rows[0].outcome, Outcome::Error { .. }));

    let capture = Capture::begin();
    let scope = Scope::begin(&contract.stateful_circuits[0], false);
    let calls = Cell::new(0);
    let value = record_option!("first", {
        calls.set(calls.get() + 1);
        Some(7)
    })
    .or_else(|| {
        record_option!("unreached", {
            calls.set(calls.get() + 1);
            Some(8)
        })
    });
    scope.finish(&Ok(RecordingOutcome::Supported(value)));
    let rows = capture.finish();
    assert_eq!(calls.get(), 1);
    assert_eq!(rows[0].attempts.len(), 1);
}

#[test]
fn nested_helpers_and_preview_phase_do_not_overwrite_the_export_owner() {
    let capture = Capture::begin();
    let contract = fixture();
    let circuit = &contract.stateful_circuits[0];
    let parent = Scope::begin(circuit, false);
    trace::handled("parent-owner");
    {
        let _phase = trace::HelperPlanning::begin();
        let child = Scope::begin(circuit, true);
        trace::handled("child-owner");
        child.finish(&Ok(RecordingOutcome::Supported(())));
    }
    parent.finish(&Ok(RecordingOutcome::Supported(())));
    let after = Scope::begin(circuit, false);
    after.finish(&Ok(RecordingOutcome::<()>::Unsupported(
        RecordingGap::no_effect(),
    )));
    let rows = capture.finish();
    assert_eq!(rows[0].body_owner, Some("parent-owner"));
    assert_eq!(rows[1].body_owner, Some("child-owner"));
    assert_eq!(rows[1].parent, Some(rows[0].id));
    assert_eq!(rows[1].phase, Phase::HelperPlanning);
    assert_eq!(rows[2].phase, Phase::PublicEmission);
    assert_eq!(rows[2].parent, None);
}

#[test]
fn unwind_discards_a_partial_capture_before_the_next_compilation() {
    let contract = fixture();
    let failure = std::panic::catch_unwind(|| {
        let _capture = Capture::begin();
        let _scope = Scope::begin(&contract.stateful_circuits[0], false);
        panic!("deliberate trace unwind");
    });
    assert!(failure.is_err());
    let (_, rows) = capture(&contract);
    assert_eq!(rows[0].id, 0);
    assert!(
        rows.iter()
            .any(|row| matches!(row.outcome, Outcome::Supported))
    );
}

#[test]
fn full_compilation_attributes_ordinary_returns_without_changing_output() {
    let contract = fixtures::counter();
    let plain = crate::render_with_capabilities(&contract).unwrap();
    let (observed, rows) = capture(&contract);
    assert_eq!(observed.source, plain.source);
    assert_eq!(
        serde_json::to_value(&observed.capabilities).unwrap(),
        serde_json::to_value(&plain.capabilities).unwrap()
    );
    let row = rows
        .iter()
        .find(|row| row.phase == Phase::PublicEmission && !row.helper)
        .unwrap();
    assert_eq!(row.body_owner, Some("algebra.ordinary"));
    assert_eq!(row.return_dispatch, None);
    assert!(matches!(row.outcome, Outcome::Supported));
    let candidates: Vec<_> = row
        .attempts
        .iter()
        .map(|attempt| serde_json::to_value(attempt).unwrap())
        .filter(|value| value["produced_candidate"] == true)
        .map(|value| value["route"].clone())
        .collect();
    assert_eq!(candidates, vec![serde_json::json!("algebra.ordinary")]);
}

#[test]
fn outer_opaque_guard_refusal_is_distinct_from_a_selected_return_dispatch() {
    let mut contract = fixtures::counter();
    contract.stateful_circuits[0].parameters.push(Parameter {
        name: "unused_key".into(),
        ty: Type::OpaqueString,
    });
    let (rendered, rows) = capture(&contract);
    assert!(rendered.capabilities.circuits[0].recorded);
    assert_eq!(rows[0].body_owner, Some("algebra.ordinary"));
    assert!(matches!(rows[0].outcome, Outcome::Supported));

    // Existing checked layout policy can still decline an otherwise native
    // interface. This preserves a real final-guard refusal after a selected
    // generic return, rather than pretending the now-qualified opaque key fails.
    contract.stateful_circuits[0].parameters.push(Parameter {
        name: "oversized".into(),
        ty: Type::Vector {
            element: Box::new(Type::Field),
            length: crate::resource_limits::recording_fixed_layout_limit() + 1,
        },
    });
    let (rendered, rows) = capture(&contract);
    assert!(!rendered.capabilities.circuits[0].recorded);
    assert_eq!(rows[0].body_owner, Some("return.dispatch"));
    assert_eq!(rows[0].return_dispatch, Some("return.dispatch"));
    assert!(matches!(rows[0].outcome, Outcome::Unsupported { .. }));
}

#[test]
fn actual_observed_call_collisions_are_not_inferred_from_recording_support() {
    let mut contract = fixtures::counter();
    let mut collision = contract.stateful_circuits[0].clone();
    collision.name = "read_call".into();
    contract.stateful_circuits.push(collision);
    let (rendered, rows) = capture(&contract);
    let joined = attribution::join(&rendered.capabilities, &inspect(&contract), &rows);
    let row = joined
        .exports
        .iter()
        .find(|row| row.circuit == "read")
        .unwrap();
    assert!(row.recorded);
    assert!(!row.observed_call);
    assert!(matches!(row.outcome, Some(Outcome::Supported)));
}

#[test]
fn actual_helper_previews_and_final_exports_are_separated() {
    let contract = fixtures::shared_helper();
    let (rendered, rows) = capture(&contract);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|row| row.recorded)
    );
    assert!(
        rows.iter()
            .any(|row| row.phase == Phase::HelperPlanning && row.helper)
    );
    assert!(
        rows.iter()
            .any(|row| row.phase == Phase::HelperPlanning && !row.helper)
    );
    let exports: Vec<_> = rows
        .iter()
        .filter(|row| row.phase == Phase::PublicEmission && !row.helper && !row.internal)
        .collect();
    assert_eq!(exports.len(), 2);
    assert!(
        exports
            .iter()
            .all(|row| matches!(row.outcome, Outcome::Supported))
    );
}

#[test]
fn retired_audited_local_handler_yields_checked_local_and_public_effect_composition() {
    for local in [true, false] {
        let contract = fixtures::audited_local(local);
        let (rendered, rows) = capture(&contract);
        let row = rows
            .iter()
            .find(|row| {
                row.phase == Phase::PublicEmission && !row.helper && row.circuit == "verify"
            })
            .unwrap();
        assert_eq!(row.body_owner, Some("algebra.unit-actions"));
        assert!(matches!(row.outcome, Outcome::Supported));
        assert!(row.action_dispatch.is_none() && row.return_dispatch.is_none());
        assert!(rendered.capabilities.circuits[0].recorded);
        assert!(rendered.capabilities.circuits[0].observed_call);
        assert!(
            !rows
                .iter()
                .any(|row| row.body_owner == Some("early.audited-local"))
        );
        // The old native-only local audit refused this transitive public write.
        // Current checked composition records it rather than calling it unseen.
        if !local {
            assert!(rendered.source.contains("record_write"));
        }
    }
}

#[test]
fn native_validation_errors_do_not_get_an_invented_recording_owner() {
    let mut contract = fixture();
    contract.stateful_circuits[0].return_value = StateReturn::Effectful {
        body: ReturnPlan::Value {
            value: Expr::Parameter {
                name: "missing".into(),
            },
        },
    };
    let capture = Capture::begin();
    assert!(crate::render_with_capabilities(&contract).is_err());
    assert!(capture.finish().is_empty());
}
