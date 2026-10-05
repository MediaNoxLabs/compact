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

//! Prove typed asset-record Insert and Update with a Unicode opaque note.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{
    AssetClass, AssetRecord, Provenance, RecordMutation,
};
use midnight_compact_runtime as runtime;

fn record(revised: bool) -> Result<AssetRecord, Box<dyn Error>> {
    Ok(AssetRecord {
        code: FixedBytes::new([if revised { 3 } else { 0 }; 32]),
        note: runtime::OpaqueString::from(if revised {
            "更新済み café"
        } else {
            "検査資料 🔒"
        }),
        provenance: Provenance {
            facility: FixedBytes::new([if revised { 6 } else { 4 }; 32]),
            registeredAt: BoundedUint::new(if revised { 101 } else { 91 })?,
        },
        kind: if revised {
            AssetClass::Container
        } else {
            AssetClass::Instrument
        },
        quantity: BoundedUint::new(if revised { 9 } else { 5 })?,
    })
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "setRecord";
    let key = runtime::OpaqueString::from("record-α-1");
    for update in [false, true] {
        let witnesses = super::asset_writable::AssetWitness;
        let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
        if initial.private_state != 10 {
            return Err("asset record constructor private state differs from TypeScript".into());
        }
        let context = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
        let context = if update {
            asset::setRecord(
                context,
                &witnesses,
                key.clone(),
                record(false)?,
                RecordMutation::Insert,
            )?
            .context
        } else {
            context
        };
        let private_state = context.private_state;
        if private_state != if update { 11 } else { 10 } {
            return Err("asset record prestate has wrong private witness count".into());
        }
        let value = record(update)?;
        let mutation = if update {
            RecordMutation::Update
        } else {
            RecordMutation::Insert
        };
        let mut rng = StdRng::seed_from_u64(if update { 0x0157_5002 } else { 0x0157_5001 });
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
        let native = asset::setRecord(
            observed.circuit_context(private_state),
            &witnesses,
            key.clone(),
            value.clone(),
            mutation,
        )?;
        let recorded = asset::recorded::setRecord(
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
                format!("asset record update={update} recording differs from native").into(),
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
            root.join("keys/setRecord.verifier"),
        )?))?;
        let typed = asset::Contract::from(witnesses)
            .recording()
            .setRecord_call(
                &observed,
                private_state,
                key.clone(),
                value.clone(),
                mutation,
            )?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("asset record update={update} typed call differs from recording").into(),
            );
        }
        check_transaction(root, CIRCUIT, deploy, typed, &mut rng, |applied| {
            let data = applied.data.get_ref();
            if data != &expected_state {
                return Err("proven asset record changed expected native state".into());
            }
            let stored = map_view_at_path::<runtime::OpaqueString, AssetRecord, _>(data, &[1, 10])?
                .lookup(key.clone())?;
            if stored != value {
                return Err("proven asset record stored wrong typed value".into());
            }
            let StateValue::Array(fields) = data else {
                return Err("asset record root is not an array".into());
            };
            let Some(StateValue::Array(metadata)) = fields.get(1) else {
                return Err("asset record metadata is not an array".into());
            };
            let expected_count = if update { 2 } else { 1 };
            if read_counter(metadata.get(5).ok_or("missing recordCount")?)? != 1 {
                return Err("proven asset record stored wrong record count".into());
            }
            for index in [8, 9] {
                if read_counter(metadata.get(index).ok_or("missing record write counter")?)?
                    != expected_count
                {
                    return Err("proven asset record stored wrong write count".into());
                }
            }
            Ok(())
        })?;
        println!("typed asset record update={update} proved and applied through ledger-8");
    }
    Ok(())
}
