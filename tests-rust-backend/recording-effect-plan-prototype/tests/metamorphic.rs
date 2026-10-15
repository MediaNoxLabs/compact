// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Execution controls for synthetic backend IR; these variants are not new
//! Compact source coverage or additional proved circuits.
#![cfg(feature = "experimental-recording-plan")]

use std::cell::RefCell;

use compact_rust_recording_effect_plan_prototype_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use runtime::context::{
    CircuitContext, CircuitResult, ConstructorContext, ConstructorResult, WitnessContext,
};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::RecordedCircuitResult;
use runtime::{CompactError, Field};

type WitnessObservations = Vec<(Field, u64, u64)>;

struct Mark {
    value: u64,
    observations: RefCell<WitnessObservations>,
}

impl Mark {
    fn new(value: u64) -> Self {
        Self {
            value,
            observations: RefCell::new(Vec::new()),
        }
    }
}

impl contract::Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, contract::LedgerView<'_>>) -> (u64, Field) {
        let state = context.ledger.state().unwrap();
        let round = context.ledger.round().unwrap().value() as u64;
        self.observations
            .borrow_mut()
            .push((state, round, *context.private_state));
        (*context.private_state + 1, Field::from(self.value))
    }
}

fn context(seeded: bool) -> CircuitContext<u64> {
    let initial = contract::initial_state(ConstructorContext::new(7_u64)).unwrap();
    if !seeded {
        return initial.into_circuit_context(ContractAddress::default());
    }
    let seed = contract::guardedChoice(
        initial.into_circuit_context(ContractAddress::default()),
        &Mark::new(77),
        Field::from(0_u64),
        Field::from(9_u64),
    )
    .unwrap();
    ConstructorResult::new(ConstructorContext::new(7_u64), seed.context.query.state)
        .into_circuit_context(ContractAddress::default())
}

type Native =
    fn(CircuitContext<u64>, &Mark, Field, Field) -> Result<CircuitResult<u64, Field>, CompactError>;
type Recorded = fn(
    CircuitContext<u64>,
    &Mark,
    Field,
    Field,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError>;

#[derive(Clone, Copy, Debug)]
struct Input {
    selector: u64,
    next: u64,
    mark: u64,
    seeded: bool,
}

#[derive(Debug, PartialEq)]
struct Execution {
    result: Field,
    state: StateValue<DefaultDB>,
    effects: serde_json::Value,
    private: u64,
    outputs: Vec<runtime::fab::AlignedValue>,
    gas: [u64; 4],
}

#[derive(Debug, PartialEq)]
struct Replay {
    program: serde_json::Value,
    state: StateValue<DefaultDB>,
    effects: serde_json::Value,
    gas: [u64; 4],
}

#[derive(Debug, PartialEq)]
struct Captured {
    execution: Result<Execution, CompactError>,
    witnesses: WitnessObservations,
    replay: Option<Replay>,
}

fn gas(cost: runtime::context::RunningCost) -> [u64; 4] {
    let cost = serde_json::to_value(cost).unwrap();
    ["readTime", "computeTime", "bytesWritten", "bytesDeleted"]
        .map(|dimension| cost[dimension].as_u64().unwrap())
}

fn execution(result: CircuitResult<u64, Field>) -> Execution {
    Execution {
        result: result.result,
        state: result.context.query.state.get_ref().clone(),
        effects: serde_json::to_value(result.context.query.effects).unwrap(),
        private: result.context.private_state,
        outputs: result.private_transcript_outputs,
        gas: gas(result.gas_cost),
    }
}

fn native(function: Native, input: Input) -> Captured {
    let witness = Mark::new(input.mark);
    let result = function(
        context(input.seeded),
        &witness,
        Field::from(input.selector),
        Field::from(input.next),
    );
    Captured {
        execution: result.map(execution),
        witnesses: witness.observations.into_inner(),
        replay: None,
    }
}

fn recorded(function: Recorded, input: Input) -> Captured {
    let witness = Mark::new(input.mark);
    let result = function(
        context(input.seeded),
        &witness,
        Field::from(input.selector),
        Field::from(input.next),
    );
    let mut replay = None;
    let result = result.map(|result| {
        let queried = result
            .public
            .initial()
            .query(
                result.public.verify_ops(),
                None,
                &result.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(
            queried.context.state.get_ref(),
            result.execution.context.query.state.get_ref()
        );
        assert_eq!(
            queried.context.effects,
            result.execution.context.query.effects
        );
        replay = Some(Replay {
            program: serde_json::to_value(result.public.verify_ops()).unwrap(),
            state: queried.context.state.get_ref().clone(),
            effects: serde_json::to_value(queried.context.effects).unwrap(),
            gas: gas(queried.gas_cost),
        });
        execution(result.execution)
    });
    Captured {
        execution: result,
        witnesses: witness.observations.into_inner(),
        replay,
    }
}

struct Variant {
    name: &'static str,
    native: Native,
    recorded: Recorded,
}
struct Family {
    native: Native,
    recorded: Recorded,
    variants: [Variant; 4],
    inputs: [(u64, u64, u64); 6],
}

#[test]
fn deterministic_rewrites_preserve_execution_refusal_and_replay() {
    let families = [
        Family {
            native: contract::guardedChoice,
            recorded: contract::recorded::guardedChoice,
            variants: [
                Variant {
                    name: "guarded-alpha",
                    native: contract::guardedChoice_alpha,
                    recorded: contract::recorded::guardedChoice_alpha,
                },
                Variant {
                    name: "guarded-sequences",
                    native: contract::guardedChoice_sequences,
                    recorded: contract::recorded::guardedChoice_sequences,
                },
                Variant {
                    name: "guarded-scopes",
                    native: contract::guardedChoice_scopes,
                    recorded: contract::recorded::guardedChoice_scopes,
                },
                Variant {
                    name: "guarded-branches",
                    native: contract::guardedChoice_branches,
                    recorded: contract::recorded::guardedChoice_branches,
                },
            ],
            // Includes a skipped bad witness, both witness branches, an early
            // guard winning over a bad witness, and a selected bad witness.
            inputs: [
                (0, 9, 76),
                (1, 9, 77),
                (2, 9, 77),
                (1, 8, 76),
                (1, 9, 76),
                (2, 0, 77),
            ],
        },
        Family {
            native: contract::orderedObservation,
            recorded: contract::recorded::orderedObservation,
            variants: [
                Variant {
                    name: "ordered-alpha",
                    native: contract::orderedObservation_alpha,
                    recorded: contract::recorded::orderedObservation_alpha,
                },
                Variant {
                    name: "ordered-sequences",
                    native: contract::orderedObservation_sequences,
                    recorded: contract::recorded::orderedObservation_sequences,
                },
                Variant {
                    name: "ordered-scopes",
                    native: contract::orderedObservation_scopes,
                    recorded: contract::recorded::orderedObservation_scopes,
                },
                Variant {
                    name: "ordered-branches",
                    native: contract::orderedObservation_branches,
                    recorded: contract::recorded::orderedObservation_branches,
                },
            ],
            // Both witness observation prefixes, a skipped bad witness, both
            // witness failures, and the post-write/post-increment assertion.
            inputs: [
                (0, 9, 77),
                (1, 9, 77),
                (2, 0, 76),
                (0, 9, 76),
                (1, 9, 76),
                (2, 9, 76),
            ],
        },
    ];
    for family in families {
        for seeded in [false, true] {
            for (selector, next, mark) in family.inputs {
                let input = Input {
                    selector,
                    next,
                    mark,
                    seeded,
                };
                let original_native = native(family.native, input);
                let original_recorded = recorded(family.recorded, input);
                assert_eq!(
                    original_native.execution, original_recorded.execution,
                    "base {input:?}"
                );
                assert_eq!(
                    original_native.witnesses, original_recorded.witnesses,
                    "base {input:?}"
                );
                for variant in &family.variants {
                    assert_eq!(
                        native(variant.native, input),
                        original_native,
                        "{} native {input:?}",
                        variant.name
                    );
                    assert_eq!(
                        recorded(variant.recorded, input),
                        original_recorded,
                        "{} recorded {input:?}",
                        variant.name
                    );
                }
            }
        }
    }
}

#[test]
fn admitted_api_can_select_a_successful_branch_without_a_public_trace() {
    let input = Input {
        selector: 99,
        next: 8,
        mark: 76,
        seeded: false,
    };
    let native = native(contract::empty_public_branch, input);
    let recorded = recorded(contract::recorded::empty_public_branch, input);
    assert_eq!(native.execution, recorded.execution);
    assert!(native.witnesses.is_empty());
    assert!(recorded.witnesses.is_empty());
    let result = recorded.execution.unwrap();
    assert_eq!(result.result, Field::from(0_u64));
    assert_eq!(result.state, context(false).query.state.get_ref().clone());
    assert_eq!(result.private, 7);
    assert_eq!(result.gas, [0; 4]);
    assert!(result.outputs.is_empty());
    assert_eq!(recorded.replay.unwrap().program, serde_json::json!([]));
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn empty_public_branch_preparation_retains_exact_empty_transcript_refusal() {
    use midnight_serialize::{Deserializable, Serializable};
    use runtime::transaction::{CallSpec, PrepareCallError, VerifierKey, prepare_call};

    // Preparation refuses the empty trace before examining the verifier key.
    // This serializable carrier contains no proof key and makes no proof claim.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let carrier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let recorded = contract::recorded::empty_public_branch(
        context(false),
        &Mark::new(76),
        Field::from(99_u64),
        Field::from(8_u64),
    )
    .unwrap();
    let spec = CallSpec::new(
        "empty_public_branch",
        carrier,
        (Field::from(99_u64), Field::from(8_u64)),
        Field::from(0_u64),
    );
    assert!(matches!(
        prepare_call(recorded, spec),
        Err(PrepareCallError::EmptyTranscript)
    ));
}
