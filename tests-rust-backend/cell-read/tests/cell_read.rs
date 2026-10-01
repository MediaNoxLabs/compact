use compact_rust_cell_read_fixture::ledger_contract::{initial_state, read_flag};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::ContractAddress;

#[test]
fn generated_cell_read_uses_ledger_gather_event() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let read = read_flag(context).unwrap();
    assert!(!read.result);

    let written = read.context.write_cell(0, true).unwrap();
    let read = read_flag(written.context).unwrap();
    assert!(read.result);
}
