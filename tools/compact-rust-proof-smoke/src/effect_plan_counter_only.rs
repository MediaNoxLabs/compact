// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Proof adapter bound to one actual generated source crate.
use super::counter_scenario::{self, CounterScenario};
use super::*;
use compact_rust_effect_plan_counter_only_source_fixture as source;
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{CircuitContext, CircuitResult, ConstructorResult};
use source::ledger_contract as contract;

type Amount = BoundedUint<65535>;
struct Scenario;
impl CounterScenario for Scenario {
    fn initial_state() -> Result<ConstructorResult<u64>, CompactError> {
        contract::initial_state(ConstructorContext::new(7_u64))
    }
    fn native(
        context: CircuitContext<u64>,
        amount: Amount,
        remove: Amount,
    ) -> Result<CircuitResult<u64, Field>, CompactError> {
        contract::adjust(context, amount, remove)
    }
    fn recorded(
        context: CircuitContext<u64>,
        amount: Amount,
        remove: Amount,
    ) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
        contract::recorded::adjust(context, amount, remove)
    }
    fn prepared(
        observed: &ObservedContractState,
        amount: Amount,
        remove: Amount,
        verifier: VerifierKey,
    ) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>> {
        Ok(contract::Contract::default()
            .recording
            .adjust_call(observed, 7_u64, amount, remove)?
            .prepare(verifier, Fr::from(0_u64))?)
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::run::<Scenario>(root, source::RETAINED_IR, "Counter-only frontend")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_artifact_mismatch_before_proof_work() {
        counter_scenario::validate_ir(source::RETAINED_IR.as_bytes(), source::RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(source::RETAINED_IR).unwrap();
        changed["stateful_circuits"][0]["name"] = "different".into();
        assert!(
            counter_scenario::validate_ir(
                &serde_json::to_vec(&changed).unwrap(),
                source::RETAINED_IR
            )
            .is_err()
        );
        assert!(
            counter_scenario::validate_ir(
                compact_rust_effect_plan_counter_source_fixture::RETAINED_IR.as_bytes(),
                source::RETAINED_IR
            )
            .is_err()
        );
        assert!(counter_scenario::validate_ir(b"{}", source::RETAINED_IR).is_err());
    }
}
