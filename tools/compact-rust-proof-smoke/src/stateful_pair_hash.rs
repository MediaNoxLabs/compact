// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Prove typed vector pure hash calls inside a shared stateful helper.

use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as contract;
use compact_rust_call_arg_declared_type_fixture::pure_circuits;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (circuit, seed) in [
        ("impureBare", 0x0108_4241_5245_4841),
        ("impureInIfArm", 0x0108_4946_4152_4d48),
    ] {
        let mut rng = StdRng::seed_from_u64(seed);
        let initial = contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = match circuit {
            "impureBare" => contract::recorded::impureBare(context)?,
            "impureInIfArm" => contract::recorded::impureInIfArm(context)?,
            _ => unreachable!(),
        };
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, ())?;

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
        let typed = match circuit {
            "impureBare" => contract::recorded::Contract.impureBare_call(&observed, ())?,
            "impureInIfArm" => contract::recorded::Contract.impureInIfArm_call(&observed, ())?,
            _ => unreachable!(),
        };
        let prepared = typed.prepare(verifier, Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err(format!("{circuit} typed call differs from the recorded call").into());
        }

        check_transaction(root, circuit, deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err(format!("{circuit} changed the expected ledger state").into());
            }
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err(format!("{circuit} state is not an array").into());
            };
            let stored: Field = read_cell(fields.get(3).ok_or("fieldCell missing")?)?;
            let expected =
                pure_circuits::sumVec(FixedVector::new([Field::from(0u64), Field::from(1u64)]))?;
            if stored != expected {
                return Err(format!("{circuit} stored a different vector hash").into());
            }
            Ok(())
        })?;
    }
    println!("typed stateful pair hashes proved and applied through ledger-8");
    Ok(())
}
