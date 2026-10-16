// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "experimental-recording-plan")]

use std::cell::RefCell;

use compact_rust_recording_effect_plan_prototype_fixture::ledger_contract::{
    self as contract, LedgerView, PublicStateView, Witnesses,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{
    CircuitContext, CircuitResult, ConstructorContext, ConstructorResult, WitnessContext,
};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::RecordedCircuitResult;
use runtime::{CompactError, Field};

struct Mark {
    result: u64,
    observations: RefCell<Vec<(Field, u64, u64)>>,
}

impl Mark {
    fn new(result: u64) -> Self {
        Self {
            result,
            observations: RefCell::new(Vec::new()),
        }
    }
}

impl Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Field) {
        let state = context.ledger.state().unwrap();
        let round = context.ledger.round().unwrap().value() as u64;
        self.observations
            .borrow_mut()
            .push((state, round, *context.private_state));
        (*context.private_state + 1, Field::from(self.result))
    }
}

#[derive(Clone, Copy, Debug)]
enum Method {
    Guarded,
    Ordered,
}

fn context(seeded: bool) -> CircuitContext<u64> {
    let initial = contract::initial_state(ConstructorContext::new(7_u64)).unwrap();
    if !seeded {
        return initial.into_circuit_context(ContractAddress::default());
    }
    let witness = Mark::new(77);
    let seed = contract::guardedChoice(
        initial.into_circuit_context(ContractAddress::default()),
        &witness,
        Field::from(0_u64),
        Field::from(9_u64),
    )
    .unwrap();
    assert!(witness.observations.borrow().is_empty());
    // Start a new call with the seeded ledger, preserving neither earlier gas
    // nor query effects. This matches the independent TS capture context.
    ConstructorResult::new(ConstructorContext::new(7_u64), seed.context.query.state)
        .into_circuit_context(ContractAddress::default())
}

fn native(
    method: Method,
    context: CircuitContext<u64>,
    witness: &Mark,
    selector: u64,
    next: u64,
) -> Result<CircuitResult<u64, Field>, CompactError> {
    match method {
        Method::Guarded => {
            contract::guardedChoice(context, witness, Field::from(selector), Field::from(next))
        }
        Method::Ordered => {
            contract::orderedObservation(context, witness, Field::from(selector), Field::from(next))
        }
    }
}

fn recorded(
    method: Method,
    context: CircuitContext<u64>,
    witness: &Mark,
    selector: u64,
    next: u64,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
    match method {
        Method::Guarded => contract::recorded::guardedChoice(
            context,
            witness,
            Field::from(selector),
            Field::from(next),
        ),
        Method::Ordered => contract::recorded::orderedObservation(
            context,
            witness,
            Field::from(selector),
            Field::from(next),
        ),
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations
        .insert(
            EntryPointBuf(b"guardedChoice".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"orderedObservation".to_vec()),
            ContractOperation::new(None),
        );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

struct Case {
    id: &'static str,
    method: Method,
    selector: u64,
    next: u64,
    seeded: bool,
    result: u64,
    state: u64,
    round: u64,
    observation: Option<(u64, u64)>,
}

#[test]
fn new_compositions_match_explicit_results_native_execution_ts_and_public_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/effect-plan-prototype.json"
    ))
    .unwrap();
    let cases = [
        Case {
            id: "guarded-zero",
            method: Method::Guarded,
            selector: 0,
            next: 9,
            seeded: false,
            result: 0,
            state: 9,
            round: 1,
            observation: None,
        },
        Case {
            id: "guarded-one",
            method: Method::Guarded,
            selector: 1,
            next: 9,
            seeded: false,
            result: 77,
            state: 77,
            round: 1,
            observation: Some((0, 1)),
        },
        Case {
            id: "guarded-two",
            method: Method::Guarded,
            selector: 2,
            next: 9,
            seeded: false,
            result: 77,
            state: 9,
            round: 1,
            observation: Some((0, 1)),
        },
        Case {
            id: "ordered-zero",
            method: Method::Ordered,
            selector: 0,
            next: 9,
            seeded: false,
            result: 78,
            state: 1,
            round: 2,
            observation: Some((1, 0)),
        },
        Case {
            id: "ordered-one",
            method: Method::Ordered,
            selector: 1,
            next: 9,
            seeded: false,
            result: 12,
            state: 12,
            round: 3,
            observation: Some((12, 3)),
        },
        Case {
            id: "ordered-two",
            method: Method::Ordered,
            selector: 2,
            next: 0,
            seeded: false,
            result: 3,
            state: 0,
            round: 3,
            observation: None,
        },
        Case {
            id: "guarded-nonzero",
            method: Method::Guarded,
            selector: 1,
            next: 9,
            seeded: true,
            result: 86,
            state: 86,
            round: 2,
            observation: Some((9, 2)),
        },
        Case {
            id: "ordered-nonzero",
            method: Method::Ordered,
            selector: 1,
            next: 9,
            seeded: true,
            result: 13,
            state: 13,
            round: 4,
            observation: Some((13, 4)),
        },
    ];
    for case in cases {
        let expected = &oracle["cases"][case.id];
        assert!(expected.is_object(), "missing TS case {}", case.id);
        let native_witness = Mark::new(77);
        let recorded_witness = Mark::new(77);
        let native = native(
            case.method,
            context(case.seeded),
            &native_witness,
            case.selector,
            case.next,
        )
        .unwrap();
        let recorded = recorded(
            case.method,
            context(case.seeded),
            &recorded_witness,
            case.selector,
            case.next,
        )
        .unwrap();
        let observations: Vec<_> = case
            .observation
            .into_iter()
            .map(|(state, round)| (Field::from(state), round, 7))
            .collect();
        let observed_json: Vec<_> = case
            .observation
            .into_iter()
            .map(|(state, round)| serde_json::json!([state.to_string(), round.to_string(), 7]))
            .collect();
        assert_eq!(
            expected["witnessObservations"],
            serde_json::json!(observed_json)
        );
        assert_eq!(
            *native_witness.observations.borrow(),
            observations,
            "{} native witness prefix",
            case.id
        );
        assert_eq!(
            *recorded_witness.observations.borrow(),
            observations,
            "{} recorded witness prefix",
            case.id
        );
        let outputs: Vec<_> = case
            .observation
            .into_iter()
            .map(|_| runtime::fab::AlignedValue::from(Field::from(77_u64)))
            .collect();
        for execution in [&native, &recorded.execution] {
            assert_eq!(
                execution.result,
                Field::from(case.result),
                "{} result",
                case.id
            );
            assert_eq!(
                PublicStateView::from(execution).state().unwrap(),
                Field::from(case.state),
                "{} state",
                case.id
            );
            assert_eq!(
                PublicStateView::from(execution).round().unwrap().value(),
                case.round as u128,
                "{} round",
                case.id
            );
            assert_eq!(
                execution.context.private_state,
                7 + observations.len() as u64,
                "{} private state",
                case.id
            );
            assert_eq!(
                execution.private_transcript_outputs, outputs,
                "{} private transcript",
                case.id
            );
            assert_eq!(
                state_hex(execution.context.query.state.get_ref().clone()),
                expected["afterStateHex"],
                "{} TS complete ledger",
                case.id
            );
            assert_eq!(
                serde_json::to_value(&execution.context.query.effects).unwrap(),
                expected["effects"],
                "{} TS effects",
                case.id
            );
            assert_eq!(
                serde_json::to_value(&execution.private_transcript_outputs).unwrap(),
                expected["privateTranscriptOutputs"],
                "{} TS private transcript",
                case.id
            );
        }
        assert_eq!(expected["result"], case.result.to_string());
        assert_eq!(expected["privateState"], 7 + observations.len() as u64);
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            native.gas_cost, recorded.execution.gas_cost,
            "{} complete execution gas",
            case.id
        );
        assert_eq!(
            serde_json::to_value(runtime::fab::AlignedValue::from(recorded.execution.result))
                .unwrap(),
            expected["output"]
        );
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            expected["publicTranscript"],
            "{} ordered public transcript",
            case.id
        );
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(
            replay.context.state.get_ref(),
            native.context.query.state.get_ref(),
            "{} replay state",
            case.id
        );
        assert_eq!(
            replay.context.effects, native.context.query.effects,
            "{} replay effects",
            case.id
        );
        let actual_gas = serde_json::to_value(native.gas_cost).unwrap();
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(
                actual_gas[dimension], expected_gas,
                "{} TS execution {dimension}",
                case.id
            );
            assert_eq!(
                replay_gas[dimension].as_u64().unwrap().to_string(),
                expected["replayGas"][dimension],
                "{} TS replay {dimension}",
                case.id
            );
        }
    }
}

#[test]
fn failed_assertions_preserve_first_failure_and_witness_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/effect-plan-prototype.json"
    ))
    .unwrap();
    for (id, method, selector, next, mark, message, observation) in [
        // Both assertions would fail: the early guard must win without a witness.
        (
            "early-guard",
            Method::Guarded,
            1,
            8,
            76,
            "next must be nine",
            None,
        ),
        (
            "guarded-mark",
            Method::Guarded,
            1,
            9,
            76,
            "unexpected mark",
            Some((0_u64, 1_u64)),
        ),
        (
            "ordered-early-mark",
            Method::Ordered,
            0,
            9,
            76,
            "unexpected mark",
            Some((1, 0)),
        ),
        (
            "ordered-late-mark",
            Method::Ordered,
            1,
            9,
            76,
            "unexpected mark",
            Some((12, 3)),
        ),
        ("late-guard", Method::Ordered, 2, 9, 76, "late guard", None),
    ] {
        let native_witness = Mark::new(mark);
        let recorded_witness = Mark::new(mark);
        let native_error = native(method, context(false), &native_witness, selector, next)
            .err()
            .expect("native assertion must fail");
        let recorded_error = recorded(method, context(false), &recorded_witness, selector, next)
            .err()
            .expect("recorded assertion must fail without returning a trace");
        assert_eq!(
            native_error,
            CompactError::AssertionFailed(message.to_owned())
        );
        assert_eq!(recorded_error, native_error);
        assert_eq!(native_error.to_string(), oracle["failures"][id]["error"]);
        let observed_json: Vec<_> = observation
            .into_iter()
            .map(|(state, round)| serde_json::json!([state.to_string(), round.to_string(), 7]))
            .collect();
        assert_eq!(
            oracle["failures"][id]["witnessObservations"],
            serde_json::json!(observed_json)
        );
        let observations: Vec<_> = observation
            .into_iter()
            .map(|(state, round)| (Field::from(state), round, 7))
            .collect();
        assert_eq!(*native_witness.observations.borrow(), observations);
        assert_eq!(*recorded_witness.observations.borrow(), observations);
    }
}

#[test]
fn branches_without_a_witness_ignore_its_bad_return_value() {
    for (method, selector, next, result) in [
        (Method::Guarded, 0, 9, 0_u64),
        (Method::Ordered, 2, 0, 3_u64),
    ] {
        let witness = Mark::new(76);
        assert_eq!(
            native(method, context(false), &witness, selector, next)
                .unwrap()
                .result,
            Field::from(result)
        );
        assert_eq!(
            recorded(method, context(false), &witness, selector, next)
                .unwrap()
                .execution
                .result,
            Field::from(result)
        );
        assert!(witness.observations.borrow().is_empty());
    }
}
