use compact_contract_list_field::ledger_contract::initial_state;
use compact_contract_list_field::runtime::{context::ConstructorContext, ledger::ContractAddress};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = compact_contract_list_field::ledger_slots::items.push_front(context, true);
}
