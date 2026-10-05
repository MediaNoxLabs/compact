// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use compact_rust_test_center_bboard_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
#[path = "../../../tests-rust-backend/test-center-bboard/support.rs"]
mod support;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut seeded = support::initial();
    for (index, text) in ["", "🌙 Midnight — 你好, Привіт"].into_iter().enumerate() {
        let message = runtime::OpaqueString::from(text);
        let mut rng = StdRng::seed_from_u64(0x166 + index as u64);
        let private = seeded.private_state;
        let deploy = make_deploy(root, "post", seeded.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = contract::post(
            observed.circuit_context(private),
            &support::Witness::default(),
            message.clone(),
        )?;
        let recorded = contract::recorded::post(
            observed.circuit_context(private),
            &support::Witness::default(),
            message.clone(),
        )?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || native.context.private_state != recorded.execution.context.private_state
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
        {
            return Err("post native/recorded mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "post", recorded, message.clone())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/post.verifier"),
        )?))?;
        let generated = contract::Contract::from(support::Witness::default());
        let prepared = generated
            .recording()
            .post_call(&observed, private, message.clone())?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("post observed-call mismatch".into());
        }
        check_transaction(root, "post", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected {
                return Err("post applied state mismatch".into());
            }
            Ok(())
        })?;
        seeded = support::rehydrate(native.context);
        let private = seeded.private_state;
        let deploy = make_deploy(
            root,
            "take_down",
            seeded.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = contract::take_down(
            observed.circuit_context(private),
            &support::Witness::default(),
        )?;
        let recorded = contract::recorded::take_down(
            observed.circuit_context(private),
            &support::Witness::default(),
        )?;
        if native.result != message
            || recorded.execution.result != message
            || native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || native.context.private_state != recorded.execution.context.private_state
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
        {
            return Err("take-down retained return/native mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "take_down", recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/take_down.verifier"),
        )?))?;
        let prepared = generated
            .recording()
            .take_down_call(&observed, private)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("take-down observed-call mismatch".into());
        }
        check_transaction(root, "take_down", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected {
                return Err("take-down applied state mismatch".into());
            }
            let view = contract::PublicStateView::from(applied);
            if view.message()?.is_some || view.instance()?.value() != 2 + index as u128 {
                return Err("clear/instance mismatch".into());
            }
            Ok(())
        })?;
        seeded = support::rehydrate(native.context);
        println!("original bboard cycle {index}: post and take_down verified and ledger-applied");
    }
    Ok(())
}
