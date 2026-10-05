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

//! Prove the two asset writes gated by a Boolean conditional assertion.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::ContractAddress as Holder;
use midnight_compact_runtime as runtime;

pub(super) struct AssetWitness;

impl asset::Witnesses<u64> for AssetWitness {
    fn localOperatorKey(
        &self,
        context: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        (
            *context.private_state + 1,
            runtime::hash_to_curve(Field::from(1_u64)),
        )
    }

    fn localAuditorKey(
        &self,
        context: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        (
            *context.private_state + 1,
            runtime::hash_to_curve(Field::from(2_u64)),
        )
    }

    fn currentTimestamp(
        &self,
        context: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, BoundedUint<{ u64::MAX as u128 }>) {
        context.ledger.recordCount().unwrap();
        context.ledger.records().unwrap();
        context.ledger.watchList().unwrap();
        (
            *context.private_state + 1,
            BoundedUint::new(1_700_000_000).unwrap(),
        )
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0097_4153_5345_5457);
    for circuit in ["setCustodian", "tag"] {
        let witnesses = AssetWitness;
        let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
        if initial.private_state != 10 {
            return Err("asset constructor private state differs from TypeScript".into());
        }
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let holder = Holder {
            bytes: FixedBytes::new([7; 32]),
        };
        let manual = if circuit == "setCustodian" {
            let recorded = asset::recorded::setCustodian(context, &witnesses, holder.clone())?;
            check_generated_trace(root, circuit, recorded, holder.clone())?
        } else {
            let recorded = asset::recorded::tag(context, &witnesses, Field::from(7_u64))?;
            check_generated_trace(root, circuit, recorded, Field::from(7_u64))?
        };
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let contract = asset::Contract::from(witnesses);
        let call = if circuit == "setCustodian" {
            contract
                .recording()
                .setCustodian_call(&observed, 10_u64, holder.clone())?
                .prepare(verifier, Fr::from(0_u64))?
        } else {
            contract
                .recording()
                .tag_call(&observed, 10_u64, Field::from(7_u64))?
                .prepare(verifier, Fr::from(0_u64))?
        };
        if format!("{manual:?}") != format!("{call:?}") {
            return Err(format!("{circuit} observed call differs from manual prototype").into());
        }
        check_transaction(root, circuit, deploy, call, &mut rng, |state| {
            let data = state.data.get_ref();
            if circuit == "setCustodian" {
                if read_cell_at_path::<Holder, _>(data, &[0, 2])? != holder {
                    return Err("setCustodian proof stored the wrong custodian".into());
                }
            } else if !set_view_at_path::<Field, _>(data, &[1, 14])?.member(Field::from(7_u64)) {
                return Err("tag proof omitted the tag".into());
            }
            if read_cell_at_path::<BoundedUint<{ u64::MAX as u128 }>, _>(data, &[1, 3])?
                != BoundedUint::new(1_700_000_000)?
            {
                return Err(format!("{circuit} proof stored the wrong timestamp").into());
            }
            let StateValue::Array(root_fields) = data else {
                return Err("asset registry state is not an array".into());
            };
            let Some(StateValue::Array(metadata)) = root_fields.get(1) else {
                return Err("asset registry metadata is not an array".into());
            };
            for (index, name) in [(8, "revision"), (9, "writeCount")] {
                if read_counter(metadata.get(index).ok_or("missing asset counter")?)? != 1 {
                    return Err(format!("{circuit} proof stored the wrong {name}").into());
                }
            }
            Ok(())
        })?;
    }
    println!("asset writable assertions proved and applied through ledger-8");
    Ok(())
}
