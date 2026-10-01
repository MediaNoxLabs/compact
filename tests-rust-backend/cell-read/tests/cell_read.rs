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

use compact_rust_cell_read_fixture::ledger_contract::{Contract, initial_state, read_flag};
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

#[test]
fn generated_cell_read_records_the_observed_value() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let written = context.write_cell(0, true).unwrap();
    let recorded = Contract::default()
        .recording
        .read_flag(written.context)
        .unwrap();
    assert!(recorded.execution.result);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
}
