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
//
// The fixture's conditional expressions sit at three body routes that
// WRITE ledger state (the pure route writes nothing observable):
//   - the constructor seeds `fieldCell`/`wideCell`/`vecCell` with
//     conditional values (captured for both `c = true` and `c = false`,
//     pinning both arms of each ledger-write destination);
//   - the impure-walker route circuits (`walkerConstAnnotated`,
//     `walkerCompareEq`, `walkerCallPure`, `walkerStructMember`,
//     `walkerWrite`) each write `fieldCell` from a conditional;
//   - `streamIncrement` (impure-streaming route) reads `flag`, then
//     conditionally increments `ops` and writes `wideCell`;
//   - `streamCompareEq` (impure-streaming route) reads `flag`, then
//     conditionally writes `fieldCell`;
//   - `streamWrite` (impure-streaming route) conditionally increments
//     `ops`, then writes `fieldCell`.
//
// Decoded-value checks cannot see an alignment divergence (a `Uint<64>`
// cell written as a 1-byte atom decodes to the same small integer), so
// each step's FULL `ContractState.serialize()` bytes are captured and
// byte-compared against the Rust side in
// tests/ternary_cond_fixture.rs.
//
// Usage:
//   compactc --skip-zk examples/ternary_cond_fixture.compact /tmp/ternary-cond-ts-driver/
//   echo '{"type":"module"}' > /tmp/ternary-cond-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/ternary-cond-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-ternary-cond-fixture.mjs \
//     > tests-e2e-rust/fixtures/ternary-cond-fixture-ts-state.json

import { Contract } from '/tmp/ternary-cond-ts-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

const witnesses = {
  echoField: (ctx, x) => [ctx.privateState, x],
};
const contract = new Contract(witnesses);

const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

const CTOR_C_TRUE = true;
const CTOR_D_TRUE = true;
const CTOR_X_TRUE = 111n;
const CTOR_C_FALSE = false;
const CTOR_D_FALSE = false;
const CTOR_X_FALSE = 222n;
const WALKER_C = true;
const WALKER_X = 555n;
const COMPARE_X = 1n;
const STREAM_WRITE_C = false;
const STREAM_WRITE_X = 777n;

function hexOf(state) {
  return Buffer.from(state.serialize()).toString('hex');
}

// Runtime 0.19 (compiler 0.34): `initialState` and every circuit wrapper are
// async, `createCircuitContext` takes an options object with a leading
// `circuitId`, and per-call state lives under `callContext`. Callers stamp
// `callContext.circuitId` before each invocation.
const initTrue = await contract.initialState(
  constructorCtx,
  CTOR_C_TRUE,
  CTOR_D_TRUE,
  CTOR_X_TRUE,
);
const afterInitTrueState = initTrue.currentContractState;
const initFalse = await contract.initialState(
  constructorCtx,
  CTOR_C_FALSE,
  CTOR_D_FALSE,
  CTOR_X_FALSE,
);

const fixture = {
  afterInit: { stateHex: hexOf(afterInitTrueState) },
  afterInitFalse: { stateHex: hexOf(initFalse.currentContractState) },
};

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

// Each step is `[circuitId, runner]`; the chain starts from the `true`
// constructor state and threads the context through the steps.
async function runChain(label, steps) {
  let ctx = cr.createCircuitContext({
    circuitId: 'constructor',
    contractAddress: cr.dummyContractAddress(),
    coinPublicKeyOrZswapState: emptyCpk,
    contractState: afterInitTrueState.data,
    privateState: initTrue.currentPrivateState,
  });
  let envelope = afterInitTrueState;
  for (const [circuitId, runner] of steps) {
    ctx.callContext.circuitId = circuitId;
    const out = await runner(ctx);
    ctx = out.context;
    envelope = rewrapEnvelope(envelope, chargedStateFromCtx(ctx));
  }
  fixture[label] = { stateHex: hexOf(envelope) };
}

await runChain('afterWalkerConstAnnotated', [
  ['walkerConstAnnotated', (ctx) => contract.circuits.walkerConstAnnotated(ctx, WALKER_C)],
]);
await runChain('afterWalkerCompareEq', [
  ['walkerCompareEq', (ctx) => contract.circuits.walkerCompareEq(ctx, WALKER_C, COMPARE_X)],
]);
await runChain('afterWalkerCallPure', [
  ['walkerCallPure', (ctx) => contract.circuits.walkerCallPure(ctx, WALKER_C)],
]);
await runChain('afterWalkerStructMember', [
  ['walkerStructMember', (ctx) => contract.circuits.walkerStructMember(ctx, WALKER_C)],
]);
await runChain('afterWalkerWrite', [
  ['walkerWrite', (ctx) => contract.circuits.walkerWrite(ctx, WALKER_C, WALKER_X)],
]);
await runChain('afterStreamIncrement', [
  ['streamIncrement', (ctx) => contract.circuits.streamIncrement(ctx)],
]);
await runChain('afterStreamCompareEq', [
  ['streamCompareEq', (ctx) => contract.circuits.streamCompareEq(ctx)],
]);
await runChain('afterStreamWrite', [
  ['streamWrite', (ctx) => contract.circuits.streamWrite(ctx, STREAM_WRITE_C, STREAM_WRITE_X)],
]);

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
