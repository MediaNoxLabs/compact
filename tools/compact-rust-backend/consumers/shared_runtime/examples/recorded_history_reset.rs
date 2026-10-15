use compact_contract_hmt_insert_oracle::ledger_contract::{initial_state, recorded};
use compact_contract_hmt_insert_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = recorded::forget_history(context).unwrap();
}
