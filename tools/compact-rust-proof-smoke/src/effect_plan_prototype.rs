// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Opt-in ADR0355 proof acceptance under the shared unbalanced smoke policy.
//! The retained compiler IR is also the integration crate's generation input.

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

use compact_rust_recording_effect_plan_prototype_fixture::ledger_contract as contract;
use midnight_compact_runtime::ledger::ContractAddress;

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/effect-plan-prototype-schema20-ir.json");

#[derive(Clone, Default)]
struct Mark(Rc<RefCell<Vec<(Field, u64, u64)>>>);

impl contract::Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, contract::LedgerView<'_>>) -> (u64, Field) {
        // Order matches the independently captured TypeScript oracle and meters
        // these reads as witness observations, outside the public replay program.
        let state = context.ledger.state().expect("prototype witness state");
        let round = context
            .ledger
            .round()
            .expect("prototype witness round")
            .value() as u64;
        self.0
            .borrow_mut()
            .push((state, round, *context.private_state));
        (*context.private_state + 1, Field::from(77_u64))
    }
}

#[derive(Clone, Copy)]
enum Method {
    Guarded,
    Ordered,
}

impl Method {
    fn circuit(self) -> &'static str {
        match self {
            Self::Guarded => "guardedChoice",
            Self::Ordered => "orderedObservation",
        }
    }
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
    witness_prefix: Option<(u64, u64)>,
}

fn validate_ir(artifact: &[u8]) -> Result<(), Box<dyn Error>> {
    let expected: serde_json::Value = serde_json::from_str(RETAINED_IR)?;
    let actual: serde_json::Value = serde_json::from_slice(artifact)?;
    if actual != expected {
        return Err("effect plan proof artifacts do not match the retained compiler IR".into());
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Check before loading keys or constructing transactions. Matching JSON
    // accepts formatting differences but rejects any changed circuit or schema.
    validate_ir(&fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/effect-plan-prototype.json"
    ))?;
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
            witness_prefix: None,
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
            witness_prefix: Some((0, 1)),
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
            witness_prefix: Some((0, 1)),
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
            witness_prefix: Some((1, 0)),
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
            witness_prefix: Some((12, 3)),
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
            witness_prefix: None,
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
            witness_prefix: Some((9, 2)),
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
            witness_prefix: Some((13, 4)),
        },
    ];
    println!("effect plan prototype: 8 cases; shared unbalanced smoke policy (balancing disabled)");
    for (index, case) in cases.into_iter().enumerate() {
        let circuit = case.method.circuit();
        let mut rng = StdRng::seed_from_u64(0x0355_0000 + index as u64);
        let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
        let state = if case.seeded {
            contract::guardedChoice(
                initial.into_circuit_context(ContractAddress::default()),
                &Mark::default(),
                Field::from(0_u64),
                Field::from(9_u64),
            )?
            .context
            .query
            .state
            .get_ref()
            .clone()
        } else {
            initial.ledger_state.get_ref().clone()
        };
        // Seeded cases deploy an explicit nonzero starting ledger. Each tested
        // operation then starts a fresh call context, including fresh gas.
        let deploy = make_deploy(root, circuit, state, &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native_witness = Mark::default();
        let recorded_witness = Mark::default();
        let selector = Field::from(case.selector);
        let next = Field::from(case.next);
        let native = match case.method {
            Method::Guarded => contract::guardedChoice(
                observed.circuit_context(7_u64),
                &native_witness,
                selector,
                next,
            )?,
            Method::Ordered => contract::orderedObservation(
                observed.circuit_context(7_u64),
                &native_witness,
                selector,
                next,
            )?,
        };
        let recorded = match case.method {
            Method::Guarded => contract::recorded::guardedChoice(
                observed.circuit_context(7_u64),
                &recorded_witness,
                selector,
                next,
            )?,
            Method::Ordered => contract::recorded::orderedObservation(
                observed.circuit_context(7_u64),
                &recorded_witness,
                selector,
                next,
            )?,
        };
        let expected_observations: Vec<_> = case
            .witness_prefix
            .into_iter()
            .map(|(state, round)| (Field::from(state), round, 7_u64))
            .collect();
        let expected_outputs: Vec<_> = case
            .witness_prefix
            .into_iter()
            .map(|_| AlignedValue::from(Field::from(77_u64)))
            .collect();
        for execution in [&native, &recorded.execution] {
            let view = contract::PublicStateView::from(execution);
            if execution.result != Field::from(case.result)
                || view.state()? != Field::from(case.state)
                || view.round()?.value() != u128::from(case.round)
                || execution.context.private_state != 7 + expected_observations.len() as u64
                || execution.private_transcript_outputs != expected_outputs
            {
                return Err(format!(
                    "{} explicit result, ledger or private transcript mismatch",
                    case.id
                )
                .into());
            }
        }
        if native.gas_cost != recorded.execution.gas_cost
            || native.context.query.state != recorded.execution.context.query.state
            || native.context.query.effects != recorded.execution.context.query.effects
            || *native_witness.0.borrow() != expected_observations
            || *recorded_witness.0.borrow() != expected_observations
        {
            return Err(format!(
                "{} native/recorded gas, ledger or witness order mismatch",
                case.id
            )
            .into());
        }
        let reference = &oracle["cases"][case.id];
        if serde_json::to_value(recorded.public.verify_ops())? != reference["publicTranscript"]
            || serde_json::to_value(&recorded.execution.private_transcript_outputs)?
                != reference["privateTranscriptOutputs"]
        {
            return Err(format!(
                "{} transcript differs from independent TypeScript capture",
                case.id
            )
            .into());
        }
        let gas = serde_json::to_value(native.gas_cost)?;
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected = reference["queries"]
                .as_array()
                .ok_or("missing oracle query costs")?
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .ok_or("missing oracle gas dimension")?
                        .parse::<u64>()
                        .map_err(|error| -> Box<dyn Error> { error.into() })
                })
                .collect::<Result<Vec<_>, Box<dyn Error>>>()?
                .into_iter()
                .sum::<u64>();
            if gas[dimension] != expected {
                return Err(format!(
                    "{} {dimension} differs from TypeScript query costs",
                    case.id
                )
                .into());
            }
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[AlignedValue::from(selector), AlignedValue::from(next)]);
        let manual = check_generated_trace(root, circuit, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let typed_witness = Mark::default();
        let typed_contract = contract::Contract::from(typed_witness.clone());
        let typed = match case.method {
            Method::Guarded => typed_contract
                .recording()
                .guardedChoice_call(&observed, 7_u64, selector, next)?,
            Method::Ordered => typed_contract
                .recording()
                .orderedObservation_call(&observed, 7_u64, selector, next)?,
        };
        let prepared = typed.prepare(verifier, Field::from(0_u64))?;
        // As in effectful_return, compare the complete prepared representation
        // and prove the case once. The general parity helper would prove twice.
        if format!("{manual:?}") != format!("{prepared:?}")
            || *typed_witness.0.borrow() != expected_observations
        {
            return Err(format!(
                "{} observed-call preparation differs from direct recording",
                case.id
            )
            .into());
        }
        check_transaction(root, circuit, deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err(format!(
                    "{} proven ledger state differs from native execution",
                    case.id
                )
                .into());
            }
            let view = contract::PublicStateView::from(state.data.get_ref());
            if view.state()? != Field::from(case.state)
                || view.round()?.value() != u128::from(case.round)
            {
                return Err(format!("{} proven Cell/Counter result mismatch", case.id).into());
            }
            Ok(())
        })?;
        println!(
            "effect plan prototype {} proved, verified, and ledger-applied under shared unbalanced smoke policy",
            case.id
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RETAINED_IR, validate_ir};

    #[test]
    fn proof_artifact_guard_accepts_formatting_but_rejects_different_ir() {
        let mut ir: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        validate_ir(&serde_json::to_vec(&ir).unwrap()).unwrap();
        ir["stateful_circuits"][0]["name"] = "differentCircuit".into();
        assert!(validate_ir(&serde_json::to_vec(&ir).unwrap()).is_err());
        assert!(validate_ir(b"{}").is_err());
        assert!(validate_ir(b"not JSON").is_err());
    }
}
