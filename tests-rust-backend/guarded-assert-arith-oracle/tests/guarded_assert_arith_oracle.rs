use compact_rust_guarded_assert_arith_oracle_fixture::ledger_contract::{
    initial_state, recordFreshEnough,
};
use compact_rust_guarded_assert_arith_oracle_fixture::pure_circuits::{
    ageGap, assertAgeWithin, assertFreshEnough,
};
use compact_rust_guarded_assert_arith_oracle_fixture::types::{
    Attestation, StatusProof, VerifierPolicy,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn uint(value: u128) -> runtime::BoundedUint<{ u64::MAX as u128 }> {
    runtime::BoundedUint::new(value).unwrap()
}

fn attestation(created_at: u128) -> Attestation {
    Attestation {
        proof: StatusProof {
            createdAt: uint(created_at),
            issuer: uint(1),
        },
        hasExpiration: false,
        expiresAt: uint(0),
    }
}

fn policy(enforce_max_age: bool) -> VerifierPolicy {
    VerifierPolicy {
        enforceMaxAge: enforce_max_age,
        maxAge: uint(20),
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"recordFreshEnough".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assertion_error<T>(result: Result<T, runtime::CompactError>, oracle: &serde_json::Value) {
    let error = result.err().expect("assertion should fail");
    assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
    assert_eq!(error.to_string(), oracle["error"]);
    assert_eq!(oracle["compactError"], true);
}

#[test]
fn exact_guarded_nested_arithmetic_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/guarded-assert-arith-oracle.json"
    ))
    .unwrap();
    let att = attestation(100);
    assert!(assertFreshEnough(policy(true), att.clone(), uint(110)).is_ok());
    assert_eq!(oracle["fresh"]["ok"], true);
    assertion_error(
        assertFreshEnough(policy(true), att.clone(), uint(130)),
        &oracle["tooOld"],
    );
    assertion_error(
        assertFreshEnough(policy(true), att.clone(), uint(90)),
        &oracle["future"],
    );
    assert!(assertFreshEnough(policy(false), att.clone(), uint(130)).is_ok());
    assert_eq!(oracle["withoutMaxAge"]["ok"], true);
    assert!(assertAgeWithin(att.clone(), uint(110), uint(10)).is_ok());
    assert_eq!(oracle["withinAge"]["ok"], true);
    assertion_error(
        assertAgeWithin(att.clone(), uint(111), uint(10)),
        &oracle["outsideAge"],
    );
    assert_eq!(
        ageGap(attestation(120), att.clone())
            .unwrap()
            .value()
            .to_string(),
        oracle["ageGap"]
    );
    assertion_error(ageGap(att.clone(), attestation(120)), &oracle["reverseGap"]);

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let accepted = recordFreshEnough(
        initial.into_circuit_context(ContractAddress::default()),
        policy(true),
        att.clone(),
        uint(110),
    )
    .unwrap();
    assert_eq!(
        state_hex(accepted.context.query.state.get_ref().clone()),
        oracle["afterRecordFresh"]
    );
    let StateValue::Array(fields) = accepted.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let count = runtime::ledger::read_counter(&fields.get(0).unwrap()).unwrap();
    assert_eq!(count.to_string(), oracle["countAfterRecordFresh"]);
    assertion_error(
        recordFreshEnough(accepted.context, policy(true), att, uint(130)),
        &oracle["recordTooOld"],
    );
    assert_eq!(oracle["afterRejectedRecord"], oracle["afterRecordFresh"]);
}
