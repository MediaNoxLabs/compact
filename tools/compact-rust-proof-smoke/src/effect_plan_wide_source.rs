// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-bound wide-value proof qualification, distinct from synthetic probes.
use super::effectful_scenario::ChooseScenario;
use super::*;
use compact_rust_effect_plan_wide_source_fixture::ledger_contract as contract;
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{CircuitContext, CircuitResult};
use midnight_compact_runtime::ledger::ContractAddress;

struct Mark;
impl contract::Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, contract::LedgerView<'_>>) -> (u64, Field) {
        (*context.private_state + 1, Field::from(77_u64))
    }
}

struct Scenario;
impl ChooseScenario for Scenario {
    fn initial_context() -> Result<CircuitContext<u64>, CompactError> {
        Ok(contract::initial_state(ConstructorContext::new(0_u64))?
            .into_circuit_context(ContractAddress::default()))
    }
    fn native(
        context: CircuitContext<u64>,
        next: Field,
    ) -> Result<CircuitResult<u64, Field>, CompactError> {
        contract::choose(context, &Mark, next)
    }
    fn recorded(
        context: CircuitContext<u64>,
        next: Field,
    ) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
        contract::recorded::choose(context, &Mark, next)
    }
    fn prepared(
        observed: &ObservedContractState,
        next: Field,
        verifier: VerifierKey,
    ) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>> {
        let contract = contract::Contract::from(Mark);
        let call = contract.recording().choose_call(observed, 0_u64, next)?;
        Ok(call.prepare(verifier, Fr::from(0_u64))?)
    }
}

fn validate_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    let expected: serde_json::Value =
        serde_json::from_str(compact_rust_effect_plan_wide_source_fixture::RETAINED_IR)?;
    if actual != expected {
        return Err("wide source proof artifacts do not match the generated fixture IR".into());
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // An input identity check before keys are loaded, not authenticated key provenance.
    validate_ir(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    effectful_scenario::run::<Scenario>(root, "effect plan wide source")
}

#[cfg(test)]
mod tests {
    use super::validate_ir;

    #[test]
    fn source_pairing_accepts_formatting_and_refuses_different_contracts() {
        let retained: serde_json::Value =
            serde_json::from_str(compact_rust_effect_plan_wide_source_fixture::RETAINED_IR)
                .unwrap();
        validate_ir(&serde_json::to_vec(&retained).unwrap()).unwrap();
        let mut changed = retained;
        changed["stateful_circuits"][0]["name"] = "different".into();
        assert!(validate_ir(&serde_json::to_vec(&changed).unwrap()).is_err());
        assert!(
            validate_ir(include_bytes!(
                "../../compact-rust-backend/tests/effectful-return-schema20-ir.json"
            ))
            .is_err()
        );
        assert!(validate_ir(b"{}").is_err());
        assert!(validate_ir(b"not JSON").is_err());
    }
}
