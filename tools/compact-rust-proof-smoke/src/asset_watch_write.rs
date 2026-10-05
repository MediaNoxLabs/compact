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

//! Prove typed asset watch Add and Drop with a Unicode opaque key.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{ListMutation, RecordMutation};
use midnight_compact_runtime as runtime;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "setWatch";
    let key = runtime::OpaqueString::from("record-α-1");
    for drop in [false, true] {
        let witnesses = super::asset_writable::AssetWitness;
        let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
        if initial.private_state != 10 {
            return Err("asset watch constructor private state differs from TypeScript".into());
        }
        let context = asset::setRecord(
            initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
            &witnesses,
            key.clone(),
            super::asset_record_write::record(false)?,
            RecordMutation::Insert,
        )?
        .context;
        let context = if drop {
            asset::setWatch(context, &witnesses, key.clone(), ListMutation::Add)?.context
        } else {
            context
        };
        let private_state = context.private_state;
        if private_state != if drop { 12 } else { 11 } {
            return Err("asset watch prestate has wrong private witness count".into());
        }
        let mutation = if drop {
            ListMutation::Drop
        } else {
            ListMutation::Add
        };
        let mut rng = StdRng::seed_from_u64(if drop { 0x0161_5002 } else { 0x0161_5001 });
        let deploy = make_deploy(
            root,
            CIRCUIT,
            context.query.state.get_ref().clone(),
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
        let native = asset::setWatch(
            observed.circuit_context(private_state),
            &witnesses,
            key.clone(),
            mutation,
        )?;
        let recorded = asset::recorded::setWatch(
            observed.circuit_context(private_state),
            &witnesses,
            key.clone(),
            mutation,
        )?;
        if native.context.query.state.get_ref() != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
            || native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        {
            return Err(format!("asset watch drop={drop} recording differs from native").into());
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[
            AlignedValue::from(key.clone()),
            AlignedValue::from(mutation),
        ]);
        let manual = check_generated_trace(root, CIRCUIT, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/setWatch.verifier"),
        )?))?;
        let typed = asset::Contract::from(witnesses)
            .recording()
            .setWatch_call(&observed, private_state, key.clone(), mutation)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("asset watch drop={drop} typed call differs from recording").into(),
            );
        }
        check_transaction(root, CIRCUIT, deploy, typed, &mut rng, |applied| {
            let data = applied.data.get_ref();
            if data != &expected_state {
                return Err("proven asset watch changed expected native state".into());
            }
            if set_view_at_path::<runtime::OpaqueString, _>(data, &[1, 13])?.member(key.clone())
                == drop
            {
                return Err("proven asset watch stored wrong membership".into());
            }
            let StateValue::Array(fields) = data else {
                return Err("asset watch root is not an array".into());
            };
            let Some(StateValue::Array(metadata)) = fields.get(1) else {
                return Err("asset watch metadata is not an array".into());
            };
            if read_counter(metadata.get(5).ok_or("missing recordCount")?)? != 1 {
                return Err("proven asset watch changed record count".into());
            }
            let expected_writes = if drop { 3 } else { 2 };
            for index in [8, 9] {
                if read_counter(metadata.get(index).ok_or("missing watch write counter")?)?
                    != expected_writes
                {
                    return Err("proven asset watch stored wrong write count".into());
                }
            }
            Ok(())
        })?;
        println!("typed asset watch drop={drop} proved and applied through ledger-8");
    }
    Ok(())
}
