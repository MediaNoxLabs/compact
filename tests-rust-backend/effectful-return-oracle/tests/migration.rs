// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// A separate integration-test crate root keeps the frozen generated code's
// `crate::ledger_slots` paths intact, independently of the fresh fixture crate.
include!("../legacy_reference.rs");

use compact_rust_effectful_return_oracle_fixture::ledger_contract as fresh;
use ledger_contract as legacy;
use runtime::context::{CircuitContext, CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::ContractAddress;
use runtime::recording::RecordedCircuitResult;
use runtime::{CompactError, Field};
use std::cell::RefCell;

#[derive(Debug)]
struct Witness {
    answer: Field,
    fail: bool,
    observations: RefCell<Vec<(Field, Field, u64)>>,
}

impl Witness {
    fn new(answer: Field, fail: bool) -> Self {
        Self {
            answer,
            fail,
            observations: RefCell::new(Vec::new()),
        }
    }

    fn observe(
        &self,
        state: Field,
        unused: Field,
        private: u64,
    ) -> Result<(u64, Field), CompactError> {
        self.observations
            .borrow_mut()
            .push((state, unused, private));
        if self.fail {
            Err(CompactError::AssertionFailed(
                "migration witness refusal".into(),
            ))
        } else {
            Ok((private + 1, self.answer))
        }
    }
}

macro_rules! witness_for {
    ($contract:ident) => {
        impl $contract::TryWitnesses<u64> for Witness {
            fn mark(
                &self,
                context: WitnessContext<'_, u64, $contract::LedgerView<'_>>,
            ) -> Result<(u64, Field), CompactError> {
                // The metered reads make witness ordering and execution gas
                // observable; these reads must not become public replay steps.
                self.observe(
                    context.ledger.state()?,
                    context.ledger.unused()?,
                    *context.private_state,
                )
            }
        }
    };
}
witness_for!(legacy);
witness_for!(fresh);

fn context(seed: Field, private: u64) -> CircuitContext<u64> {
    let initial = fresh::initial_state(ConstructorContext::new(private)).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let context = ledger_slots::state.write(context, seed).unwrap().context;
    ledger_slots::unused
        .write(context, Field::from(23_u64))
        .unwrap()
        .context
}

fn compare_execution(left: &CircuitResult<u64, Field>, right: &CircuitResult<u64, Field>) {
    assert_eq!(left.result, right.result);
    assert_eq!(left.context.query.state, right.context.query.state);
    assert_eq!(left.context.query.effects, right.context.query.effects);
    assert_eq!(left.context.private_state, right.context.private_state);
    assert_eq!(
        left.private_transcript_outputs,
        right.private_transcript_outputs
    );
    assert_eq!(left.gas_cost, right.gas_cost);
}

fn compare_recording(
    left: &RecordedCircuitResult<u64, Field>,
    right: &RecordedCircuitResult<u64, Field>,
) {
    compare_execution(&left.execution, &right.execution);
    assert_eq!(
        serde_json::to_value(left.public.verify_ops()).unwrap(),
        serde_json::to_value(right.public.verify_ops()).unwrap()
    );
    let mut replay_cost = None;
    for result in [left, right] {
        let replay = result
            .public
            .initial()
            .query(
                result.public.verify_ops(),
                None,
                &result.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.context.state, result.execution.context.query.state);
        assert_eq!(
            replay.context.effects,
            result.execution.context.query.effects
        );
        if let Some(cost) = replay_cost {
            assert_eq!(replay.gas_cost, cost);
        }
        replay_cost = Some(replay.gas_cost);
    }
}

#[test]
fn fresh_default_recording_preserves_legacy_behavior_over_field_and_witness_boundaries() {
    let seeds = [
        Field::from(0_u64),
        Field::from(1_u64),
        Field::from(9_u64),
        Field::from(u64::MAX),
        -Field::from(1_u64),
    ];
    let mut cases = 0;
    for seed in seeds {
        for next in [0_u64, 1, 9, u64::MAX] {
            for private in [0_u64, 7] {
                for answer in [0_u64, 77, u64::MAX] {
                    let witnesses = std::array::from_fn::<_, 4, _>(|_| {
                        Witness::new(Field::from(answer), false)
                    });
                    let legacy_native =
                        legacy::choose(context(seed, private), &witnesses[0], Field::from(next))
                            .unwrap();
                    let fresh_native =
                        fresh::choose(context(seed, private), &witnesses[1], Field::from(next))
                            .unwrap();
                    let legacy_recorded = legacy::recorded::choose(
                        context(seed, private),
                        &witnesses[2],
                        Field::from(next),
                    )
                    .unwrap();
                    let fresh_recorded = fresh::recorded::choose(
                        context(seed, private),
                        &witnesses[3],
                        Field::from(next),
                    )
                    .unwrap();
                    compare_execution(&legacy_native, &fresh_native);
                    compare_execution(&legacy_native, &legacy_recorded.execution);
                    compare_execution(&legacy_native, &fresh_recorded.execution);
                    compare_recording(&legacy_recorded, &fresh_recorded);
                    let takes_witness = seed != Field::from(0_u64);
                    let expected_result = if takes_witness {
                        seed + Field::from(100_u64)
                    } else {
                        seed
                    };
                    assert_eq!(fresh_recorded.execution.result, expected_result);
                    assert_eq!(
                        fresh_recorded.execution.context.private_state,
                        private + u64::from(takes_witness)
                    );
                    let expected_observations = if takes_witness {
                        vec![(seed + Field::from(1_u64), Field::from(23_u64), private)]
                    } else {
                        vec![]
                    };
                    for witness in &witnesses {
                        assert_eq!(*witness.observations.borrow(), expected_observations);
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 120);
}

#[test]
fn witness_failure_and_skipped_callback_preserve_the_legacy_execution_prefix() {
    for seed in [0_u64, 1, 9, u64::MAX] {
        let witnesses = std::array::from_fn::<_, 4, _>(|_| Witness::new(Field::from(77_u64), true));
        let outcomes = [
            legacy::choose(
                context(Field::from(seed), 7),
                &witnesses[0],
                Field::from(9_u64),
            ),
            fresh::choose(
                context(Field::from(seed), 7),
                &witnesses[1],
                Field::from(9_u64),
            ),
            legacy::recorded::choose(
                context(Field::from(seed), 7),
                &witnesses[2],
                Field::from(9_u64),
            )
            .map(|result| result.execution),
            fresh::recorded::choose(
                context(Field::from(seed), 7),
                &witnesses[3],
                Field::from(9_u64),
            )
            .map(|result| result.execution),
        ];
        for outcome in outcomes {
            if seed == 0 {
                assert_eq!(outcome.unwrap().result, Field::from(0_u64));
            } else {
                assert_eq!(
                    outcome.err().expect("selected witness must fail"),
                    CompactError::AssertionFailed("migration witness refusal".into())
                );
            }
        }
        let expected = if seed == 0 {
            vec![]
        } else {
            vec![(
                Field::from(seed) + Field::from(1_u64),
                Field::from(23_u64),
                7,
            )]
        };
        for witness in &witnesses {
            assert_eq!(*witness.observations.borrow(), expected);
        }
    }
}
