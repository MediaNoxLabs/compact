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

use compact_rust_cell_struct_fixture::ledger_contract::{initial_state, read_record, set_record};
use compact_rust_cell_struct_fixture::types::Pair;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::fab::{Aligned, Alignment, AlignmentAtom};
use midnight_compact_runtime::ledger::{
    ContractAddress, DefaultDB, StateValue, constructor_cell, read_cell,
};

#[test]
fn generated_struct_cell_has_ledger_alignment_and_round_trips() {
    assert_eq!(
        Pair::alignment(),
        Alignment::concat(
            [
                Alignment::singleton(AlignmentAtom::Field),
                Alignment::singleton(AlignmentAtom::Bytes { length: 1 }),
            ]
            .iter()
        ),
    );
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(
        read_cell::<Pair, _>(fields.get(0).unwrap()).unwrap(),
        Pair::default()
    );

    let value = Pair {
        amount: Field::from(42_u64),
        active: true,
    };
    let cell = constructor_cell::<Pair, DefaultDB>(value.clone());
    assert_eq!(read_cell::<Pair, _>(&cell).unwrap(), value);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = set_record(context, value.clone()).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_cell::<Pair, _>(fields.get(0).unwrap()).unwrap(), value);
    let read = read_record(result.context).unwrap();
    assert_eq!(read.result, value);
}
