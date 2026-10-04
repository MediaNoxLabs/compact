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

// Compile set_size_oracle.compact or nested_collection_query_write.compact
// with --target ts --skip-zk, link its generated contract to this runtime,
// then pass contract/index.js and set-size|nested as arguments.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, mode] = process.argv.slice(2);
if (!contractPath || !['set-size', 'nested'].includes(mode)) {
  throw new Error('expected contract/index.js and set-size|nested');
}
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()]),
    ),
    opTags: args[0].map((operation) =>
      typeof operation === 'string' ? operation : Object.keys(operation)[0]),
  });
  return result;
};

function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.rem) return { kind: 'rem', cached: operation.rem.cached };
  if (operation.dup) return { kind: 'dup', n: operation.dup.n };
  if (operation.popeq) {
    return {
      kind: 'popeq', cached: operation.popeq.cached,
      resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
    };
  }
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}

function run(seeded = false) {
  const initial = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  let context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  if (seeded) {
    context = contract.circuits.seed(context, 42n).context;
  }
  if (mode === 'nested') {
    context = contract.circuits.check_member(context, 42n).context;
  }
  function capture(name) {
    const start = queries.length;
    const output = contract.circuits[name](context);
    context = output.context;
    return {
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
      queries: queries.slice(start),
      reportedGas: Object.fromEntries(
        Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
      ),
    };
  }
  return {
    set: capture('check_set_empty'),
    map: capture('check_map_empty'),
  };
}

process.stdout.write(JSON.stringify({ empty: run(), ...(mode === 'nested' ? { seeded: run(true) } : {}) }, null, 2) + '\n');
