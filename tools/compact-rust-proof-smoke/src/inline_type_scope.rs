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

//! Prove each helper-formal persistent-hash assertion from the original source.

use super::*;
use compact_rust_inline_type_scope_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let pair = |a, b| FixedVector::new([Field::from(a), Field::from(b)]);
    let scalar = Field::from(5_u64);
    let aggregate = FixedVector::new([
        Field::from(1_u64),
        Field::from(2_u64),
        Field::from(3_u64),
        Field::from(4_u64),
    ]);

    macro_rules! prove_case {
        ($circuit:literal, $method:ident, $call:ident, $hash:expr, $expected:expr,
         $input:expr, $seed:expr, [$($arg:expr),* $(,)?]) => {{
            let hash: FixedBytes<32> = $hash;
            let expected_vector: FixedVector<Field, 2> = $expected;
            let initial = contract::initial_state(ConstructorContext::new(()))?;
            let seeded = contract::setHash(
                initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
                hash,
            )?;
            let pre_state = seeded.context.query.state.get_ref().clone();
            let mut rng = StdRng::seed_from_u64($seed);
            let deploy = make_deploy(root, $circuit, pre_state, &mut rng)?;
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let native = contract::$method(observed.circuit_context(()), $($arg.clone()),*)?;
            let recorded = contract::recorded::$method(
                observed.circuit_context(()), $($arg.clone()),*
            )?;
            if native.result != ()
                || recorded.execution.result != ()
                || !recorded.execution.private_transcript_outputs.is_empty()
                || native.gas_cost != recorded.execution.gas_cost
                || native.context.query.effects != recorded.execution.context.query.effects
                || native.context.query.state.get_ref()
                    != recorded.execution.context.query.state.get_ref()
            {
                return Err(format!("{} recording differs from native execution", $circuit).into());
            }
            let expected_state = native.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, $circuit, recorded, $input)?;
            let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
                root.join(concat!("keys/", $circuit, ".verifier")),
            )?))?;
            let typed = contract::recorded::Contract
                .$call(&observed, (), $($arg.clone()),*)?
                .prepare(verifier, Fr::from(0_u64))?;
            if format!("{manual:?}") != format!("{typed:?}") {
                return Err(format!("{} observed call differs from recording", $circuit).into());
            }
            check_transaction(root, $circuit, deploy, typed, &mut rng, |applied| {
                if applied.data.get_ref() != &expected_state {
                    return Err(format!("{} applied state differs from native", $circuit).into());
                }
                let view = contract::PublicStateView::from(applied);
                if view.fieldVec()? != expected_vector || view.hashCell()? != hash {
                    return Err(format!("{} applied wrong typed Cell values", $circuit).into());
                }
                Ok(())
            })?;
            println!("{} helper-formal hash assertion proved and applied", $circuit);
        }};
    }

    let scalar_pair = pair(9, 10);
    prove_case!(
        "checkScalarScope",
        checkScalarScope,
        checkScalarScope_call,
        runtime::persistent_hash(scalar),
        scalar_pair.clone(),
        (scalar_pair.clone(), scalar),
        0x0153_5001,
        [scalar_pair, scalar]
    );
    let aggregate_pair = pair(7, 8);
    prove_case!(
        "checkAggScope",
        checkAggScope,
        checkAggScope_call,
        runtime::persistent_hash(aggregate.clone()),
        aggregate_pair.clone(),
        (aggregate_pair.clone(), aggregate.clone()),
        0x0153_5002,
        [aggregate_pair, aggregate]
    );
    let collision_pair = pair(3, 4);
    prove_case!(
        "checkNoCollisionScope",
        checkNoCollisionScope,
        checkNoCollisionScope_call,
        runtime::persistent_hash(collision_pair.clone()),
        collision_pair.clone(),
        collision_pair.clone(),
        0x0153_5003,
        [collision_pair]
    );
    Ok(())
}
