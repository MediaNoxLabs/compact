use compact_rust_hmt_insert_oracle_fixture::ledger_contract::{
    append, forget_history, full, initial_state, known, place,
};
use compact_rust_hmt_insert_oracle_fixture::types::MerkleTreeDigest;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["append", "place", "forget_history", "full", "known"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn bounded<const MAX: u128>(value: u128) -> runtime::BoundedUint<MAX> {
    runtime::BoundedUint::new(value).unwrap()
}

fn current_root(state: &StateValue<DefaultDB>) -> MerkleTreeDigest {
    let root = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
        .unwrap()
        .root()
        .unwrap();
    MerkleTreeDigest { field: root.0 }
}

fn assert_state(
    state: &StateValue<DefaultDB>,
    oracle: &serde_json::Value,
    step: &str,
    first_free: u128,
) {
    assert_eq!(
        state_hex(state.clone()),
        oracle[step],
        "state mismatch at {step}"
    );
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    assert_eq!(tree.first_free().unwrap().value(), first_free);
    assert!(tree.root().is_some());
}

#[test]
fn historic_merkle_insert_modes_match_typescript_state_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_state(initial.ledger_state.get_ref(), &oracle, "afterInit", 0);
    let initial_root = current_root(initial.ledger_state.get_ref());

    let context = initial.into_circuit_context(ContractAddress::default());
    let initial_full = full(context).unwrap();
    assert_eq!(initial_full.result, oracle["fullAtInit"]);
    let initial_known = known(initial_full.context, initial_root.clone()).unwrap();
    assert_eq!(initial_known.result, oracle["knownAtInit"]);
    let after_append7 = append(initial_known.context, bounded::<255>(7)).unwrap();
    assert_state(
        after_append7.context.query.state.get_ref(),
        &oracle,
        "afterAppend7",
        1,
    );

    let known_old = known(after_append7.context, initial_root.clone()).unwrap();
    assert_eq!(known_old.result, oracle["knownInitialAfterAppend"]);
    let after_place9 = place(
        known_old.context,
        bounded::<255>(9),
        bounded::<{ u64::MAX as u128 }>(3),
    )
    .unwrap();
    assert_state(
        after_place9.context.query.state.get_ref(),
        &oracle,
        "afterPlace9At3",
        4,
    );

    let after_append11 = append(after_place9.context, bounded::<255>(11)).unwrap();
    assert_state(
        after_append11.context.query.state.get_ref(),
        &oracle,
        "afterAppend11",
        5,
    );

    let after_place13 = place(
        after_append11.context,
        bounded::<255>(13),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap();
    assert_state(
        after_place13.context.query.state.get_ref(),
        &oracle,
        "afterPlace13At1",
        5,
    );

    let root_before_reset = current_root(after_place13.context.query.state.get_ref());
    let after_forget = forget_history(after_place13.context).unwrap();
    assert_state(
        after_forget.context.query.state.get_ref(),
        &oracle,
        "afterForgetHistory",
        5,
    );
    let forgotten_old = known(after_forget.context, initial_root).unwrap();
    assert_eq!(forgotten_old.result, oracle["knownInitialAfterReset"]);
    let retained_current = known(forgotten_old.context, root_before_reset).unwrap();
    assert_eq!(retained_current.result, oracle["knownCurrentAfterReset"]);
    let before_capacity = full(retained_current.context).unwrap();
    assert_eq!(before_capacity.result, oracle["fullBeforeCapacity"]);
    let after_21 = append(before_capacity.context, bounded::<255>(21)).unwrap();
    let after_22 = append(after_21.context, bounded::<255>(22)).unwrap();
    let after_23 = append(after_22.context, bounded::<255>(23)).unwrap();
    assert_state(
        after_23.context.query.state.get_ref(),
        &oracle,
        "afterCapacity",
        8,
    );
    let at_capacity = full(after_23.context).unwrap();
    assert_eq!(at_capacity.result, oracle["fullAtCapacity"]);
}
