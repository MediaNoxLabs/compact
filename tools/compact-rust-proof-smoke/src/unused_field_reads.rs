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

//! Prove original PM-19252 unit returns whose unused bindings still read state.
use super::*;
use midnight_compact_runtime::ledger::ContractAddress;

pub(super) fn run(seven: &Path, eight_a: &Path, eight_b: &Path) -> Result<(), Box<dyn Error>> {
    macro_rules! prove_read {
        ($fixture:ident, $root:expr, $circuit:ident, $call:ident, $input:expr, [$($arg:expr),*], [$($seed:ident)?], $rng:expr) => {{
            use $fixture::ledger_contract as contract;
            let root = $root;
            let mut rng = StdRng::seed_from_u64($rng);
            let initial = contract::initial_state(ConstructorContext::new(()))?;
            let context = initial.into_circuit_context(ContractAddress::default());
            $(let context = contract::$seed(context, Field::from(21u64))?.context;)?
            let initial_state = context.query.state.get_ref().clone();
            let deploy = make_deploy(root, stringify!($circuit), initial_state.clone(), &mut rng)?;
            let observed = ObservedContractState::new(deploy.address(), deploy.initial_state.clone(), Observation {
                transaction_hash: [0; 32], block_hash: [0; 32], block_height: 0,
            });
            let native = contract::$circuit(observed.circuit_context(()), $($arg),*)?;
            let recorded = contract::recorded::$circuit(observed.circuit_context(()), $($arg),*)?;
            if native.context.query.state.get_ref() != &initial_state
                || recorded.execution.context.query.state.get_ref() != &initial_state
                || native.gas_cost != recorded.execution.gas_cost
                || !recorded.execution.private_transcript_outputs.is_empty() {
                return Err("discarded Field read changed state or execution behavior".into());
            }
            let manual = check_generated_trace(root, stringify!($circuit), recorded, $input)?;
            let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(root.join(format!("keys/{}.verifier", stringify!($circuit))))?))?;
            let prepared = contract::recorded::Contract.$call(&observed, (), $($arg),*)?.prepare(verifier, Fr::from(0u64))?;
            if format!("{manual:?}") != format!("{prepared:?}") { return Err("discarded Field read observed call differs from direct recording".into()); }
            check_transaction(root, stringify!($circuit), deploy, prepared, &mut rng, |state| {
                if state.data.get_ref() != &initial_state { return Err("discarded Field read proof changed ledger state".into()); }
                Ok(())
            })?;
        }};
    }
    prove_read!(
        compact_rust_pm_19252_unused_read_seven_fixture,
        seven,
        test,
        test_call,
        Field::from(9u64),
        [Field::from(9u64)],
        [],
        0x0154_0001
    );
    prove_read!(
        compact_rust_pm_19252_unused_read_eight_a_fixture,
        eight_a,
        test1,
        test1_call,
        (),
        [],
        [test],
        0x0154_0002
    );
    prove_read!(
        compact_rust_pm_19252_unused_read_eight_b_fixture,
        eight_b,
        test,
        test_call,
        (),
        [],
        [test1],
        0x0154_0003
    );
    println!("three original unused Field read calls proved and applied without ledger changes");
    Ok(())
}
