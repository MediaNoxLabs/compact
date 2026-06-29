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

// SPDX-License-Identifier: Apache-2.0
//
// Bug-11 byte-parity capture: drives bug11_fixture.compact's
// initial_state() and each of the three set_* write circuits through the
// TS-generated contract and emits the serialised ContractState after
// every step. The Rust test (tests/bug11_fixture.rs) replays the same
// sequence and asserts byte-parity.
//
// The fixture exists to lock down the Rust codegen's treatment of
// `Uint<L..U>` ledger writes with non-power-of-2 byte_len. Pre-Bug-11
// the Rust walker refused to emit any non-literal write into such a
// field; post-fix the cell-builder routes through
// `new_cell_bounded_uint(value as u128, byte_len)` matching TS's
// `CompactTypeUnsignedInteger(maxValue, byte_len).toValue(v)` shape.
//
// Usage (run from the repo root):
//   ./result/bin/compactc --skip-zk examples/bug11_fixture.compact \
//     /tmp/bug11-ts-driver/
//   echo '{"type":"module"}' > /tmp/bug11-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/bug11-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-bug11-fixture.mjs \
//     > tests-e2e-rust/fixtures/bug11-fixture-ts-state.json

import { Contract } from '/tmp/bug11-ts-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

const witnesses = {};
const contract = new Contract(witnesses);

const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

// ---- Step 1: initialState -------------------------------------------------
const initResult = contract.initialState(constructorCtx);
const afterInitContractState = initResult.currentContractState;
const afterInitHex = Buffer.from(afterInitContractState.serialize()).toString('hex');

// Build a CircuitContext from the post-init state so we can call set_*.
let circuitCtx = cr.createCircuitContext(
  cr.dummyContractAddress(),
  emptyCpk,
  afterInitContractState.data,
  initResult.currentPrivateState,
);

function rewrapEnvelope(prev, newChargedState) {
  const next = new cr.ContractState();
  next.data = newChargedState;
  for (const opKey of prev.operations()) {
    next.setOperation(opKey, prev.operation(opKey));
  }
  next.maintenanceAuthority = prev.maintenanceAuthority;
  next.balance = prev.balance;
  return next;
}

function chargedStateFromCtx(ctx) {
  return new cr.ChargedState(ctx.currentQueryContext.state.state);
}

// ---- Step 2: set_tiny(42) — 1-byte field, exercises the new_cell path ----
const setTinyOut = contract.circuits.set_tiny(circuitCtx, 42n);
circuitCtx = setTinyOut.context;
const afterSetTinyContractState = rewrapEnvelope(
  afterInitContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterSetTinyHex = Buffer.from(afterSetTinyContractState.serialize()).toString('hex');

// ---- Step 3: set_medium(65000) — 3-byte field, exercises bounded-uint path
const setMediumOut = contract.circuits.set_medium(circuitCtx, 65000n);
circuitCtx = setMediumOut.context;
const afterSetMediumContractState = rewrapEnvelope(
  afterSetTinyContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterSetMediumHex = Buffer.from(afterSetMediumContractState.serialize()).toString('hex');

// ---- Step 4: set_wide(4_500_000_000) — 5-byte field, bounded-uint path ----
const setWideOut = contract.circuits.set_wide(circuitCtx, 4500000000n);
circuitCtx = setWideOut.context;
const afterSetWideContractState = rewrapEnvelope(
  afterSetMediumContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterSetWideHex = Buffer.from(afterSetWideContractState.serialize()).toString('hex');

const fixture = {
  afterInit: { stateHex: afterInitHex },
  afterSetTiny: { stateHex: afterSetTinyHex },
  afterSetMedium: { stateHex: afterSetMediumHex },
  afterSetWide: { stateHex: afterSetWideHex },
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
