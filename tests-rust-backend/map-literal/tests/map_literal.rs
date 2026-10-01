use compact_rust_map_literal_fixture::ledger_contract::{initial_state, lookup_true, put_literal};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::ContractAddress;

#[test]
fn compiler_typed_local_flows_into_map_insert() {
    let context =
        initial_state(ConstructorContext::new(())).into_circuit_context(ContractAddress::default());
    let write = put_literal(context).unwrap();
    let read = lookup_true(write.context).unwrap();
    assert_eq!(read.result, Field::from(42_u64));
}
