use compact_contract_nested_map_oracle::ledger_contract::initial_state;
use compact_contract_nested_map_oracle::runtime::{Field, context::ConstructorContext, ledger::ContractAddress};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = compact_contract_nested_map_oracle::ledger_slots::users_by_org.lookup(context, Field::from(1_u64));
}
