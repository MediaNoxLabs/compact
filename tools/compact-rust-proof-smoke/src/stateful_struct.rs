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

//! Prove nonempty composite return paths; retain exact empty-call refusal.
use super::*;
use compact_rust_stateful_struct_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime::{self as runtime, BoundedUint};
struct Witness;
impl c::Witnesses<Vec<u8>> for Witness {
    fn next_value(
        &self,
        context: WitnessContext<'_, Vec<u8>, c::LedgerView<'_>>,
        tag: BoundedUint<255>,
    ) -> (Vec<u8>, BoundedUint<18446744073709551615>) {
        let result =
            BoundedUint::new(tag.value() * 10 + context.private_state.len() as u128).unwrap();
        let mut p = context.private_state.clone();
        p.push(tag.value() as u8);
        (p, result)
    }
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, name) in ["snapshot", "reverse", "nested"].into_iter().enumerate() {
        let mut rng = StdRng::seed_from_u64(0x0187_0000 + index as u64);
        let initial = c::initial_state(ConstructorContext::new(Vec::<u8>::new()))?;
        let expected_state = initial.ledger_state.get_ref().clone();
        let deploy = make_deploy(root, name, expected_state.clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let generated = c::Contract::from(Witness);
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        let (manual, prepared) = match name {
            "snapshot" => {
                let empty = generated
                    .recording()
                    .snapshot_call(&observed, vec![], false)?;
                if !matches!(
                    empty.prepare(verifier.clone(), Fr::from(0_u64)),
                    Err(ObservedCallError::Prepare(
                        runtime::transaction::PrepareCallError::EmptyTranscript
                    ))
                ) {
                    return Err(
                        "empty snapshot must refuse preparation without fabricated operations"
                            .into(),
                    );
                }
                println!(
                    "snapshot(false): zero operations retained; exact EmptyTranscript preparation refusal"
                );
                let recorded =
                    c::recorded::snapshot(observed.circuit_context(vec![]), &Witness, true)?;
                if recorded.execution.context.private_state != [1, 2] {
                    return Err("snapshot witness order".into());
                }
                let manual = check_generated_trace(root, name, recorded, true)?;
                let typed = generated
                    .recording()
                    .snapshot_call(&observed, vec![], true)?;
                (manual, typed.prepare(verifier, Fr::from(0_u64))?)
            }
            "reverse" => {
                let recorded = c::recorded::reverse(observed.circuit_context(vec![]), &Witness)?;
                if recorded.execution.context.private_state != [1, 2] {
                    return Err("normalized member order".into());
                }
                let manual = check_generated_trace(root, name, recorded, ())?;
                let typed = generated.recording().reverse_call(&observed, vec![])?;
                (manual, typed.prepare(verifier, Fr::from(0_u64))?)
            }
            "nested" => {
                let recorded = c::recorded::nested(observed.circuit_context(vec![]), &Witness)?;
                if recorded.execution.context.private_state != [1, 2, 3]
                    || recorded.execution.result.tail.value() != 32
                {
                    return Err("nested helper witness order/result".into());
                }
                let manual = check_generated_trace(root, name, recorded, ())?;
                let typed = generated.recording().nested_call(&observed, vec![])?;
                (manual, typed.prepare(verifier, Fr::from(0_u64))?)
            }
            _ => unreachable!(),
        };
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("composite observed/direct recording mismatch".into());
        }
        check_transaction(root, name, deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("composite read changed ledger state".into());
            }
            Ok(())
        })?;
        println!(
            "{name} composite return proved, verified, ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}
