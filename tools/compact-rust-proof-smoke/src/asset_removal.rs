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

//! Prove asset removal from a state seeded with one valid record.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{AssetClass, AssetRecord, RecordMutation};
use midnight_compact_runtime as runtime;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "removeRecord";
    let witnesses = super::asset_writable::AssetWitness;
    let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
    if initial.private_state != 10 {
        return Err("asset constructor private state differs from TypeScript".into());
    }
    let key = runtime::OpaqueString::from("asset-1");
    let record = AssetRecord {
        kind: AssetClass::Instrument,
        ..Default::default()
    };
    // setRecord is currently native-only. Seed the deployment state with that
    // exact native transition, then prove removal through the observed call.
    let seeded = asset::setRecord(
        initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
        &witnesses,
        key.clone(),
        record,
        RecordMutation::Insert,
    )?;
    if seeded.context.private_state != 11 {
        return Err("asset insertion did not advance private state".into());
    }
    let mut rng = StdRng::seed_from_u64(0x0097_4153_5245_4d4f);
    let deploy = make_deploy(
        root,
        CIRCUIT,
        seeded.context.query.state.get_ref().clone(),
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
    let manual = check_generated_trace(
        root,
        CIRCUIT,
        asset::recorded::removeRecord(observed.circuit_context(11_u64), &witnesses, key.clone())?,
        key.clone(),
    )?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/removeRecord.verifier"),
    )?))?;
    let call = asset::Contract::from(witnesses)
        .recording()
        .removeRecord_call(&observed, 11_u64, key.clone())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{call:?}") {
        return Err("removeRecord observed call differs from manual prototype".into());
    }
    check_transaction(root, CIRCUIT, deploy, call, &mut rng, |state| {
        let data = state.data.get_ref();
        if map_view_at_path::<runtime::OpaqueString, AssetRecord, _>(data, &[1, 10])?
            .member(key.clone())
        {
            return Err("asset removal proof left record in Map".into());
        }
        if !set_view_at_path::<runtime::OpaqueString, _>(data, &[1, 12])?.member(key.clone()) {
            return Err("asset removal proof omitted retired key".into());
        }
        let StateValue::Array(root_fields) = data else {
            return Err("asset registry state is not an array".into());
        };
        let Some(StateValue::Array(metadata)) = root_fields.get(1) else {
            return Err("asset registry metadata is not an array".into());
        };
        for (index, name) in [(8, "revision"), (9, "writeCount")] {
            if read_counter(metadata.get(index).ok_or("missing asset counter")?)? != 2 {
                return Err(format!("asset removal proof stored wrong {name}").into());
            }
        }
        Ok(())
    })?;
    println!("asset removal proved, verified, validated and applied through ledger-8");
    Ok(())
}
