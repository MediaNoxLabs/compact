use compact_contract_hmt_insert_oracle::ledger_contract::{initial_state, recorded, PublicStateView};
use compact_contract_hmt_insert_oracle::types::MerkleTreeDigest;
use compact_contract_hmt_insert_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let root = PublicStateView::from(state.ledger_state.get_ref()).t().unwrap().root().unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _ = recorded::known(context, MerkleTreeDigest { field: root.0 }).unwrap();
}
