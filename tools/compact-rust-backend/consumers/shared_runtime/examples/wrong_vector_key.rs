use compact_contract_vector_key_adt::ledger_contract::initial_state;
use compact_contract_vector_key_adt::runtime::{context::ConstructorContext, ledger::ContractAddress, recording::RecordingFrame};
fn main() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let frame = RecordingFrame::new(state.into_circuit_context(ContractAddress::default()));
    let _ = compact_contract_vector_key_adt::ledger_slots::keys.record_insert(frame, [0u8, 1u8]);
}
