// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{
    ConstructorContext, ConstructorResult, RunningCost, WitnessReadMeter,
};
use midnight_compact_runtime::ledger::{
    ContractAddress, DefaultDB, constructor_cell, constructor_counter,
    constructor_historic_merkle_tree, constructor_list, constructor_map, constructor_merkle_tree,
    constructor_set, contract_state, metered_historic_merkle_tree_view_at_path, metered_list_view,
    metered_map_view_at_path, metered_merkle_tree_view_at_path, metered_set_view_at_path,
};
use midnight_compact_runtime::slots::{
    CellSlot, CounterSlot, ListSlot, MapSlot, MerkleSlot, SetSlot,
};

#[test]
fn typed_witness_slots_preserve_cell_cost_and_reject_wrong_shapes() {
    let context = || {
        ConstructorResult::new(
            ConstructorContext::new(()),
            contract_state(vec![
                constructor_cell::<bool, DefaultDB>(true),
                constructor_counter::<DefaultDB>(),
            ]),
        )
        .into_circuit_context(ContractAddress::default())
    };

    let circuit = context();
    let meter = WitnessReadMeter::new(&circuit);
    let direct_meter = WitnessReadMeter::new(&circuit);
    let slot = CellSlot::<bool>::new(&[0]);
    assert!(slot.witness_read(&meter).unwrap());
    assert!(direct_meter.read_cell::<bool>(&[0]).unwrap());
    assert_eq!(meter.gas_cost(), direct_meter.gas_cost());
    assert_eq!(CounterSlot::new(&[1]).witness_read(&meter).unwrap(), 0);
    assert_eq!(direct_meter.read_cell::<u64>(&[1]).unwrap(), 0);
    assert_eq!(meter.gas_cost(), direct_meter.gas_cost());

    assert!(matches!(
        CounterSlot::new(&[0]).witness_read(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        CellSlot::<bool>::new(&[1]).witness_read(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));

    let mut limited = context();
    limited.gas_limit = Some(RunningCost::ZERO);
    let limited_meter = WitnessReadMeter::new(&limited);
    assert!(matches!(
        slot.witness_read(&limited_meter),
        Err(CompactError::LedgerQueryRejected(_))
    ));
}

#[test]
fn typed_witness_views_preserve_validation_reads_and_cost() {
    let context = || {
        ConstructorResult::new(
            ConstructorContext::new(()),
            contract_state(vec![
                constructor_set::<DefaultDB>(),
                constructor_map::<DefaultDB>(),
                constructor_list::<DefaultDB>(),
                constructor_merkle_tree::<DefaultDB>(3),
                constructor_historic_merkle_tree::<DefaultDB>(4),
            ]),
        )
        .into_circuit_context(ContractAddress::default())
    };
    let circuit = context();
    let meter = WitnessReadMeter::new(&circuit);
    let direct = WitnessReadMeter::new(&circuit);

    assert_eq!(
        SetSlot::<bool>::new(&[0])
            .witness_view(&meter)
            .unwrap()
            .size()
            .unwrap(),
        metered_set_view_at_path::<bool, _>(&direct, &[0])
            .unwrap()
            .size()
            .unwrap()
    );
    assert_eq!(
        MapSlot::<bool, bool>::new(&[1])
            .witness_view(&meter)
            .unwrap()
            .is_empty()
            .unwrap(),
        metered_map_view_at_path::<bool, bool, _>(&direct, &[1])
            .unwrap()
            .is_empty()
            .unwrap()
    );
    assert_eq!(
        ListSlot::<bool>::new(&[2])
            .witness_view(&meter)
            .unwrap()
            .length()
            .unwrap(),
        metered_list_view::<bool, _>(&direct, 2)
            .unwrap()
            .length()
            .unwrap()
    );
    assert_eq!(
        MerkleSlot::<bool, 3, false>::new(&[3])
            .witness_view::<u8, _>(&meter)
            .unwrap()
            .is_full()
            .unwrap(),
        metered_merkle_tree_view_at_path::<u8, _>(&direct, &[3], 3)
            .unwrap()
            .is_full()
            .unwrap()
    );
    assert_eq!(
        MerkleSlot::<bool, 4, true>::new(&[4])
            .witness_view::<u8, _>(&meter)
            .unwrap()
            .is_full()
            .unwrap(),
        metered_historic_merkle_tree_view_at_path::<u8, _>(&direct, &[4], 4)
            .unwrap()
            .is_full()
            .unwrap()
    );
    assert_eq!(meter.gas_cost(), direct.gas_cost());

    assert!(matches!(
        SetSlot::<bool>::new(&[2]).witness_view(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        MapSlot::<bool, bool>::new(&[2]).witness_view(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        ListSlot::<bool>::new(&[0]).witness_view(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        MerkleSlot::<bool, 3, false>::new(&[4]).witness_view::<u8, _>(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        MerkleSlot::<bool, 4, true>::new(&[3]).witness_view::<u8, _>(&meter),
        Err(CompactError::InvalidLedgerCell(_))
    ));

    let mut limited = context();
    limited.gas_limit = Some(RunningCost::ZERO);
    let limited_meter = WitnessReadMeter::new(&limited);
    assert!(matches!(
        SetSlot::<bool>::new(&[0])
            .witness_view(&limited_meter)
            .unwrap()
            .size(),
        Err(CompactError::LedgerQueryRejected(_))
    ));
}
