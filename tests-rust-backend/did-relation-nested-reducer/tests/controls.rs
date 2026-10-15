// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_did_relation_nested_reducer_fixture::{ledger_contract as c, runtime as r, types};
use r::context::ConstructorContext;

fn method(id: &str, curve: types::Curve, x: &str) -> types::Method {
    types::Method {
        id: r::OpaqueString(id.into()),
        publicKeyJwk: types::Jwk {
            curve,
            x: r::OpaqueString(x.into()),
        },
    }
}

#[test]
fn native_seed_inserts_and_replaces_complete_nested_record() {
    let original = method("constructor-control", types::Curve::Ed25519, "original");
    let initial = c::initial_state(ConstructorContext::new(73u64), original.clone()).unwrap();
    let inserted = method("référence-東京", types::Curve::X25519, "clé-東京");
    let seeded = c::seed(
        initial.into_circuit_context(Default::default()),
        inserted.clone(),
    )
    .unwrap();
    assert_eq!(seeded.context.private_state, 73);
    assert!(seeded.private_transcript_outputs.is_empty());
    let view = c::PublicStateView::from(&seeded.context);
    let methods = view.methods().unwrap();
    assert_eq!(methods.size().unwrap().value(), 2);
    assert_eq!(methods.lookup(original.id.clone()).unwrap(), original);
    assert_eq!(methods.lookup(inserted.id.clone()).unwrap(), inserted);
    let checked = c::check(seeded.context, inserted.id.clone(), true).unwrap();
    let replacement = method("référence-東京", types::Curve::Ed25519, "");
    let replaced = c::seed(checked.context, replacement.clone()).unwrap();
    assert_eq!(replaced.context.private_state, 73);
    assert!(replaced.private_transcript_outputs.is_empty());
    let view = c::PublicStateView::from(&replaced.context);
    let methods = view.methods().unwrap();
    assert_eq!(methods.size().unwrap().value(), 2);
    assert_eq!(methods.lookup(original.id.clone()).unwrap(), original);
    assert_eq!(methods.lookup(replacement.id.clone()).unwrap(), replacement);
    let checked = c::check(replaced.context, replacement.id.clone(), false).unwrap();
    assert_eq!(checked.context.private_state, 73);
    assert!(checked.private_transcript_outputs.is_empty());
    let error = c::check(checked.context, replacement.id, true)
        .err()
        .unwrap();
    assert_eq!(
        error,
        r::CompactError::AssertionFailed("wrong curve".into())
    );
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn recorded_seed_matches_native_manual_and_observed_insert_and_replace() {
    use compact_rust_did_relation_nested_reducer_fixture::ledger_slots;
    use r::recording::RecordingFrame;
    use r::transaction::{Observation, ObservedContractState};

    let original = method("constructor-control", types::Curve::Ed25519, "original");
    let mut context = c::initial_state(ConstructorContext::new(73u64), original.clone())
        .unwrap()
        .into_circuit_context(Default::default());
    for value in [
        method("référence\0東京", types::Curve::X25519, "clé-東京"),
        method("référence\0東京", types::Curve::Ed25519, ""),
    ] {
        let observed = ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                context.query.state.get_ref().clone(),
                Default::default(),
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = c::seed(observed.circuit_context(73), value.clone()).unwrap();
        let recorded = c::recorded::seed(observed.circuit_context(73), value.clone()).unwrap();
        let call = c::recorded::Contract
            .seed_call(&observed, 73, value.clone())
            .unwrap();
        let manual = ledger_slots::methods
            .record_insert(
                RecordingFrame::new(observed.circuit_context(73)),
                value.id.clone(),
                value.clone(),
            )
            .unwrap()
            .finish(());
        for actual in [&recorded, call.recorded(), &manual] {
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(actual.execution.context.private_state, 73);
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert_eq!(
                actual.execution.private_transcript_outputs,
                native.private_transcript_outputs
            );
            assert!(actual.execution.private_transcript_outputs.is_empty());
            assert!(!actual.public.verify_ops().is_empty());
            assert_eq!(
                serde_json::to_value(actual.public.verify_ops()).unwrap(),
                serde_json::to_value(manual.public.verify_ops()).unwrap()
            );
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            let reference = manual
                .public
                .initial()
                .query(manual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, reference.gas_cost);
        }
        let view = c::PublicStateView::from(&native.context);
        let methods = view.methods().unwrap();
        assert_eq!(methods.size().unwrap().value(), 2);
        assert_eq!(methods.lookup(original.id.clone()).unwrap(), original);
        assert_eq!(methods.lookup(value.id.clone()).unwrap(), value);
        context = native.context;
    }
}
