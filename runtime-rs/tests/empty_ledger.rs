use midnight_compact_runtime::ledger::{empty_contract_state, empty_query_context};

#[test]
fn empty_contract_uses_the_ledger_state_and_query_context() {
    let state = empty_contract_state();
    let context = empty_query_context();
    assert_eq!(state, context.state);
}
