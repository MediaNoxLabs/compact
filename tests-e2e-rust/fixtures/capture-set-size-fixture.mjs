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
// A20: capture TS reference state for set_size_fixture.compact's read-no-arg
// gather chain. Three-step capture so a divergence between the Rust gather
// chain (post-A20 — uses `.size().push().eq()`) and the TS reference
// (always correct) surfaces as a byte-parity failure at the exact step
// that triggered it.
//
//   1. initialState()                  → empty Set/Map, both flags = false
//   2. circuits.check_set_empty()      → flag_set = s.isEmpty() = true
//   3. circuits.check_map_empty()      → flag_map = m.isEmpty() = true
//
// Both circuits exercise the same `(dup) (idx N) (size) (push align-0-8)
// (eq) (popeq cached)` gather chain, but on different containers (Set vs
// Map). A pre-A20 miscompile would emit `dup → idx → popeq` instead and
// fail to flip the Boolean flag (either the read crashes — the cell is
// not bool — or it captures the wrong value).
//
// Usage:
//   compactc --skip-zk --rust examples/set_size_fixture.compact /tmp/set-size-fixture-driver/
//   echo '{"type":"module"}' > /tmp/set-size-fixture-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/set-size-fixture-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-set-size-fixture.mjs \
//     > tests-e2e-rust/fixtures/set-size-fixture-ts-state.json

import { Contract } from '/tmp/set-size-fixture-driver/contract/index.js';
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

// Build the running CircuitContext from the post-init ContractState.
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

// ---- Step 2: check_set_empty() --------------------------------------------
const setEmptyOut = contract.circuits.check_set_empty(circuitCtx);
circuitCtx = setEmptyOut.context;
const afterSetEmptyContractState = rewrapEnvelope(
  afterInitContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterSetEmptyHex = Buffer.from(afterSetEmptyContractState.serialize()).toString('hex');

// ---- Step 3: check_map_empty() --------------------------------------------
const mapEmptyOut = contract.circuits.check_map_empty(circuitCtx);
circuitCtx = mapEmptyOut.context;
const afterMapEmptyContractState = rewrapEnvelope(
  afterSetEmptyContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterMapEmptyHex = Buffer.from(afterMapEmptyContractState.serialize()).toString('hex');

const fixture = {
  afterInit: { stateHex: afterInitHex },
  afterCheckSetEmpty: { stateHex: afterSetEmptyHex },
  afterCheckMapEmpty: { stateHex: afterMapEmptyHex },
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
