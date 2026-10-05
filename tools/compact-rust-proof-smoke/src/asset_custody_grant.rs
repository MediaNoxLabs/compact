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

//! Prove typed custody-grant Insert and Update with nonempty opaque keys.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{
    ContractAddress as Holder, CustodyGrant, RecordMutation,
};
use midnight_compact_runtime as runtime;

fn grant(revised: bool) -> Result<CustodyGrant, Box<dyn Error>> {
    Ok(CustodyGrant {
        code: FixedBytes::new([if revised { 3 } else { 0 }; 32]),
        holder: Holder {
            bytes: FixedBytes::new([if revised { 8 } else { 7 }; 32]),
        },
        grantedAt: BoundedUint::new(if revised { 110 } else { 100 })?,
    })
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "setCustodyGrant";
    let key = runtime::OpaqueString::from("grant-note-1");
    for update in [false, true] {
        let witnesses = super::asset_writable::AssetWitness;
        let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
        if initial.private_state != 10 {
            return Err("custody grant constructor private state differs from TypeScript".into());
        }
        let context = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
        let context = if update {
            asset::setCustodyGrant(
                context,
                &witnesses,
                key.clone(),
                grant(false)?,
                RecordMutation::Insert,
            )?
            .context
        } else {
            context
        };
        let private_state = context.private_state;
        if private_state != if update { 11 } else { 10 } {
            return Err("custody grant prestate has wrong private witness count".into());
        }
        let value = grant(update)?;
        let mutation = if update {
            RecordMutation::Update
        } else {
            RecordMutation::Insert
        };
        let mut rng = StdRng::seed_from_u64(if update { 0x0151_5002 } else { 0x0151_5001 });
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
        let native = asset::setCustodyGrant(
            observed.circuit_context(private_state),
            &witnesses,
            key.clone(),
            value.clone(),
            mutation,
        )?;
        let recorded = asset::recorded::setCustodyGrant(
            observed.circuit_context(private_state),
            &witnesses,
            key.clone(),
            value.clone(),
            mutation,
        )?;
        if native.context.query.state.get_ref() != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
            || native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        {
            return Err(
                format!("custody grant update={update} recording differs from native").into(),
            );
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[
            AlignedValue::from(key.clone()),
            AlignedValue::from(value.clone()),
            AlignedValue::from(mutation),
        ]);
        let manual = check_generated_trace(root, CIRCUIT, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/setCustodyGrant.verifier"),
        )?))?;
        let typed = asset::Contract::from(witnesses)
            .recording()
            .setCustodyGrant_call(
                &observed,
                private_state,
                key.clone(),
                value.clone(),
                mutation,
            )?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("custody grant update={update} typed call differs from recording").into(),
            );
        }
        check_transaction(root, CIRCUIT, deploy, typed, &mut rng, |applied| {
            let data = applied.data.get_ref();
            if data != &expected_state {
                return Err("proven custody grant changed expected native state".into());
            }
            let stored =
                map_view_at_path::<runtime::OpaqueString, CustodyGrant, _>(data, &[1, 11])?
                    .lookup(key.clone())?;
            if stored != value {
                return Err("proven custody grant stored wrong typed value".into());
            }
            let StateValue::Array(fields) = data else {
                return Err("custody grant root is not an array".into());
            };
            let Some(StateValue::Array(metadata)) = fields.get(1) else {
                return Err("custody grant metadata is not an array".into());
            };
            let expected_count = if update { 2 } else { 1 };
            for index in [8, 9] {
                if read_counter(metadata.get(index).ok_or("missing grant write counter")?)?
                    != expected_count
                {
                    return Err("proven custody grant stored wrong write count".into());
                }
            }
            Ok(())
        })?;
        println!("typed custody grant update={update} proved and applied through ledger-8");
    }
    Ok(())
}
