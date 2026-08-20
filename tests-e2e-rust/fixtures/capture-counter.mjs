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

// TS reference capture for counter.compact.
//
// This script did not exist before the 0.33 integration: counter's
// fixture was the one *-ts-state.json in the corpus with no way to
// regenerate it, so the ledger-8 -> ledger-9 tag change
// (contract-state[v6] -> [v8]) left `tests/counter.rs` with a stale
// reference and no recipe. Adding it closes that hole.
//
// Driver sequence (mirrors what tests/counter.rs reproduces in Rust):
//   1. `await contract.initialState(ctx)` — counter.compact's implicit
//      constructor seeds `round: Counter` to 0.
//   2. `circuits.increment(ctx)` — `round.increment(1)`, so the counter
//      cell ends at 1.
//   3. Re-wrap the resulting ChargedState into the ContractState envelope
//      (operations / maintenance authority / balance are carried over from
//      the post-init state, since circuits only mutate `data`) and
//      serialize it.
//
// Usage:
//   compactc --rust --skip-zk examples/counter.compact /tmp/counter-ts-driver/
//   echo '{"type":"module"}' > /tmp/counter-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/counter-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-counter.mjs \
//     > tests-e2e-rust/fixtures/counter-ts-state.json

import { Contract, ledger } from '/tmp/counter-ts-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

// counter.compact declares no witnesses.
const contract = new Contract({});

const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

// ---- Step 1: initialState -------------------------------------------------
const initResult = await contract.initialState(constructorCtx);
const afterInitContractState = initResult.currentContractState;

// ---- Step 2: increment() --------------------------------------------------
let circuitCtx = cr.createCircuitContext(
  'constructor',
  cr.dummyContractAddress(),
  emptyCpk,
  afterInitContractState.data,
  initResult.currentPrivateState,
);
circuitCtx.callContext.circuitId = 'increment';
const incOut = await contract.circuits.increment(circuitCtx);
circuitCtx = incOut.context;

// ---- Step 3: re-wrap + serialize -----------------------------------------
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

const afterIncrementContractState = rewrapEnvelope(
  afterInitContractState,
  new cr.ChargedState(circuitCtx.callContext.currentQueryContext.state.state),
);

const fixture = {
  stateHex: Buffer.from(afterIncrementContractState.serialize()).toString('hex'),
  // `round: Counter` surfaces as a plain bigint getter on the ledger
  // accessor (see the generated index.d.ts), not as a `.read()` ADT.
  counterValue: ledger(afterIncrementContractState.data).round.toString(),
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
