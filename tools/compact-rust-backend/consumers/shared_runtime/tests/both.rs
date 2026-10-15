use compact_contract_counter::ledger_contract::{Contract, initial_state};
use compact_contract_cell_boolean::ledger_contract::{Contract as CellContract, initial_state as initial_cell_state};
use compact_contract_field_add::pure_circuits::field_add;
use compact_contract_set_oracle::ledger_contract::{Contract as SetContract, initial_state as initial_set_state};
use compact_contract_map_boolean_field::ledger_contract::{Contract as MapContract, initial_state as initial_map_state};
use compact_contract_nested_map_oracle::ledger_contract::initial_state as initial_nested_map_state;
use compact_contract_list_field::ledger_contract::{Contract as ListContract, initial_state as initial_list_state};
use compact_contract_merkle_tree_oracle::ledger_contract::initial_state as initial_merkle_state;
use compact_contract_hmt_insert_oracle::ledger_contract::initial_state as initial_historic_merkle_state;
use compact_contract_vector_key_adt::ledger_contract::{Contract as VectorContract, initial_state as initial_vector_state};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::ContractAddress;
use midnight_compact_runtime::Field;
#[test]
fn two_generated_contracts_share_one_runtime() {
    let state = initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let result = Contract::default().increment(context).unwrap();
    let read = Contract::default().read_round(result.context).unwrap();
    assert_eq!(read.result.value(), 1);
    let cell = initial_cell_state(ConstructorContext::new(())).unwrap();
    let cell_context = cell.into_circuit_context(ContractAddress::default());
    let _: midnight_compact_runtime::slots::CellSlot<bool> = compact_contract_cell_boolean::ledger_slots::flag;
    let cell_call = CellContract::default().recording.set_flag(cell_context).unwrap();
    let cell_replay = cell_call.public.initial().query(
        cell_call.public.verify_ops(), None, &cell_call.execution.context.cost_model,
    ).unwrap();
    assert_eq!(cell_replay.context.effects, cell_call.execution.context.query.effects);
    let set = initial_set_state(ConstructorContext::new(())).unwrap();
    let set_context = set.into_circuit_context(ContractAddress::default());
    let set_step = compact_contract_set_oracle::ledger_slots::s.insert(set_context, Field::from(7_u64)).unwrap();
    let member = compact_contract_set_oracle::ledger_slots::s.member(set_step.context, Field::from(7_u64)).unwrap();
    assert!(member.result);
    let checked = SetContract::default().check(member.context, Field::from(7_u64)).unwrap();
    assert!(compact_contract_set_oracle::runtime::ledger::read_root_cell::<bool, _>(checked.context.query.state.get_ref(), 0).unwrap());
    let map = initial_map_state(ConstructorContext::new(())).unwrap();
    let map_context = map.into_circuit_context(ContractAddress::default());
    let _: midnight_compact_runtime::slots::MapSlot<bool, Field> = compact_contract_map_boolean_field::ledger_slots::table;
    let map_call = MapContract::default().recording.put(map_context, true, Field::from(9_u64)).unwrap();
    let map_replay = map_call.public.initial().query(
        map_call.public.verify_ops(), None, &map_call.execution.context.cost_model,
    ).unwrap();
    assert_eq!(map_replay.context.effects, map_call.execution.context.query.effects);
    let lookup = MapContract::default().recording.get(map_call.execution.context, true).unwrap();
    assert_eq!(lookup.execution.result, Field::from(9_u64));
    let nested = initial_nested_map_state(ConstructorContext::new(())).unwrap();
    let nested_context = nested.into_circuit_context(ContractAddress::default());
    let _: midnight_compact_runtime::slots::MapSlot<Field, midnight_compact_runtime::slots::MapNode<Field, midnight_compact_runtime::BoundedUint<18446744073709551615>>> = compact_contract_nested_map_oracle::ledger_slots::users_by_org;
    let nested_empty = compact_contract_nested_map_oracle::ledger_slots::users_by_org.is_empty(nested_context).unwrap();
    assert!(nested_empty.result);
    let list = initial_list_state(ConstructorContext::new(())).unwrap();
    let list_context = list.into_circuit_context(ContractAddress::default());
    let _: midnight_compact_runtime::slots::ListSlot<Field> = compact_contract_list_field::ledger_slots::items;
    let list_call = ListContract::default().recording.prepend(list_context, Field::from(5_u64)).unwrap();
    let list_replay = list_call.public.initial().query(
        list_call.public.verify_ops(), None, &list_call.execution.context.cost_model,
    ).unwrap();
    assert_eq!(list_replay.context.effects, list_call.execution.context.query.effects);
    let first = ListContract::default().recording.first_item(list_call.execution.context).unwrap();
    assert!(first.execution.result.is_some);
    assert_eq!(first.execution.result.value, Field::from(5_u64));
    let merkle = initial_merkle_state(ConstructorContext::new(())).unwrap();
    let merkle_context = merkle.into_circuit_context(ContractAddress::default());
    let _: midnight_compact_runtime::slots::MerkleSlot<midnight_compact_runtime::BoundedUint<255>, 3, false> = compact_contract_merkle_tree_oracle::ledger_slots::t;
    let merkle_insert = compact_contract_merkle_tree_oracle::ledger_slots::t.insert(merkle_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let merkle_full = compact_contract_merkle_tree_oracle::ledger_slots::t.is_full(merkle_insert.context).unwrap();
    assert!(!merkle_full.result);
    let merkle = initial_merkle_state(ConstructorContext::new(())).unwrap();
    let merkle_context = merkle.into_circuit_context(ContractAddress::default());
    let recorded_merkle = compact_contract_merkle_tree_oracle::ledger_contract::recorded::append(merkle_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let merkle_replay = recorded_merkle.public.initial().query(recorded_merkle.public.verify_ops(), None, &recorded_merkle.execution.context.cost_model).unwrap();
    assert_eq!(merkle_replay.context.effects, recorded_merkle.execution.context.query.effects);
    let recorded_merkle_full = compact_contract_merkle_tree_oracle::ledger_contract::recorded::full(recorded_merkle.execution.context).unwrap();
    assert!(!recorded_merkle_full.execution.result);
    let merkle_full_replay = recorded_merkle_full.public.initial().query(recorded_merkle_full.public.verify_ops(), None, &recorded_merkle_full.execution.context.cost_model).unwrap();
    assert_eq!(merkle_full_replay.context.effects, recorded_merkle_full.execution.context.query.effects);
    let historic = initial_historic_merkle_state(ConstructorContext::new(())).unwrap();
    let historic_context = historic.into_circuit_context(ContractAddress::default());
    let _: midnight_compact_runtime::slots::MerkleSlot<midnight_compact_runtime::BoundedUint<255>, 3, true> = compact_contract_hmt_insert_oracle::ledger_slots::t;
    let historic_insert = compact_contract_hmt_insert_oracle::ledger_slots::t.insert(historic_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let _historic_reset = compact_contract_hmt_insert_oracle::ledger_slots::t.reset_history(historic_insert.context).unwrap();
    let historic = initial_historic_merkle_state(ConstructorContext::new(())).unwrap();
    let historic_context = historic.into_circuit_context(ContractAddress::default());
    let recorded_historic = compact_contract_hmt_insert_oracle::ledger_contract::recorded::append(historic_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let historic_replay = recorded_historic.public.initial().query(recorded_historic.public.verify_ops(), None, &recorded_historic.execution.context.cost_model).unwrap();
    assert_eq!(historic_replay.context.effects, recorded_historic.execution.context.query.effects);
    let recorded_historic_full = compact_contract_hmt_insert_oracle::ledger_contract::recorded::full(recorded_historic.execution.context).unwrap();
    assert!(!recorded_historic_full.execution.result);
    let historic_full_replay = recorded_historic_full.public.initial().query(recorded_historic_full.public.verify_ops(), None, &recorded_historic_full.execution.context.cost_model).unwrap();
    assert_eq!(historic_full_replay.context.effects, recorded_historic_full.execution.context.query.effects);
    let vector = initial_vector_state(ConstructorContext::new(())).unwrap();
    let vector_context = vector.into_circuit_context(ContractAddress::default());
    let vector_call = VectorContract::default().recording.setInsert(vector_context).unwrap();
    let vector_replay = vector_call.public.initial().query(vector_call.public.verify_ops(), None, &vector_call.execution.context.cost_model).unwrap();
    assert_eq!(vector_replay.context.effects, vector_call.execution.context.query.effects);
    let vector_member = VectorContract::default().recording.setMember(vector_call.execution.context).unwrap();
    assert!(midnight_compact_runtime::ledger::read_root_cell::<bool, _>(vector_member.execution.context.query.state.get_ref(), 2).unwrap());
    assert_eq!(field_add(2u64.into(), 3u64.into()).unwrap(), 5u64.into());
}
