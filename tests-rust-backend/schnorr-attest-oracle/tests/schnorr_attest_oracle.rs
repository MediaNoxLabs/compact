use compact_rust_schnorr_attest_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, acceptAttestation, initial_state, verifyAttestation,
};
use compact_rust_schnorr_attest_oracle_fixture::pure_circuits::attestationDigest;
use compact_rust_schnorr_attest_oracle_fixture::types::SchnorrSignature;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, Field, FixedBytes, JubjubPoint, WideUint};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

type Uint248 = WideUint<
    1329227995784915872903807060280344575_u128,
    340282366920938463463374607431768211455_u128,
>;

struct OracleWitness;

impl Witnesses<u64> for OracleWitness {
    fn getSchnorrReduction(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        challenge_hash: Field,
    ) -> (u64, (BoundedUint<127>, Uint248)) {
        let bytes = challenge_hash.as_le_bytes();
        let quotient = BoundedUint::<127>::new(bytes[31] as u128).unwrap();
        let remainder = Uint248::from_le_bytes(&bytes[..31]).unwrap();
        (*context.private_state + 1, (quotient, remainder))
    }

    fn localAttestorKey(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, JubjubPoint) {
        (
            *context.private_state + 1,
            midnight_compact_runtime::ec_mul_generator(Field::from(1_u64)).unwrap(),
        )
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["verifyAttestation", "acceptAttestation"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn schnorr_constructor_and_digest_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/schnorr-attest-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64), &OracleWitness).unwrap();
    assert_eq!(
        initial.private_state,
        reference["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let digest = attestationDigest(
        FixedBytes::new([0_u8; 32]),
        BoundedUint::<{ u64::MAX as u128 }>::new(1).unwrap(),
        Field::from(2_u64),
    )
    .unwrap();
    let digest_hex = digest
        .0
        .iter()
        .map(|field| hex::encode(field.as_le_bytes()))
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(digest_hex).unwrap(),
        reference["digestHex"]
    );
    let generator = midnight_compact_runtime::ec_mul_generator(Field::from(1_u64)).unwrap();
    assert_eq!(
        hex::encode(generator.x().unwrap().as_le_bytes()),
        reference["generator"]["x"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(generator.y().unwrap().as_le_bytes()),
        reference["generator"]["y"].as_str().unwrap()
    );
    let response_bytes = hex::decode(reference["signatureResponseHex"].as_str().unwrap()).unwrap();
    let signature = SchnorrSignature {
        announcement: generator,
        response: Field::from_le_bytes(&response_bytes).unwrap(),
    };
    let context = initial.into_circuit_context(ContractAddress::default());
    let verified =
        verifyAttestation(context, &OracleWitness, digest.clone(), signature.clone()).unwrap();
    assert_eq!(
        verified.context.private_state,
        reference["afterVerifyPrivateState"].as_u64().unwrap()
    );
    assert_transcript(
        &verified.private_transcript_outputs,
        &reference["verifyTranscript"],
    );
    let accepted = acceptAttestation(verified.context, &OracleWitness, digest, signature).unwrap();
    assert_eq!(
        accepted.context.private_state,
        reference["afterAcceptPrivateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(accepted.context.query.state.get_ref().clone()),
        reference["afterAcceptHex"]
    );
    assert_transcript(
        &accepted.private_transcript_outputs,
        &reference["acceptTranscript"],
    );
}

fn assert_transcript(
    actual: &[midnight_compact_runtime::fab::AlignedValue],
    expected: &serde_json::Value,
) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        let atoms = actual
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
        assert_eq!(
            serde_json::to_value(&actual.alignment).unwrap(),
            expected["alignment"]
        );
    }
}
