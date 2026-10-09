// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! The original and mitigated source have the same selected runtime program.
use compact_rust_effect_plan_jubjub_source_fixture::ledger_contract as hoisted;
use compact_rust_guarded_point_read_source_fixture::{ledger_contract as original, runtime as r};
use r::context::ConstructorContext;
use r::{Field, JubjubPoint};

#[test]
fn both_original_branches_preserve_execution_and_transcript() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../jubjub-scalar-cell/oracle/boundaries.json"
    ))
    .unwrap();
    let mut cases = 0;
    for point in [
        JubjubPoint::identity(),
        r::ec_mul_generator(Field::from(7_u64)).unwrap(),
    ] {
        for row in data["values"].as_array().unwrap() {
            let scalar =
                Field::from_le_bytes(&hex::decode(row["hex_le"].as_str().unwrap()).unwrap())
                    .unwrap();
            for selected in [false, true] {
                let context = || {
                    original::initial_state(ConstructorContext::new(7_u64))
                        .unwrap()
                        .into_circuit_context(Default::default())
                };
                let native = original::read_branch(context(), selected, point, scalar).unwrap();
                let recorded =
                    original::recorded::read_branch(context(), selected, point, scalar).unwrap();
                let reference =
                    hoisted::recorded::read_branch(context(), selected, point, scalar).unwrap();
                let reduced = r::jubjub_scalar_from_native(scalar);
                let expected = if selected {
                    r::ec_add(point, r::ec_mul_generator(reduced).unwrap())
                } else {
                    r::ec_mul(point, reduced).unwrap()
                };
                assert_eq!(native.result, expected);
                for execution in [&recorded.execution, &reference.execution] {
                    assert_eq!(execution.result, native.result);
                    assert_eq!(execution.context.query.state, native.context.query.state);
                    assert_eq!(
                        execution.context.query.effects,
                        native.context.query.effects
                    );
                    assert_eq!(execution.gas_cost, native.gas_cost);
                    assert_eq!(execution.context.private_state, 7);
                    assert!(execution.private_transcript_outputs.is_empty());
                }
                assert_eq!(
                    serde_json::to_value(recorded.public.verify_ops()).unwrap(),
                    serde_json::to_value(reference.public.verify_ops()).unwrap()
                );
                let replay = recorded
                    .public
                    .initial()
                    .query(
                        recorded.public.verify_ops(),
                        None,
                        &recorded.execution.context.cost_model,
                    )
                    .unwrap();
                let reference_replay = reference
                    .public
                    .initial()
                    .query(
                        reference.public.verify_ops(),
                        None,
                        &reference.execution.context.cost_model,
                    )
                    .unwrap();
                assert_eq!(replay.context.state, native.context.query.state);
                assert_eq!(replay.context.effects, native.context.query.effects);
                assert_eq!(replay.gas_cost, reference_replay.gas_cost);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 28);
}
