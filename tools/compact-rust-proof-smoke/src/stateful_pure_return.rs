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

//! Prove a scalar pure Field return after a recorded Cell write.

use super::*;
use compact_rust_stateful_pure_call_fixture::ledger_contract as stateful_pure;
use pure_field_arguments::{check_stored_field, observed_state, verifier};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0100_5055_5245_5245);
    let initial = stateful_pure::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "save",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let value = Field::from(7_u64);
    let recorded =
        stateful_pure::recorded::save(initial.into_circuit_context(deploy.address()), value)?;
    if recorded.execution.result != Field::from(49_u64)
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("scalar pure return or private effects differ from Compact".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "save", recorded, value)?;
    let observed = observed_state(&deploy);
    let typed = stateful_pure::recorded::Contract.save_call(&observed, (), value)?;
    let prepared = typed.prepare(verifier(root, "save")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("pure Field return observed call differs from direct recording".into());
    }
    check_transaction(root, "save", deploy, prepared, &mut rng, |state| {
        check_stored_field(state, &expected_state, 0, 49)
    })?;
    println!("scalar pure Field return after a state action proved on ledger-8");
    Ok(())
}
