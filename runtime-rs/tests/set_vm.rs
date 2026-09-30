use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_set,
    insert_set, is_empty_set, member_set, remove_set, reset_set, size_set,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;

#[test]
fn ledger_vm_inserts_and_checks_a_boolean_set() {
    let state = StateValue::Array(vec![constructor_set::<DefaultDB>()].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    let (result, empty) = is_empty_set(&context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(empty);
    let (result, size) = size_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(size, 0);
    let (result, present) =
        member_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);

    let result = insert_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    let (result, present) =
        member_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(present);
    let (result, present) =
        member_set(&result.context, 0, false, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);
    let (result, size) = size_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(size, 1);
    let (result, empty) = is_empty_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!empty);
    let result = remove_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    let (result, empty) = is_empty_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(empty);
    let result = insert_set(&result.context, 0, false, None, &INITIAL_COST_MODEL).unwrap();
    let result = reset_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    let (_, empty) = is_empty_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(empty);
}
