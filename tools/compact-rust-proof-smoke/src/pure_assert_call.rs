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

//! Prove successful pure assertions and guard intentional failures before VM effects.

use super::*;
use compact_rust_assert_parity_oracle_fixture::ledger_contract as assert_contract;
use compact_rust_pure_call_action_fixture::ledger_contract as pure_contract;
use midnight_compact_runtime::CompactError;

pub(super) fn run(pure_root: &Path, assert_root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0117_4153_5345_5254);
    let initial = pure_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        pure_root,
        "save",
        initial.ledger_state.get_ref().clone(),
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
    let failure = pure_contract::recorded::Contract
        .save_call(&observed, (), Field::from(0_u64))
        .err()
        .ok_or("save(0) unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("save(0) failed with the wrong error: {failure}").into());
    }
    let recorded = pure_contract::recorded::save(observed.circuit_context(()), Field::from(7_u64))?;
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(pure_root, "save", recorded, Field::from(7_u64))?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        pure_root.join("keys/save.verifier"),
    )?))?;
    let typed = pure_contract::recorded::Contract
        .save_call(&observed, (), Field::from(7_u64))?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("save(7) typed observed call differs from manual trace".into());
    }
    check_transaction(pure_root, "save", deploy, typed, &mut rng, |state| {
        let data = state.data.get_ref();
        if data != &expected_state {
            return Err("proven save(7) changed unexpected ledger state".into());
        }
        if read_cell_at_path::<Field, _>(data, &[0])? != Field::from(7_u64) {
            return Err("proven save(7) stored the wrong Field".into());
        }
        Ok(())
    })?;

    let initial = assert_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        assert_root,
        "trigger_ok",
        initial.ledger_state.get_ref().clone(),
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
    let failure = assert_contract::recorded::Contract
        .trigger_fail_call(&observed, ())
        .err()
        .ok_or("trigger_fail unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("trigger_fail failed with the wrong error: {failure}").into());
    }
    let recorded = assert_contract::recorded::trigger_ok(observed.circuit_context(()))?;
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(assert_root, "trigger_ok", recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        assert_root.join("keys/trigger_ok.verifier"),
    )?))?;
    let typed = assert_contract::recorded::Contract
        .trigger_ok_call(&observed, ())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("trigger_ok typed observed call differs from manual trace".into());
    }
    check_transaction(
        assert_root,
        "trigger_ok",
        deploy,
        typed,
        &mut rng,
        |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err("proven trigger_ok changed unexpected ledger state".into());
            }
            if !read_cell_at_path::<bool, _>(data, &[0])? {
                return Err("proven trigger_ok failed to store true".into());
            }
            Ok(())
        },
    )?;
    println!("closed pure assertions proved and applied; failing calls rejected before VM");
    Ok(())
}
