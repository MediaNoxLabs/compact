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

// Capture the TS backend's state for call_arg_declared_type_fixture.compact:
// the constructor, then each exported circuit run from a fresh post-init
// context. Every circuit writes its result to the ledger, so the Rust side
// (tests/call_arg_declared_type.rs) compares FULL `ContractState` bytes per
// circuit — a call argument rendered at the wrong Rust type changes the
// value written (e.g. a commitment to an `i32` instead of a `Field`) even
// when the generated crate builds.
//
// Usage:
//   compactc --skip-zk examples/call_arg_declared_type_fixture.compact /tmp/call-arg-declared-type-ts-driver/
//   echo '{"type":"module"}' > /tmp/call-arg-declared-type-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/call-arg-declared-type-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-call-arg-declared-type-fixture.mjs \
//     > tests-e2e-rust/fixtures/call-arg-declared-type-fixture-ts-state.json
//
// `CAPTURE_DRIVER_DIR` overrides the driver directory.

import * as cr from '@midnight-ntwrk/compact-runtime';

const driverDir = process.env.CAPTURE_DRIVER_DIR ?? '/tmp/call-arg-declared-type-ts-driver';
const { Contract } = await import(`${driverDir}/contract/index.js`);

// Witnesses, mirrored by `FixtureWitnesses` in tests/call_arg_declared_type.rs.
// `sumWitness` returns `v[0] + v[1]`, so the argument's value reaches the
// ledger (the sum stays far below the Field modulus).
const witnesses = {
  sumWitness: (ctx, v) => [ctx.privateState, v[0] + v[1]],
};

// Exported circuits, each run once from the post-init state.
const CIRCUITS = [
  'commitSmall',
  'commitU128',
  'commitFieldOnly',
  'pureBodyVec',
  'pureBodyFieldOnly',
  'bridgeTupleIntoVec',
  'bridgeVecIntoTuple',
  'witnessConst',
  'witnessBare',
  'pureFromImpure',
  'impureConst',
  'impureBare',
  'impureInIfArm',
  'inlinedAssert',
  'hashPersistentVec',
  'hashTransientVec',
];

const contract = new Contract(witnesses);
const emptyCpk = { bytes: new Uint8Array(32) };
const constructorCtx = {
  initialPrivateState: null,
  initialZswapLocalState: cr.emptyZswapLocalState(emptyCpk),
};

function hexOf(state) {
  return Buffer.from(state.serialize()).toString('hex');
}

// The circuits move the ChargedState forward; the ContractState envelope's
// operations / authority / balance carry over unchanged (as in
// capture-ternary-cond-fixture.mjs).
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

const init = contract.initialState(constructorCtx);
const afterInit = init.currentContractState;
const fixture = { afterInit: { stateHex: hexOf(afterInit) }, circuits: {} };

for (const name of CIRCUITS) {
  const ctx = cr.createCircuitContext(
    cr.dummyContractAddress(),
    emptyCpk,
    afterInit.data,
    init.currentPrivateState,
  );
  const out = contract.circuits[name](ctx);
  const state = new cr.ChargedState(out.context.currentQueryContext.state.state);
  fixture.circuits[name] = { stateHex: hexOf(rewrapEnvelope(afterInit, state)) };
}

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
