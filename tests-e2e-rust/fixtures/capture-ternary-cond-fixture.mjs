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
// Capture TS reference state for ternary_cond_fixture.compact.
// Three ledger fields (`lastPick: Uint<64>`, `picks: Counter`,
// `origin: Field`), eight exported pure circuits covering conditional
// expressions in every sub-expression position, four exported impure
// circuits (`recordPick`, `recordLiteralPick`, `recordFieldPick`,
// `recordFieldBranches`), and a constructor whose body evaluates a
// const ternary with a guarded subtraction and writes the result to
// `lastPick` — the state captured here pins the constructor route
// against the Rust side (tests/ternary_cond_fixture.rs).
//
// The constructor argument is 20n: 20 > 15, so the ternary's THEN
// branch computes 20 - 10 = 10 and `lastPick` initialises to 10.
// (CAPTURE_START in tests/ternary_cond_fixture.rs must stay in sync.)
//
// `afterRecordLiteralPick` additionally EXECUTES
// `recordLiteralPick(true)` from the post-init state — the dogfood
// review found the Rust backend committing a 1-byte-aligned cell for
// `lastPick` where the TS field descriptor commits 8 bytes (same
// decoded value, divergent state bytes); the decoded-value test alone
// cannot see that, so the post-circuit state bytes are captured here
// and byte-compared on the Rust side
// (record_literal_pick_byte_parity). Both drivers execute the same
// circuit from the same post-init state, so the comparison pins the
// WRITE path's serialization, not just the constructor's.
//
// Usage:
//   compactc --target ts --skip-zk examples/ternary_cond_fixture.compact /tmp/ternary-cond-ts-driver/
//   echo '{"type":"module"}' > /tmp/ternary-cond-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/ternary-cond-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-ternary-cond-fixture.mjs \
//     > tests-e2e-rust/fixtures/ternary-cond-fixture-ts-state.json

import { Contract } from '/tmp/ternary-cond-ts-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

const witnesses = {};
const contract = new Contract(witnesses);

const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

const initResult = contract.initialState(constructorCtx, 20n);
const afterInitContractState = initResult.currentContractState;

const afterInitHex = Buffer.from(afterInitContractState.serialize()).toString('hex');

const fixture = {
  afterInit: { stateHex: afterInitHex },
};

// --- afterRecordLiteralPick: init -> recordLiteralPick(true) ----------
// Same envelope-rewrapping discipline as capture-election.mjs: the
// circuit returns a fresh CircuitContext; rebuild the ContractState
// envelope around its ChargedState (copying the operations map the TS
// initialState() pre-registered) and serialise that.
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

const circuitCtx = cr.createCircuitContext(
  cr.dummyContractAddress(),
  emptyCpk,
  afterInitContractState.data,
  initResult.currentPrivateState,
);
const recordOut = contract.circuits.recordLiteralPick(circuitCtx, true);
if (recordOut.result.err !== undefined) {
  throw new Error(`recordLiteralPick(true) failed: ${recordOut.result.err}`);
}
const afterRecordEnvelope = rewrapEnvelope(
  afterInitContractState,
  new cr.ChargedState(recordOut.context.currentQueryContext.state.state),
);
fixture.afterRecordLiteralPick = {
  stateHex: Buffer.from(afterRecordEnvelope.serialize()).toString('hex'),
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
