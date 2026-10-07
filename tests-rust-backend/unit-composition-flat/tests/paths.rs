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

use compact_rust_unit_composition_flat_fixture as fixture;
use fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use r::context::{ConstructorContext, WitnessContext};
use serde_json::Value;
#[allow(dead_code)]
#[path = "../../support/oracle_recorded_trace.rs"]
mod trace;
struct Witness;
impl c::Witnesses<u64> for Witness {
    fn now(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> (u64, r::BoundedUint<18446744073709551615>) {
        (*ctx.private_state + 1, r::BoundedUint::new(42).unwrap())
    }
}
#[test]
fn selected_and_empty_branches_preserve_declared_paths() {
    let capture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/unit-composition-paths.json"
    ))
    .unwrap();
    let rows: Vec<_> = capture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["source"] == "flat")
        .collect();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let state: r::ledger::ContractState<r::ledger::DefaultDB> =
            midnight_serialize::tagged_deserialize(
                &mut hex::decode(row["before"].as_str().unwrap())
                    .unwrap()
                    .as_slice(),
            )
            .unwrap();
        let initial = c::initial_state(ConstructorContext::new(7)).unwrap();
        assert_eq!(initial.ledger_state, state.data);
        let take = row["take"].as_bool().unwrap();
        let context = || {
            c::initial_state(ConstructorContext::new(7))
                .unwrap()
                .into_circuit_context(r::ledger::ContractAddress::default())
        };
        let native =
            c::maybeClose(context(), &Witness, take, r::BoundedUint::new(0).unwrap()).unwrap();
        let recorded =
            c::recorded::maybeClose(context(), &Witness, take, r::BoundedUint::new(0).unwrap())
                .unwrap();
        if take {
            trace::assert_trace(&native, &recorded, row, |s| {
                let mut st = state.clone();
                st.data = r::ledger::ChargedState::new(s);
                let mut b = vec![];
                midnight_serialize::tagged_serialize(&st, &mut b).unwrap();
                hex::encode(b)
            });
        } else {
            assert_eq!(native.context.query.state, state.data);
            assert_eq!(recorded.execution.context.query.state, state.data);
            assert!(recorded.public.verify_ops().is_empty());
            assert!(recorded.execution.private_transcript_outputs.is_empty());
            assert_eq!(recorded.execution.context.private_state, 7);
            assert_eq!(row["before"], row["after"]);
            assert!(row["queries"].as_array().unwrap().is_empty());
        }
    }
}
#[cfg(feature = "ledger-transaction")]
#[test]
fn empty_selected_branch_is_execution_only_and_refuses_preparation() {
    use r::transaction::{CallSpec, PrepareCallError, prepare_call};
    use rand::{Rng, SeedableRng, rngs::StdRng};
    let result = c::recorded::maybeClose(
        c::initial_state(ConstructorContext::new(7))
            .unwrap()
            .into_circuit_context(r::ledger::ContractAddress::default()),
        &Witness,
        false,
        r::BoundedUint::new(0).unwrap(),
    )
    .unwrap();
    let spec = CallSpec::new(
        "maybeClose",
        StdRng::seed_from_u64(251).r#gen(),
        (
            false,
            r::BoundedUint::<18446744073709551615>::new(0).unwrap(),
        ),
        midnight_transient_crypto::curve::Fr::from(0),
    );
    assert!(matches!(
        prepare_call(result, spec),
        Err(PrepareCallError::EmptyTranscript)
    ));
}

#[test]
fn direct_close_changes_only_declared_state_and_checks_guards_before_witness() {
    use fixture::ledger_slots as slots;
    struct CountingWitness(std::cell::Cell<usize>);
    impl c::Witnesses<u64> for CountingWitness {
        fn now(
            &self,
            ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        ) -> (u64, r::BoundedUint<18446744073709551615>) {
            self.0.set(self.0.get() + 1);
            (*ctx.private_state + 1, r::BoundedUint::new(42).unwrap())
        }
    }
    let context = || {
        c::initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(Default::default())
    };
    let nw = CountingWitness(std::cell::Cell::new(0));
    let rw = CountingWitness(std::cell::Cell::new(0));
    let native = c::close(context(), &nw, r::BoundedUint::new(0).unwrap()).unwrap();
    let recorded = c::recorded::close(context(), &rw, r::BoundedUint::new(0).unwrap()).unwrap();
    assert_eq!(nw.0.get(), 1);
    assert_eq!(rw.0.get(), 1);
    assert_eq!(native.context.private_state, 8);
    assert_eq!(recorded.execution.context.private_state, 8);
    let after = native.context.query.state.get_ref();
    assert!(!slots::active.inspect(after).unwrap());
    assert_eq!(slots::version.inspect(after).unwrap(), 1);
    assert_eq!(slots::updated.inspect(after).unwrap().value(), 42);
    assert_eq!(native.private_transcript_outputs.len(), 1);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);

    // Both stale and inactive conditions fail before now(). The closed/stale
    // combination establishes stale-before-inactive diagnostic precedence.
    for (closed, expected, message) in [
        (false, 1, "stale"),
        (true, 0, "stale"),
        (true, 1, "inactive"),
    ] {
        let prestate = || {
            let ctx = context();
            if closed {
                c::close(ctx, &Witness, r::BoundedUint::new(0).unwrap())
                    .unwrap()
                    .context
            } else {
                ctx
            }
        };
        let nw = CountingWitness(std::cell::Cell::new(0));
        let rw = CountingWitness(std::cell::Cell::new(0));
        let native = c::close(prestate(), &nw, r::BoundedUint::new(expected).unwrap())
            .err()
            .unwrap();
        let recorded = c::recorded::close(prestate(), &rw, r::BoundedUint::new(expected).unwrap())
            .err()
            .unwrap();
        assert_eq!(native, r::CompactError::AssertionFailed(message.into()));
        assert_eq!(recorded, native);
        assert_eq!(nw.0.get(), 0);
        assert_eq!(rw.0.get(), 0);
    }
}
