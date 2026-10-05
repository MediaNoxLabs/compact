// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Compile qualified_coin_set_oracle.compact with --target ts --skip-zk and
// install the matching compact-runtime package in contract/node_modules.
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected generated contract/index.js');
const packagePath = join(dirname(contractPath), 'node_modules/@midnight-ntwrk/compact-runtime/dist/index.js');
const runtime = await import(pathToFileURL(packagePath).href);
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const address = runtime.dummyContractAddress();
const addressBytes = runtime.encodeContractAddress(address);
const coinPublicKey = { bytes: new Uint8Array(32) };
const coin = {
  nonce: Uint8Array.from([110, 111, 110, 99, 101, ...Array(27).fill(0)]),
  color: Uint8Array.from([99, 111, 108, 111, 114, ...Array(27).fill(0)]),
  value: 42n,
};
const right = {
  is_left: false,
  left: { bytes: new Uint8Array(32) },
  right: { bytes: addressBytes },
};
const left = {
  is_left: true,
  left: { bytes: Uint8Array.from([7, ...Array(31).fill(0)]) },
  right: { bytes: new Uint8Array(32) },
};
const queryLog = [];
const query = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = query.call(this, ...args);
  queryLog.push({
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
    gasCost: Object.fromEntries(Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()])),
  });
  return result;
};
const gas = (cost) => Object.fromEntries(Object.entries(cost).map(([key, value]) => [key, value.toString()]));
const hex = (state) => Buffer.from(state.serialize()).toString('hex');
function run(recipient, index, allocate) {
  const deployed = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const initialStateHex = hex(deployed.currentContractState);
  const context = runtime.createCircuitContext(
    address, coinPublicKey, deployed.currentContractState.data, deployed.currentPrivateState,
  );
  if (allocate) {
    context.currentZswapLocalState.currentIndex = BigInt(index);
    runtime.createZswapOutput(context, coin, recipient);
  }
  const before = queryLog.length;
  try {
    const inserted = contract.circuits.insert_coin(context, coin, recipient);
    const afterInsert = queryLog.length;
    const qualified = { ...coin, mt_index: BigInt(index) };
    const member = contract.circuits.contains(inserted.context, qualified);
    deployed.currentContractState.data = new runtime.ChargedState(
      inserted.context.currentQueryContext.state.state,
    );
    return {
      index, allocate, initialStateHex,
      afterStateHex: hex(deployed.currentContractState),
      effects: inserted.context.currentQueryContext.effects,
      insertedResult: String(inserted.result), member: member.result,
      insertGas: gas(inserted.gasCost), memberGas: gas(member.gasCost),
      insertQueries: queryLog.slice(before, afterInsert),
      memberQueries: queryLog.slice(afterInsert),
      privateTranscriptCount: inserted.proofData.privateTranscriptOutputs.length,
    };
  } catch (error) {
    return { index, allocate, initialStateHex, error: String(error), queries: queryLog.slice(before) };
  }
}
process.stdout.write(JSON.stringify({
  right7: run(right, 7, true),
  left11: run(left, 11, true),
  missing: run(right, 7, false),
}, null, 2) + '\n');
