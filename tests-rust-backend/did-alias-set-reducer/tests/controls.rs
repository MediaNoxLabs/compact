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

use compact_rust_did_alias_set_reducer_fixture::{ledger_contract as c, runtime as r};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment};
use r::context::{ConstructorContext, ConstructorResult};
use sha2::{Digest, Sha256};

fn value(text: &str) -> r::OpaqueString {
    r::OpaqueString(text.into())
}

#[test]
fn native_controls_preserve_unrelated_alias_and_counter() {
    let initial = c::initial_state(ConstructorContext::new(73u64)).unwrap();
    let absent = c::member_control(
        initial.into_circuit_context(Default::default()),
        value("référence-東京"),
    )
    .unwrap();
    assert!(!absent.result);
    assert!(absent.private_transcript_outputs.is_empty());
    let unrelated = c::insert_control(absent.context, value("keep")).unwrap();
    assert!(unrelated.private_transcript_outputs.is_empty());
    let inserted = c::insert_control(unrelated.context, value("référence-東京")).unwrap();
    assert!(inserted.private_transcript_outputs.is_empty());
    let before_read = inserted.context.query.state.clone();
    let present = c::member_control(inserted.context, value("référence-東京")).unwrap();
    assert!(present.result);
    assert_eq!(present.context.query.state, before_read);
    assert!(present.private_transcript_outputs.is_empty());
    let view = c::PublicStateView::from(&present.context);
    assert_eq!(view.aliases().unwrap().size().unwrap().value(), 2);
    assert_eq!(view.count().unwrap().value(), 0);
    assert_eq!(present.context.private_state, 73);
    let removed = c::remove_control(present.context, value("référence-東京")).unwrap();
    assert!(removed.private_transcript_outputs.is_empty());
    let before_read = removed.context.query.state.clone();
    let absent = c::member_control(removed.context, value("référence-東京")).unwrap();
    assert!(!absent.result);
    assert_eq!(absent.context.query.state, before_read);
    assert!(absent.private_transcript_outputs.is_empty());
    assert_eq!(absent.context.private_state, 73);
    let view = c::PublicStateView::from(&absent.context);
    assert!(view.aliases().unwrap().member(value("keep")));
    assert_eq!(view.aliases().unwrap().size().unwrap().value(), 1);
    assert_eq!(view.count().unwrap().value(), 0);
}

#[test]
fn recorded_insert_and_membership_match_native_with_local_replay() {
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!(
            "../../../tools/compact-rust-backend/tests/set-composition/set_string.compact"
        ))
        .into(),
        generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
    };
    let mut lab = ContractLab::from_constructor(
        identity,
        Environment::new(Default::default(), Default::default(), [0; 32]),
        c::initial_state(ConstructorContext::new(73u64)).unwrap(),
    )
    .unwrap();
    for text in ["", "référence-東京"] {
        for expected in [false, true] {
            let before = lab.snapshot();
            let context = ConstructorResult::new(
                ConstructorContext::new(73u64),
                before.public_state().clone(),
            )
            .into_circuit_context(Default::default());
            let native = c::member_control(context, value(text)).unwrap();
            let recorded = lab
                .recorded(|ctx| c::recorded::member_control(ctx, value(text)))
                .unwrap();
            assert_eq!(native.result, expected);
            assert_eq!(*recorded.output(), expected);
            assert_eq!(recorded.public_state(), before.public_state());
            assert_eq!(recorded.public_state(), &native.context.query.state);
            assert_eq!(recorded.effects(), &native.context.query.effects);
            assert_eq!(recorded.execution_gas(), native.gas_cost);
            assert!(recorded.private_outputs().is_empty());
            assert!(!recorded.replay().unwrap().program().is_empty());
            assert_eq!(*lab.private_state(), 73);
            if !expected {
                let context = ConstructorResult::new(
                    ConstructorContext::new(73u64),
                    lab.snapshot().public_state().clone(),
                )
                .into_circuit_context(Default::default());
                let native = c::insert_control(context, value(text)).unwrap();
                let recorded = lab
                    .recorded(|ctx| c::recorded::insert_control(ctx, value(text)))
                    .unwrap();
                assert_eq!(recorded.public_state(), &native.context.query.state);
                assert_eq!(recorded.effects(), &native.context.query.effects);
                assert_eq!(recorded.execution_gas(), native.gas_cost);
                assert!(native.private_transcript_outputs.is_empty());
                assert!(recorded.private_outputs().is_empty());
                assert!(!recorded.replay().unwrap().program().is_empty());
                assert_eq!(native.context.private_state, 73);
                assert_eq!(*lab.private_state(), 73);
            }
        }
    }
    let snapshot = lab.snapshot();
    let view = c::PublicStateView::from(snapshot.public_state().get_ref());
    assert_eq!(view.aliases().unwrap().size().unwrap().value(), 2);
    assert_eq!(view.count().unwrap().value(), 0);
}
