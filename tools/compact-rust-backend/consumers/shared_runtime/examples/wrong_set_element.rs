use compact_contract_set_oracle::ledger_contract::initial_state;
use compact_contract_set_oracle::runtime::context::ConstructorContext;
use compact_contract_set_oracle::runtime::ledger::ContractAddress;
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = compact_contract_set_oracle::ledger_slots::s.insert(context, true);
}
