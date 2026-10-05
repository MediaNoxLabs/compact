// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Seed through native echo calls; prior state is a local execution fixture.
use compact_rust_terminal_lexical_return_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use serde_json::{Value, json};
use std::cell::RefCell;
pub struct Witnesses {
    pub delta: u64,
    pub seed: u64,
    pub trace: RefCell<Vec<Value>>,
}
impl c::Witnesses<Vec<String>> for Witnesses {
    fn next_value(
        &self,
        ctx: WitnessContext<'_, Vec<String>, c::LedgerView<'_>>,
        seed: runtime::Field,
    ) -> (Vec<String>, runtime::Field) {
        assert_eq!(seed, runtime::Field::from(self.seed));
        let seed = self.seed.to_string();
        self.trace
            .borrow_mut()
            .push(json!({"seed":seed,"prior":ctx.private_state}));
        let mut private = ctx.private_state.clone();
        private.push(seed);
        (private, runtime::Field::from(self.delta))
    }
}
impl Witnesses {
    pub fn new(seed: u64, delta: u64) -> Self {
        Self {
            seed,
            delta,
            trace: RefCell::new(vec![]),
        }
    }
}
pub fn seeded(seed: u64) -> CircuitContext<Vec<String>> {
    let mut ctx = c::initial_state(ConstructorContext::new(Vec::<String>::new()))
        .unwrap()
        .into_circuit_context(runtime::ledger::ContractAddress::default());
    for _ in 0..seed {
        ctx = c::echo(ctx, runtime::Field::from(0)).unwrap().context;
    }
    ctx
}
