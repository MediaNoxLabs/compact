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
// A21: capture TS reference state for hmt_default_fixture.compact. Three
// snapshots covering the HMT.insertIndexDefault bounded-index branch
// sequence so a divergence between the Rust circuit body (post-A21 —
// emits `lt/branch/jmp/swap/pop` plus the `rt-null value_type` leaf hash
// push) and the TS reference (always correct) surfaces as a byte-parity
// failure at the exact step that triggered it.
//
//   1. initialState()                  → empty HMT (depth 3, value_type Uint<8>)
//   2. circuits.add_default(0n)        → leaf 0 holds default hash; first_free = 1; history += root
//   3. circuits.add_default(2n)        → leaf 2 holds default hash; first_free = 3; history += root
//
// Step 3 exercises the `index + 1 > old_first_free` branch (3 > 1),
// taking the `branch 2` arm; step 2 exercises the same arm with a
// freshly-zero first_free (1 > 0). A future test with a sub-current
// index (e.g. `add_default(0)` after `add_default(2)`) would exercise
// the fallthrough `swap/pop` arm — out of scope for A21's initial
// regression net.
//
// Usage:
//   compactc --skip-zk --rust examples/hmt_default_fixture.compact /tmp/hmt-default-fixture-driver/
//   echo '{"type":"module"}' > /tmp/hmt-default-fixture-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/hmt-default-fixture-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-hmt-default-fixture.mjs \
//     > tests-e2e-rust/fixtures/hmt-default-fixture-ts-state.json

import { Contract } from '/tmp/hmt-default-fixture-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

const witnesses = {};
const contract = new Contract(witnesses);

const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

// ---- Step 1: initialState -------------------------------------------------
const initResult = await contract.initialState(constructorCtx);
const afterInitContractState = initResult.currentContractState;
const afterInitHex = Buffer.from(afterInitContractState.serialize()).toString('hex');

// Build the running CircuitContext from the post-init ContractState.
let circuitCtx = cr.createCircuitContext(
  'constructor',
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
  return new cr.ChargedState(ctx.callContext.currentQueryContext.state.state);
}

// ---- Step 2: add_default(0n) ---------------------------------------------
circuitCtx.callContext.circuitId = 'add_default';
const addDefault0Out = await contract.circuits.add_default(circuitCtx, 0n);
circuitCtx = addDefault0Out.context;
const afterAddDefault0ContractState = rewrapEnvelope(
  afterInitContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterAddDefault0Hex = Buffer.from(afterAddDefault0ContractState.serialize()).toString('hex');

// ---- Step 3: add_default(2n) ---------------------------------------------
circuitCtx.callContext.circuitId = 'add_default';
const addDefault2Out = await contract.circuits.add_default(circuitCtx, 2n);
circuitCtx = addDefault2Out.context;
const afterAddDefault2ContractState = rewrapEnvelope(
  afterAddDefault0ContractState,
  chargedStateFromCtx(circuitCtx),
);
const afterAddDefault2Hex = Buffer.from(afterAddDefault2ContractState.serialize()).toString('hex');

const fixture = {
  afterInit: { stateHex: afterInitHex },
  afterAddDefault0: { stateHex: afterAddDefault0Hex },
  afterAddDefault2: { stateHex: afterAddDefault2Hex },
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
