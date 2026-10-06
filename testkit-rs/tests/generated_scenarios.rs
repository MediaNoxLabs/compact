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

mod support;
use compact_rust_counter_parameter_fixture::ledger_contract as counter;
use compact_rust_set_size_oracle_fixture::{ledger_contract as sets, ledger_slots};
use compact_rust_witness_conditional_fixture::ledger_contract as branches;
use compact_rust_witnesses_oracle_fixture::ledger_contract as cell;
use midnight_compact_testkit::runtime::{
    BoundedUint, CompactError, Field,
    context::{ConstructorContext, RunningCost, WitnessContext},
    ledger::{query_cell_at_path, read_root_cell},
    recording::RecordingFrame,
};
use midnight_compact_testkit::{ContractLab, LabError, WitnessScript};
use serde_json::{Value, json};

#[test]
fn counter_sequences_fork_restore_and_late_failure_are_atomic() {
    let mut lab = support::counter();
    let start = lab.snapshot();
    let first = lab
        .recorded(|c| counter::recorded::increment_by(c, BoundedUint::new(3)?))
        .unwrap();
    assert_eq!(first.before(), start.public_state());
    let evidence = first.replay().unwrap();
    let replay = midnight_compact_testkit::runtime::ledger::QueryContext::new(
        start.public_state().clone(),
        lab.environment().address,
    )
    .query(evidence.program(), None, &lab.environment().cost_model)
    .unwrap();
    assert_eq!(
        replay.context.state.get_ref(),
        first.public_state().get_ref()
    );
    assert_eq!(replay.context.effects, *first.effects());
    assert_eq!(replay.gas_cost, evidence.gas());
    let mut fork = lab.fork();
    let second = lab
        .native(|c| counter::increment_by(c, BoundedUint::new(4)?))
        .unwrap();
    assert_ne!(first.public_state(), second.public_state());
    assert!(second.replay().is_none());
    let before = lab.snapshot();
    let error = lab
        .recorded(|mut c| {
            c.private_state.push("uncommitted secret".into());
            let r = counter::recorded::increment_by(c, BoundedUint::new(2)?)?;
            counter::recorded::decrement_by(r.execution.context, BoundedUint::new(20)?)
        })
        .unwrap_err();
    assert!(matches!(error, LabError::Execution(_)));
    assert_eq!(lab.snapshot().public_state(), before.public_state());
    assert!(lab.private_state().is_empty());
    let reset = fork.recorded(counter::recorded::reset_round).unwrap();
    assert_eq!(
        reset.public_state().get_ref(),
        start.public_state().get_ref()
    );
    assert_ne!(
        lab.snapshot().public_state(),
        fork.snapshot().public_state()
    );
    lab.restore(&start).unwrap();
    assert_eq!(lab.snapshot().public_state(), start.public_state());
}

#[derive(Clone)]
struct Private {
    script: WitnessScript<(), Field>,
    calls: u64,
}
struct CellWitness;
impl cell::TryWitnesses<Private> for CellWitness {
    fn fetch_field(
        &self,
        context: WitnessContext<'_, Private, cell::LedgerView<'_>>,
    ) -> Result<(Private, Field), CompactError> {
        let _observed = context.ledger.v()?; // Real metered witness ledger read.
        let mut private = context.private_state.clone();
        let value = private.script.answer(())?;
        private.calls += 1;
        Ok((private, value))
    }
}
fn private_outputs(outputs: &[midnight_compact_testkit::runtime::fab::AlignedValue]) -> Value {
    json!(
        outputs
            .iter()
            .map(|v| {
                let atoms: Vec<_> = v.value.0.iter().map(|a| &a.0).collect();
                json!({"valueAtoms":atoms,"alignment":v.alignment})
            })
            .collect::<Vec<_>>()
    )
}
#[test]
fn witnessed_cell_matches_frozen_typescript_and_commits_owned_script() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../runtime-rs/tests/fixtures/oracle-recorded-traces.json"
    ))
    .unwrap();
    for value in [42_u64, 0] {
        let row = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == format!("witnesses_oracle/pull/witness-{value}"))
            .unwrap();
        let initial = cell::initial_state(ConstructorContext::new(Private {
            script: WitnessScript::new([((), Ok(Field::from(value)))]),
            calls: 7,
        }))
        .unwrap();
        let mut lab = ContractLab::from_constructor(
            support::identity_for(
                include_bytes!("../../examples/rust_backend/witnesses_oracle.compact"),
                include_bytes!("../../tests-rust-backend/witnesses-oracle/lib.rs"),
            ),
            support::environment(),
            initial,
        )
        .unwrap();
        // Independent component queries for this one-read/one-write fixture.
        // Equality of its gas delta to this read cost is not a general law.
        let component_context = cell::initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(lab.environment().address);
        let (read, _) = query_cell_at_path::<Field, _>(
            &component_context.query,
            &[0],
            None,
            &lab.environment().cost_model,
        )
        .unwrap();
        let write = component_context
            .write_cell(0u8, Field::from(value))
            .unwrap();
        let mut native = lab.fork();
        let n = native.native(|c| cell::pull(c, &CellWitness)).unwrap();
        let r = lab
            .recorded(|c| cell::recorded::pull(c, &CellWitness))
            .unwrap();
        assert_eq!(r.public_state(), n.public_state());
        assert_eq!(r.effects(), n.effects());
        assert_eq!(r.execution_gas(), n.execution_gas());
        assert_eq!(r.execution_gas(), read.gas_cost + write.gas_cost);
        let evidence = r.replay().unwrap();
        let full = midnight_compact_testkit::runtime::ledger::QueryContext::new(
            r.before().clone(),
            lab.environment().address,
        )
        .query(evidence.program(), None, &lab.environment().cost_model)
        .unwrap();
        assert_eq!(evidence.gas(), full.gas_cost);
        assert_eq!(full.context.state.get_ref(), r.public_state().get_ref());
        assert_eq!(&full.context.effects, r.effects());
        assert_eq!(evidence.gas(), write.gas_cost);
        assert_eq!(r.execution_gas(), evidence.gas() + read.gas_cost);
        assert_eq!(
            json!(r.replay().unwrap().program()),
            row["publicTranscript"]
        );
        assert_eq!(
            private_outputs(r.private_outputs()),
            row["privateTranscript"]
        );
        assert_eq!(
            read_root_cell::<Field, _>(r.public_state().get_ref(), 0).unwrap(),
            Field::from(value)
        );
        assert_eq!(lab.private_state().calls, 8);
        assert_eq!(lab.private_state().script.remaining(), 0);
        assert_eq!(lab.private_state().script.journal(), &[()]);
        let gas = json!(r.execution_gas());
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q["gasCost"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dim], sum);
        }
        let before = lab.snapshot();
        assert!(
            lab.recorded(|c| cell::recorded::pull(c, &CellWitness))
                .is_err()
        );
        assert_eq!(lab.snapshot().public_state(), before.public_state());
        assert_eq!(lab.private_state().calls, 8);
    }
}
#[test]
fn collections_use_real_vm_on_empty_and_nonempty_prestates() {
    for populated in [false, true] {
        let initial = sets::initial_state(ConstructorContext::new(())).unwrap();
        let mut lab = ContractLab::from_constructor(
            support::identity_for(
                include_bytes!("../../examples/rust_backend/set_size_oracle.compact"),
                include_bytes!("../../tests-rust-backend/set-size-oracle/lib.rs"),
            ),
            support::environment(),
            initial,
        )
        .unwrap();
        if populated {
            lab.native(|c| ledger_slots::s.insert(c, Field::from(42_u64)))
                .unwrap();
            lab.native(|c| ledger_slots::m.insert(c, Field::from(7_u64), Field::from(9_u64)))
                .unwrap();
        }
        let mut native = lab.fork();
        let a = lab.recorded(sets::recorded::check_set_empty).unwrap();
        let b = native.native(sets::check_set_empty).unwrap();
        assert_eq!(a.public_state(), b.public_state());
        assert_eq!(
            read_root_cell::<bool, _>(a.public_state().get_ref(), 0).unwrap(),
            !populated
        );
        let a = lab.recorded(sets::recorded::check_map_empty).unwrap();
        let b = native.native(sets::check_map_empty).unwrap();
        assert_eq!(a.public_state(), b.public_state());
        assert_eq!(
            read_root_cell::<bool, _>(a.public_state().get_ref(), 1).unwrap(),
            !populated
        );
        assert!(!a.replay().unwrap().program().is_empty());
    }
}

struct BranchWitness;
impl branches::TryWitnesses<WitnessScript<Field, Field>> for BranchWitness {
    fn secret(
        &self,
        c: WitnessContext<'_, WitnessScript<Field, Field>, branches::LedgerView<'_>>,
        seed: Field,
    ) -> Result<(WitnessScript<Field, Field>, Field), CompactError> {
        let mut private = c.private_state.clone();
        let result = private.answer(seed)?;
        Ok((private, result))
    }
}
#[test]
fn native_only_branches_and_ordered_witness_outputs_are_not_labeled_replay() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../runtime-rs/tests/fixtures/witness-conditional-ts-output.json"
    ))
    .unwrap();
    for (selected, seed, answer, key) in [(true, 2, 9, "selected"), (false, 20, 27, "unselected")] {
        let initial = branches::initial_state(ConstructorContext::new(WitnessScript::new([(
            Field::from(seed as u64),
            Ok(Field::from(answer as u64)),
        )])))
        .unwrap();
        let mut lab = ContractLab::from_constructor(
            support::identity_for(
                include_bytes!("../../examples/rust_backend/witness_conditional.compact"),
                include_bytes!("../../tests-rust-backend/witness-conditional/lib.rs"),
            ),
            support::environment(),
            initial,
        )
        .unwrap();
        let r = lab
            .native(|c| {
                branches::choose_secret(
                    c,
                    &BranchWitness,
                    selected,
                    Field::from(2_u64),
                    Field::from(20_u64),
                )
            })
            .unwrap();
        assert_eq!(*r.output(), Field::from(answer as u64));
        assert_eq!(
            private_outputs(r.private_outputs()),
            oracle[key]["privateTranscriptOutputs"]
        );
        assert!(r.replay().is_none());
        assert_eq!(lab.private_state().journal(), &[Field::from(seed as u64)]);
    }
    let initial = branches::initial_state(ConstructorContext::new(WitnessScript::new([
        (Field::from(2_u64), Ok(Field::from(9_u64))),
        (Field::from(2_u64), Ok(Field::from(10_u64))),
    ])))
    .unwrap();
    let mut lab = ContractLab::from_constructor(
        support::identity_for(
            include_bytes!("../../examples/rust_backend/witness_conditional.compact"),
            include_bytes!("../../tests-rust-backend/witness-conditional/lib.rs"),
        ),
        support::environment(),
        initial,
    )
    .unwrap();
    let r = lab
        .native(|c| branches::pair_secret(c, &BranchWitness, Field::from(2_u64)))
        .unwrap();
    assert_eq!(
        private_outputs(r.private_outputs()),
        oracle["pair"]["privateTranscriptOutputs"]
    );
    assert_eq!(
        lab.private_state().journal(),
        &[Field::from(2_u64), Field::from(2_u64)]
    );
}
#[test]
fn owned_forks_can_run_independently_on_normal_test_workers() {
    let lab = support::counter();
    let threads: Vec<_> = [2, 8]
        .into_iter()
        .map(|amount| {
            let mut fork = lab.fork();
            std::thread::spawn(move || {
                fork.recorded(|c| counter::recorded::increment_by(c, BoundedUint::new(amount)?))
                    .unwrap();
                fork.snapshot()
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_ne!(results[0].public_state(), results[1].public_state());
    assert_ne!(lab.snapshot().public_state(), results[0].public_state());
}

#[test]
fn query_grouping_changes_replay_cost_without_witnesses() {
    // Runtime-frame control, not a new generated contract. Use the existing
    // generated constructor, then compare separate queries with one full query.
    for count in [1u64, 2] {
        let initial = cell::initial_state(ConstructorContext::new(())).unwrap();
        let mut lab = ContractLab::from_constructor(
            support::identity_for(
                include_bytes!("../../examples/rust_backend/witnesses_oracle.compact"),
                include_bytes!("../../tests-rust-backend/witnesses-oracle/lib.rs"),
            ),
            support::environment(),
            initial,
        )
        .unwrap();
        let mut context = cell::initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(lab.environment().address);
        let mut components = RunningCost::ZERO;
        for value in 42..42 + count {
            let result = context.write_cell(0u8, Field::from(value)).unwrap();
            components += result.gas_cost;
            context = result.context;
        }
        let report = lab
            .recorded(|c| {
                let mut frame = RecordingFrame::new(c);
                for value in 42..42 + count {
                    frame = frame.write_cell(&[0u8], Field::from(value))?;
                }
                Ok(frame.finish(()))
            })
            .unwrap();
        assert!(report.private_outputs().is_empty());
        assert_eq!(report.execution_gas(), components);
        assert_eq!(
            report.public_state().get_ref(),
            context.query.state.get_ref()
        );
        assert_eq!(report.effects(), &context.query.effects);
        let evidence = report.replay().unwrap();
        let replay = midnight_compact_testkit::runtime::ledger::QueryContext::new(
            report.before().clone(),
            lab.environment().address,
        )
        .query(evidence.program(), None, &lab.environment().cost_model)
        .unwrap();
        assert_eq!(evidence.gas(), replay.gas_cost);
        assert_eq!(
            replay.context.state.get_ref(),
            report.public_state().get_ref()
        );
        assert_eq!(&replay.context.effects, report.effects());
        if count == 1 {
            assert_eq!(components, replay.gas_cost);
        } else {
            // Fixture-only arithmetic, deliberately no universal ordering claim.
            let execution = json!(components);
            let replay = json!(replay.gas_cost);
            assert_eq!(execution["readTime"], replay["readTime"]);
            assert_ne!(execution["computeTime"], replay["computeTime"]);
            assert_eq!(execution["bytesWritten"], 76);
            assert_eq!(execution["bytesDeleted"], 74);
            assert_eq!(replay["bytesWritten"], 38);
            assert_eq!(replay["bytesDeleted"], 36);
        }
    }
}
