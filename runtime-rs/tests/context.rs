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

use midnight_compact_runtime::context::{ConstructorContext, ConstructorResult};
use midnight_compact_runtime::ledger::{ContractAddress, empty_contract_state};

#[test]
fn constructor_to_circuit_transfer_preserves_private_and_ledger_state() {
    let private = vec![1_u8, 2, 3];
    let initial = empty_contract_state();
    let constructor = ConstructorContext::new(private.clone());
    let result = ConstructorResult::new(constructor, initial.clone());
    let circuit = result.into_circuit_context(ContractAddress::default());
    assert_eq!(circuit.private_state, private);
    assert_eq!(circuit.query.state, initial);
    assert!(circuit.gas_limit.is_none());
}
