use midnight_compact_runtime::Field;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_map,
    insert_map, lookup_map, member_map,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;

#[test]
fn ledger_vm_inserts_checks_and_looks_up_typed_map_values() {
    let state = StateValue::Array(vec![constructor_map::<DefaultDB>()].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    let (result, present) = member_map(&context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);
    assert!(
        lookup_map::<bool, Field, _>(&result.context, 0, true, None, &INITIAL_COST_MODEL).is_err()
    );

    let result = insert_map(
        &result.context,
        0,
        true,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let (result, present) =
        member_map(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(present);
    let (result, value) =
        lookup_map::<bool, Field, _>(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(value, Field::from(42_u64));
    let result = insert_map(
        &result.context,
        0,
        true,
        Field::from(7_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let (result, value) =
        lookup_map::<bool, Field, _>(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(value, Field::from(7_u64));
    let (_, present) = member_map(&result.context, 0, false, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);
}
