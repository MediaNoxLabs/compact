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
    ContractAddress, DefaultDB, constructor_cell, constructor_counter, contract_state,
};
use midnight_compact_runtime::slots::{CellSlot, CounterSlot};

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
    assert_eq!(slot.witness_read(&meter).unwrap(), true);
    assert_eq!(direct_meter.read_cell::<bool>(&[0]).unwrap(), true);
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
