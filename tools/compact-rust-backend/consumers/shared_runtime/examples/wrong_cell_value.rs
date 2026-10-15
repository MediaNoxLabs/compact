use compact_contract_cell_boolean::ledger_contract::initial_state;
use compact_contract_cell_boolean::runtime::context::ConstructorContext;
use compact_contract_cell_boolean::runtime::ledger::ContractAddress;
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = compact_contract_cell_boolean::ledger_slots::flag.write(context, 7_u64);
}
