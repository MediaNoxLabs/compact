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

//! Prove a custody grant read from a state seeded with one valid grant.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{
    ContractAddress as Holder, CustodyGrant, RecordMutation,
};
use midnight_compact_runtime as runtime;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "assertGrantEffective";
    let witnesses = super::asset_writable::AssetWitness;
    let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
    if initial.private_state != 10 {
        return Err("asset constructor private state differs from TypeScript".into());
    }
    let key = runtime::OpaqueString::from("grant-1");
    let grant = CustodyGrant {
        code: FixedBytes::new([0; 32]),
        holder: Holder {
            bytes: FixedBytes::new([7; 32]),
        },
        grantedAt: BoundedUint::new(100)?,
    };
    // setCustodyGrant is currently native-only. Seed the deployment state
    // with its exact native transition, then prove the read-only guard.
    let seeded = asset::setCustodyGrant(
        initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
        &witnesses,
        key.clone(),
        grant.clone(),
        RecordMutation::Insert,
    )?;
    if seeded.context.private_state != 11 {
        return Err("custody grant insertion did not advance private state".into());
    }
    let seeded_state = seeded.context.query.state.get_ref().clone();
    let mut rng = StdRng::seed_from_u64(0x0097_4153_4752_414e);
    let deploy = make_deploy(root, CIRCUIT, seeded_state.clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let as_of = BoundedUint::new(120)?;
    let manual = check_generated_trace(
        root,
        CIRCUIT,
        asset::recorded::assertGrantEffective(
            observed.circuit_context(11_u64),
            key.clone(),
            as_of,
        )?,
        (key.clone(), as_of),
    )?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/assertGrantEffective.verifier"),
    )?))?;
    let call = asset::Contract::from(witnesses)
        .recording()
        .assertGrantEffective_call(&observed, 11_u64, key.clone(), as_of)?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{call:?}") {
        return Err("assertGrantEffective observed call differs from manual prototype".into());
    }
    check_transaction(root, CIRCUIT, deploy, call, &mut rng, |state| {
        let data = state.data.get_ref();
        if data != &seeded_state {
            return Err("effective grant proof changed read-only state".into());
        }
        let stored = map_view_at_path::<runtime::OpaqueString, CustodyGrant, _>(data, &[1, 11])?
            .lookup(key.clone())?;
        if stored != grant {
            return Err("effective grant proof read wrong composite grant".into());
        }
        Ok(())
    })?;
    println!("custody grant guard proved, verified, validated and applied through ledger-8");
    Ok(())
}
