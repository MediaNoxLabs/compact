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

use compact_contract_witness_list_shapes::ledger_contract::{
    LedgerView, TryWitnesses, initial_state, push_choice, push_count, push_flag, push_packet,
    push_tag, read_choices, read_counts, read_flags, read_packets, read_tags,
};
use compact_contract_witness_list_shapes::runtime::context::{ConstructorContext, WitnessContext};
use compact_contract_witness_list_shapes::runtime::ledger::ContractAddress;
use compact_contract_witness_list_shapes::runtime::{BoundedUint, CompactError, FixedBytes};
use compact_contract_witness_list_shapes::types::{Choice, Packet};

struct Inspect;

macro_rules! method {
    ($name:ident, $field:ident) => {
        fn $name(
            &self,
            context: WitnessContext<'_, (), LedgerView<'_>>,
        ) -> Result<((), bool), CompactError> {
            let list = context.ledger.$field()?;
            let head = list.head()?;
            assert_eq!(head.is_none(), list.is_empty()?);
            assert_eq!(u128::from(head.is_some()), list.length()?.value());
            Ok(((), head.is_some()))
        }
    };
}

impl TryWitnesses<()> for Inspect {
    method!(inspect_flags, flags);
    method!(inspect_counts, counts);
    method!(inspect_tags, tags);
    method!(inspect_choices, choices);
    method!(inspect_packets, packets);
}

#[test]
fn packaged_crate_exposes_all_typed_list_witnesses() {
    let mut context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let empty = read_flags(context, &Inspect).unwrap();
    assert!(!empty.result && empty.gas_cost.read_time > Default::default());
    context = empty.context;
    context = push_flag(context, true).unwrap().context;
    context = push_count(context, BoundedUint::<65535>::new(42).unwrap())
        .unwrap()
        .context;
    context = push_tag(context, FixedBytes::new([1, 2, 3]))
        .unwrap()
        .context;
    context = push_choice(context, Choice::no).unwrap().context;
    context = push_packet(
        context,
        Packet {
            tag: FixedBytes::new([4, 5, 6]),
            count: BoundedUint::<65535>::new(7).unwrap(),
        },
    )
    .unwrap()
    .context;
    let flags = read_flags(context, &Inspect).unwrap();
    assert!(flags.result);
    let counts = read_counts(flags.context, &Inspect).unwrap();
    assert!(counts.result);
    let tags = read_tags(counts.context, &Inspect).unwrap();
    assert!(tags.result);
    let choices = read_choices(tags.context, &Inspect).unwrap();
    assert!(choices.result);
    let packets = read_packets(choices.context, &Inspect).unwrap();
    assert!(packets.result);
}
