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

// --- afterStreamLiteralPick: init -> streamLiteralPick(true) --------
// The streaming route's both-literal const write — same width oracle
// as afterRecordLiteralPick but through emit-streaming-body, whose
// const emitter predates the literal-arm coercion (a bare i32 if into
// new_cell fails `Into<AlignedValue>` at cargo build).
const streamLitOut = contract.circuits.streamLiteralPick(
  cr.createCircuitContext(
    cr.dummyContractAddress(),
    emptyCpk,
    afterRecordEnvelope.data,
    initResult.currentPrivateState,
  ),
  true,
);
if (streamLitOut.result.err !== undefined) {
  throw new Error(`streamLiteralPick(true) failed: ${streamLitOut.result.err}`);
}
const afterStreamLitEnvelope = rewrapEnvelope(
  afterRecordEnvelope,
  new cr.ChargedState(streamLitOut.context.currentQueryContext.state.state),
);
fixture.afterStreamLiteralPick = {
  stateHex: Buffer.from(afterStreamLitEnvelope.serialize()).toString('hex'),
};

// --- afterStreamNarrowWrite: init -> streamNarrowWrite(true, 7n) -----
// THE streaming width pin: a Uint<8> value into the Uint<64> `lastPick`
// field. The pre-fix streaming cell-write committed a 1-byte-aligned
// cell where the TS field descriptor commits 8 — same decoded value,
// divergent state bytes — so only this byte comparison catches it.
const streamNarrowOut = contract.circuits.streamNarrowWrite(
  cr.createCircuitContext(
    cr.dummyContractAddress(),
    emptyCpk,
    afterStreamLitEnvelope.data,
    initResult.currentPrivateState,
  ),
  true,
  7n,
);
if (streamNarrowOut.result.err !== undefined) {
  throw new Error(`streamNarrowWrite(true, 7n) failed: ${streamNarrowOut.result.err}`);
}
const afterStreamNarrowEnvelope = rewrapEnvelope(
  afterStreamLitEnvelope,
  new cr.ChargedState(streamNarrowOut.context.currentQueryContext.state.state),
);
fixture.afterStreamNarrowWrite = {
  stateHex: Buffer.from(afterStreamNarrowEnvelope.serialize()).toString('hex'),
};

// --- afterRecordStructPick: init -> recordStructPick(true, s, s2) ----
// Bug-12 (dogfood review, round 3): the non-Copy ternary-arm pin. The
// circuit picks between two STRUCT args and re-reads both after the
// pick (`lastPick.write(disclose(picked.value))` after two asserts); the
// Rust if-expression moves the taken arm, so only `.clone()`d arms
// compile on the Rust side. Executed here from the POST-INIT state
// (same discipline as afterRecordLiteralPick) so the Rust test can
// byte-compare the same step.
//
// NOTE: adding this impure circuit also changed `afterInit` (the TS
// initialState pre-registers every impure circuit in the operations
// map), so ALL captured steps in this file were recaptured together.
const structPickOut = contract.circuits.recordStructPick(
  cr.createCircuitContext(
    cr.dummyContractAddress(),
    emptyCpk,
    afterInitContractState.data,
    initResult.currentPrivateState,
  ),
  true,
  { low: false, value: 2n },
  { low: true, value: 3n },
);
if (structPickOut.result.err !== undefined) {
  throw new Error(`recordStructPick(true, ..) failed: ${structPickOut.result.err}`);
}
const afterStructPickEnvelope = rewrapEnvelope(
  afterInitContractState,
  new cr.ChargedState(structPickOut.context.currentQueryContext.state.state),
);
fixture.afterRecordStructPick = {
  stateHex: Buffer.from(afterStructPickEnvelope.serialize()).toString('hex'),
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
