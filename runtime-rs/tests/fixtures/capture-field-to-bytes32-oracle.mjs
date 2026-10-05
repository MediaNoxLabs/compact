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

// Compile field_to_bytes32_oracle.compact with --target ts --skip-zk,
// install the matching Nix runtime tarball into contract/, then pass index.js.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const requireFromContract = createRequire(contractPath);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const bytesHex = (value) => Buffer.from(value).toString('hex');
const cost = (gas) => Object.fromEntries(Object.entries(gas).map(([k, v]) => [k, v.toString()]));
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: cost(result.gasCost),
    opTags: args[0].map((operation) => typeof operation === 'string' ? operation : Object.keys(operation)[0]),
  });
  return result;
};
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) return { kind: 'idx', cached: operation.idx.cached, pushPath: operation.idx.pushPath, pathLength: operation.idx.path.length };
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.dup) return { kind: 'dup', n: operation.dup.n };
  if (operation.rem) return { kind: 'rem', cached: operation.rem.cached };
  if (operation.popeq) return { kind: 'popeq', cached: operation.popeq.cached, resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)) };
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}

const modulus = BigInt('0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001');
const values = [0n, 257n, modulus - 1n];
const pure = values.map((value) => ({ input: value.toString(), resultHex: bytesHex(pureCircuits.encode(value)) }));
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({ initialPrivateState: null, initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey) });
const initialStateHex = bytesHex(initial.currentContractState.serialize());
// Rehydrate as an observed deployment would, so constructor query caches do
// not leak into the circuit's public VM program.
const deployed = runtime.ContractState.deserialize(initial.currentContractState.serialize());
const context = runtime.createCircuitContext(runtime.dummyContractAddress(), coinPublicKey, deployed.data, initial.currentPrivateState);
const queryStart = queries.length;
const output = contract.circuits.snapshot(context);
deployed.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  pure,
  initialStateHex,
  snapshot: {
    resultHex: bytesHex(output.result),
    afterStateHex: bytesHex(deployed.serialize()),
    reportedGas: cost(output.gasCost),
    queries: queries.slice(queryStart),
    publicTranscriptShape: output.proofData.publicTranscript.map(shape),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
  },
}, null, 2) + '\n');
