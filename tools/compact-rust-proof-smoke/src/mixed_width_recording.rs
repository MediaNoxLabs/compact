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

//! Prove the exact mixed-width guard before the Counter-one VM continuation.

use super::*;
use compact_rust_mixed_width_operand_oracle_fixture::ledger_contract as mixed;
use midnight_compact_runtime::CompactError;
use pure_field_arguments::{observed_state, verifier};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0134_4d49_5845_4457);
    let initial = mixed::initial_state(
        ConstructorContext::new(()),
        BoundedUint::<4294967295>::new(20)?,
        BoundedUint::<4294967295>::new(4)?,
    )?;
    let deploy = make_deploy(
        root,
        "recordMatching",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = observed_state(&deploy);
    let small = BoundedUint::<255>::new(7)?;
    let matching = BoundedUint::<4294967295>::new(7)?;
    let different = BoundedUint::<4294967295>::new(8)?;
    let failure = mixed::recorded::Contract
        .recordMatching_call(&observed, (), small, different)
        .err()
        .ok_or("recordMatching(7,8) unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("recordMatching(7,8) failed with wrong error: {failure}").into());
    }
    let recorded = mixed::recorded::recordMatching(
        initial.into_circuit_context(deploy.address()),
        small,
        matching,
    )?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("recordMatching private output changed".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "recordMatching", recorded, (small, matching))?;
    let typed = mixed::recorded::Contract.recordMatching_call(&observed, (), small, matching)?;
    let prepared = typed.prepare(verifier(root, "recordMatching")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("recordMatching observed call differs from direct recording".into());
    }
    check_transaction(
        root,
        "recordMatching",
        deploy,
        prepared,
        &mut rng,
        |state| {
            if state.data.get_ref() != &expected_state {
                return Err("proven recordMatching changed unexpected ledger state".into());
            }
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err("recordMatching state is not an array".into());
            };
            if read_counter(fields.get(1).ok_or("recordMatching Counter missing")?)? != 1 {
                return Err("proven recordMatching incremented Counter incorrectly".into());
            }
            Ok(())
        },
    )?;
    let initial = mixed::initial_state(
        ConstructorContext::new(()),
        BoundedUint::<4294967295>::new(20)?,
        BoundedUint::<4294967295>::new(4)?,
    )?;
    let deploy = make_deploy(
        root,
        "recordPinned",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = observed_state(&deploy);
    let q = BoundedUint::<4294967295>::new(4)?;
    let y = BoundedUint::<4294967295>::new(20)?;
    let rejected_q = BoundedUint::<4294967295>::new(1_073_741_824)?;
    let rejected_y = BoundedUint::<4294967295>::new(4_294_967_295)?;
    let failure = mixed::recorded::Contract
        .recordPinned_call(&observed, (), rejected_q, rejected_y)
        .err()
        .ok_or("recordPinned over bound unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("recordPinned over bound failed with wrong error: {failure}").into());
    }
    let recorded =
        mixed::recorded::recordPinned(initial.into_circuit_context(deploy.address()), q, y)?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("recordPinned private output changed".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "recordPinned", recorded, (q, y))?;
    let typed = mixed::recorded::Contract.recordPinned_call(&observed, (), q, y)?;
    let prepared = typed.prepare(verifier(root, "recordPinned")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("recordPinned observed call differs from direct recording".into());
    }
    check_transaction(root, "recordPinned", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven recordPinned changed unexpected ledger state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("recordPinned state is not an array".into());
        };
        if read_counter(fields.get(1).ok_or("recordPinned Counter missing")?)? != 1 {
            return Err("proven recordPinned incremented Counter incorrectly".into());
        }
        Ok(())
    })?;
    println!("two mixed-width guards proved, verified and applied through ledger-8");
    Ok(())
}
