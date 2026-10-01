use compact_rust_call_arg_declared_type_fixture::ledger_contract::{
    LedgerView, Witnesses, bridgeTupleIntoVec, bridgeVecIntoTuple, commitFieldOnly, commitSmall,
    commitU128, hashPersistentVec, hashTransientVec, impureBare, impureConst, impureInIfArm,
    initial_state, inlinedAssert, pureBodyFieldOnly, pureBodyVec, pureFromImpure, witnessBare,
    witnessConst,
};
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

const CIRCUITS: &[&str] = &[
    "commitSmall",
    "commitU128",
    "commitFieldOnly",
    "pureBodyVec",
    "pureBodyFieldOnly",
    "bridgeTupleIntoVec",
    "bridgeVecIntoTuple",
    "witnessConst",
    "witnessBare",
    "pureFromImpure",
    "impureConst",
    "impureBare",
    "impureInIfArm",
    "inlinedAssert",
    "hashPersistentVec",
    "hashTransientVec",
];

struct SumWitness;

impl Witnesses<()> for SumWitness {
    fn sumWitness(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        values: FixedVector<Field, 2>,
    ) -> ((), Field) {
        ((), values.0[0] + values.0[1])
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in CIRCUITS {
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

#[test]
fn declared_call_arguments_match_typescript_state_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/call-arg-declared-type.json"
    ))
    .unwrap();
    let captured = oracle["circuits"].as_object().unwrap();
    assert_eq!(captured.len(), CIRCUITS.len());
    for name in CIRCUITS {
        assert!(captured.contains_key(*name));
    }

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]["stateHex"]
    );

    for name in CIRCUITS {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = match *name {
            "commitSmall" => commitSmall(context).unwrap().context,
            "commitU128" => commitU128(context).unwrap().context,
            "commitFieldOnly" => commitFieldOnly(context).unwrap().context,
            "pureBodyVec" => pureBodyVec(context).unwrap().context,
            "pureBodyFieldOnly" => pureBodyFieldOnly(context).unwrap().context,
            "bridgeTupleIntoVec" => bridgeTupleIntoVec(context).unwrap().context,
            "bridgeVecIntoTuple" => bridgeVecIntoTuple(context).unwrap().context,
            "witnessConst" => witnessConst(context, &SumWitness).unwrap().context,
            "witnessBare" => witnessBare(context, &SumWitness).unwrap().context,
            "pureFromImpure" => pureFromImpure(context).unwrap().context,
            "impureConst" => impureConst(context).unwrap().context,
            "impureBare" => impureBare(context).unwrap().context,
            "impureInIfArm" => impureInIfArm(context).unwrap().context,
            "inlinedAssert" => inlinedAssert(context).unwrap().context,
            "hashPersistentVec" => hashPersistentVec(context).unwrap().context,
            "hashTransientVec" => hashTransientVec(context).unwrap().context,
            _ => unreachable!(),
        };
        assert_eq!(
            state_hex(result.query.state.get_ref().clone()),
            oracle["circuits"][name]["stateHex"],
            "{name}"
        );
    }
}
