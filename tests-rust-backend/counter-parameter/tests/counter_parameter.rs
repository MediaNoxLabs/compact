use compact_rust_counter_parameter_fixture::ledger_contract::{
    decrement_by, increment_by, initial_state, reset_round,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_counter};

#[test]
fn generated_counter_uses_bounded_parameter() {
    let constructor = initial_state(ConstructorContext::new(()));
    let context = constructor.into_circuit_context(ContractAddress::default());
    let amount = BoundedUint::<65535>::new(7).unwrap();
    let result = increment_by(context, amount).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 7);

    let result = increment_by(result.context, BoundedUint::<65535>::new(2).unwrap()).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 9);

    let result = decrement_by(result.context, BoundedUint::<65535>::new(5).unwrap()).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 4);
    let result = reset_round(result.context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 0);
    assert!(decrement_by(result.context, BoundedUint::<65535>::new(1).unwrap()).is_err());
}
