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

//! Prove a scoped Field Cell update followed by an independent Field return.

use super::*;
use compact_rust_root_let_action_return_oracle_fixture::ledger_contract as contract;

fn validate_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let expected: serde_json::Value =
        serde_json::from_str(compact_rust_root_let_action_return_oracle_fixture::RETAINED_IR)?;
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    if expected != actual {
        return Err("root Let return source/proof IR mismatch".into());
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    validate_ir(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    const CIRCUIT: &str = "step";
    let mut rng = StdRng::seed_from_u64(0x4144_5231_3738);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let echo = Fr::from(9_u64);
    let recorded = contract::recorded::step(initial.into_circuit_context(deploy.address()), echo)?;
    if recorded.execution.result != echo
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("root Let changed its independent return or private transcript".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, CIRCUIT, recorded, echo)?;
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
        root.join("keys/step.verifier"),
    )?))?;
    let typed = contract::recorded::Contract.step_call(&observed, (), echo)?;
    let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("root Let observed call differs from direct recording".into());
    }
    check_transaction(root, CIRCUIT, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state
            || read_cell_at_path::<Fr, _>(state.data.get_ref(), &[0])? != Fr::from(1_u64)
        {
            return Err("root Let ledger application changed the expected Field Cell".into());
        }
        Ok(())
    })?;
    println!(
        "root Let Field update and independent return proved and ledger-applied under the shared unbalanced smoke policy"
    );
    Ok(())
}

#[cfg(test)]
mod source_pairing_tests {
    use super::*;
    #[test]
    fn rejects_mismatched_source_before_keys() {
        let source = compact_rust_root_let_action_return_oracle_fixture::RETAINED_IR;
        validate_ir(source.as_bytes()).unwrap();
        let mut altered: serde_json::Value = serde_json::from_str(source).unwrap();
        altered["stateful_circuits"][0]["name"] = "different".into();
        assert!(validate_ir(&serde_json::to_vec(&altered).unwrap()).is_err());
        assert!(
            validate_ir(
                compact_rust_terminal_lexical_return_oracle_fixture::RETAINED_IR.as_bytes()
            )
            .is_err()
        );
    }
}
