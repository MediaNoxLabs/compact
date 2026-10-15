use compact_contract_merkle_tree_oracle::ledger_contract::initial_state;
use compact_contract_merkle_tree_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = compact_contract_merkle_tree_oracle::ledger_slots::t.reset_history(context);
}
