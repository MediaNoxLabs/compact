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

// Capture exact guarded_assert_arith_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node guarded_assert_arith_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits, ledger } = await import(pathToFileURL(contractIndex));
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function snapshot() {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  for (const key of initial.currentContractState.operations()) {
    state.setOperation(key, initial.currentContractState.operation(key));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return Buffer.from(state.serialize()).toString('hex');
}
function outcome(call) {
  try { call(); return { ok: true }; }
  catch (error) { return { error: error.message, compactError: error instanceof runtime.CompactError }; }
}
function attestation(createdAt) {
  return { proof: { createdAt, issuer: 1n }, hasExpiration: false, expiresAt: 0n };
}
const policy = { enforceMaxAge: true, maxAge: 20n };
const unboundedPolicy = { enforceMaxAge: false, maxAge: 20n };
const att = attestation(100n);
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const fresh = outcome(() => pureCircuits.assertFreshEnough(policy, att, 110n));
const tooOld = outcome(() => pureCircuits.assertFreshEnough(policy, att, 130n));
const future = outcome(() => pureCircuits.assertFreshEnough(policy, att, 90n));
const withoutMaxAge = outcome(() => pureCircuits.assertFreshEnough(unboundedPolicy, att, 130n));
const withinAge = outcome(() => pureCircuits.assertAgeWithin(att, 110n, 10n));
const outsideAge = outcome(() => pureCircuits.assertAgeWithin(att, 111n, 10n));
const ageGap = pureCircuits.ageGap(attestation(120n), att).toString();
const reverseGap = outcome(() => pureCircuits.ageGap(att, attestation(120n)));
context = contract.circuits.recordFreshEnough(context, policy, att, 110n).context;
const afterRecordFresh = snapshot();
const countAfterRecordFresh = ledger(context.currentQueryContext.state).accepted.toString();
const recordTooOld = outcome(() => contract.circuits.recordFreshEnough(context, policy, att, 130n));
const afterRejectedRecord = snapshot();
process.stdout.write(JSON.stringify({
  afterInit, fresh, tooOld, future, withoutMaxAge, withinAge, outsideAge, ageGap,
  reverseGap, afterRecordFresh, countAfterRecordFresh, recordTooOld, afterRejectedRecord,
}, null, 2) + '\n');
