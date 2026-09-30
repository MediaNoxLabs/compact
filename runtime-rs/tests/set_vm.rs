use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_set,
    insert_set, member_set,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;

#[test]
fn ledger_vm_inserts_and_checks_a_boolean_set() {
    let state = StateValue::Array(vec![constructor_set::<DefaultDB>()].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    let (result, present) = member_set(&context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);

    let result = insert_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    let (result, present) =
        member_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(present);
    let (_, present) = member_set(&result.context, 0, false, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);
}
