use compact_contract_map_boolean_field::ledger_contract::initial_state;
use compact_contract_map_boolean_field::runtime::{context::ConstructorContext, ledger::ContractAddress};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = compact_contract_map_boolean_field::ledger_slots::table.insert(context, true, false);
}
