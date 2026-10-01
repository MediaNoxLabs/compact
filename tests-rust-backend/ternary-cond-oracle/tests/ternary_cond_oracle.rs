use compact_rust_ternary_cond_oracle_fixture::ledger_contract::{
    initial_state, streamCompareEq, streamIncrement, streamWrite, walkerCallPure, walkerCompareEq,
    walkerConstAnnotated, walkerStructMember, walkerWrite,
};
use compact_rust_ternary_cond_oracle_fixture::pure_circuits::{
    constAnnotatedBothLiteral, constUnannotatedSeqLifted, enumValued, returnTailNested,
};
use compact_rust_ternary_cond_oracle_fixture::types::Color;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, Field};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

const EXPORTED: &[&str] = &[
    "walkerConstAnnotated",
    "walkerCompareEq",
    "walkerCallPure",
    "walkerStructMember",
    "walkerWrite",
    "witnessArg",
    "streamIncrement",
    "streamCompareEq",
    "streamWrite",
    "walkerVectorElement",
    "walkerNativeArg",
    "walkerNestedIf",
    "walkerInlineWrite",
    "streamCallPure",
    "streamVectorElement",
    "streamNativeArg",
    "streamStructMember",
    "streamCallWitness",
    "streamConstAnnotated",
    "streamAssertEq",
    "streamNestedIf",
];

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in EXPORTED {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-cond-oracle.json"
    ))
    .unwrap()
}

fn initial(c: bool, d: bool, x: u64) -> midnight_compact_runtime::context::ConstructorResult<()> {
    initial_state(ConstructorContext::new(()), c, d, Field::from(x)).unwrap()
}

#[test]
fn constructor_and_stateful_ternary_routes_match_typescript_bytes() {
    let reference = oracle();
    assert_eq!(
        state_hex(initial(true, true, 111).ledger_state.get_ref().clone()),
        reference["afterInit"]["stateHex"]
    );
    assert_eq!(
        state_hex(initial(false, false, 222).ledger_state.get_ref().clone()),
        reference["afterInitFalse"]["stateHex"]
    );

    for label in [
        "afterWalkerConstAnnotated",
        "afterWalkerCompareEq",
        "afterWalkerCallPure",
        "afterWalkerStructMember",
        "afterWalkerWrite",
        "afterStreamIncrement",
        "afterStreamCompareEq",
        "afterStreamWrite",
    ] {
        let context = initial(true, true, 111).into_circuit_context(ContractAddress::default());
        let next = match label {
            "afterWalkerConstAnnotated" => walkerConstAnnotated(context, true).unwrap().context,
            "afterWalkerCompareEq" => {
                walkerCompareEq(context, true, BoundedUint::<255>::new(1).unwrap())
                    .unwrap()
                    .context
            }
            "afterWalkerCallPure" => walkerCallPure(context, true).unwrap().context,
            "afterWalkerStructMember" => walkerStructMember(context, true).unwrap().context,
            "afterWalkerWrite" => {
                walkerWrite(context, true, Field::from(555_u64))
                    .unwrap()
                    .context
            }
            "afterStreamIncrement" => streamIncrement(context).unwrap().context,
            "afterStreamCompareEq" => streamCompareEq(context).unwrap().context,
            "afterStreamWrite" => {
                streamWrite(context, false, Field::from(777_u64))
                    .unwrap()
                    .context
            }
            _ => unreachable!(),
        };
        assert_eq!(
            state_hex(next.query.state.get_ref().clone()),
            reference[label]["stateHex"],
            "{label}"
        );
    }
}

#[test]
fn pure_ternary_arms_and_subtraction_guard_execute() {
    assert_eq!(constAnnotatedBothLiteral(true).unwrap().value(), 1);
    assert_eq!(constAnnotatedBothLiteral(false).unwrap().value(), 2);
    for (c, d, expected) in [
        (true, true, 1),
        (true, false, 2),
        (false, true, 3),
        (false, false, 4),
    ] {
        assert_eq!(returnTailNested(c, d).unwrap().value(), expected);
    }
    assert_eq!(enumValued(true).unwrap(), Color::red);
    assert_eq!(enumValued(false).unwrap(), Color::green);
    let zero = BoundedUint::<255>::new(0).unwrap();
    let five = BoundedUint::<255>::new(5).unwrap();
    assert_eq!(constUnannotatedSeqLifted(false, zero).unwrap().value(), 0);
    assert_eq!(constUnannotatedSeqLifted(true, five).unwrap().value(), 4);
    assert!(constUnannotatedSeqLifted(true, zero).is_err());
}
