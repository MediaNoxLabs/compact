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

//! Prove the typed wide unsigned cast and closed pure unsigned call slices.

use super::*;
use compact_rust_field_cast_uint128_fixture::ledger_contract as wide_cast;
use compact_rust_widening_arith_oracle_fixture::ledger_contract as widening;
use pure_field_arguments::{observed_state, verifier};

pub(super) fn wide_cast(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0118_5749_4445_4341);
    let initial = wide_cast::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "save",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let value = BoundedUint::<{ u128::MAX }>::new((1u128 << 80) + 7)?;
    let recorded =
        wide_cast::recorded::save(initial.into_circuit_context(deploy.address()), value)?;
    if recorded.execution.result != Field::from(value.value())
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("wide cast recorded result or private output changed".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "save", recorded, value)?;
    let observed = observed_state(&deploy);
    let typed = wide_cast::recorded::Contract.save_call(&observed, (), value)?;
    let prepared = typed.prepare(verifier(root, "save")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("wide cast observed call differs from direct recording".into());
    }
    check_transaction(root, "save", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven wide cast changed expected Cell state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("wide cast state is not an array".into());
        };
        let stored: Field = read_cell(fields.get(0).ok_or("wide cast Cell missing")?)?;
        if stored != Field::from(value.value()) {
            return Err("proven wide cast stored the wrong Field".into());
        }
        Ok(())
    })?;
    println!("wide unsigned Field cast proved and applied through ledger-8");
    Ok(())
}

pub(super) fn widening_call(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0118_4152_4541_4341);
    let initial = widening::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "recordArea",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let width = BoundedUint::<65535>::new(65535)?;
    let height = BoundedUint::<65535>::new(65535)?;
    let recorded = widening::recorded::recordArea(
        initial.into_circuit_context(deploy.address()),
        width,
        height,
    )?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("unsigned pure call produced private outputs".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "recordArea", recorded, (width, height))?;
    let observed = observed_state(&deploy);
    let typed = widening::recorded::Contract.recordArea_call(&observed, (), width, height)?;
    let prepared = typed.prepare(verifier(root, "recordArea")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("unsigned pure call observed call differs from direct recording".into());
    }
    check_transaction(root, "recordArea", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven unsigned pure call changed expected Counter state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("unsigned pure call state is not an array".into());
        };
        if read_counter(fields.get(0).ok_or("recordArea Counter missing")?)? != 1 {
            return Err("proven unsigned pure call incremented Counter incorrectly".into());
        }
        Ok(())
    })?;
    println!("closed pure unsigned call proved and applied through ledger-8");
    Ok(())
}
