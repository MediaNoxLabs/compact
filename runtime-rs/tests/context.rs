use midnight_compact_runtime::context::{ConstructorContext, ConstructorResult};
use midnight_compact_runtime::ledger::{ContractAddress, empty_contract_state};

#[test]
fn constructor_to_circuit_transfer_preserves_private_and_ledger_state() {
    let private = vec![1_u8, 2, 3];
    let initial = empty_contract_state();
    let constructor = ConstructorContext::new(private.clone());
    let result = ConstructorResult::new(constructor, initial.clone());
    let circuit = result.into_circuit_context(ContractAddress::default());
    assert_eq!(circuit.private_state, private);
    assert_eq!(circuit.query.state, initial);
    assert!(circuit.gas_limit.is_none());
}
