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

// Compile observed_composite_keys.compact with --target ts --skip-zk, link
// contract/node_modules/@midnight-ntwrk/compact-runtime to this runtime,
// then pass the generated contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});

function capture(name, ...inputs) {
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const output = contract.circuits[name](context, ...inputs);
  const proofData = output.proofData;
  const publicTranscriptShape = proofData.publicTranscript.map((operation) => {
    if (typeof operation === 'string') return { kind: operation };
    if (operation.idx) {
      const { cached, pushPath, path } = operation.idx;
      return { kind: 'idx', cached, pushPath, pathLength: path.length };
    }
    if (operation.push) return { kind: 'push', storage: operation.push.storage };
    if (operation.ins) {
      const { cached, n } = operation.ins;
      return { kind: 'ins', cached, n };
    }
    if (operation.rem) return { kind: 'rem', cached: operation.rem.cached };
    if (operation.dup) return { kind: 'dup', n: operation.dup.n };
    if (operation.popeq) {
      const { cached, result } = operation.popeq;
      return {
        kind: 'popeq', cached,
        resultAtoms: result.value.map((atom) => Array.from(atom)),
      };
    }
    throw new Error(`unexpected ${name} operation: ${Object.keys(operation)}`);
  });
  return {
    result: output.result,
    valueAtoms: proofData.input.value.map((atom) => Array.from(atom)),
    alignment: proofData.input.alignment,
    publicTranscriptShape,
  };
}

process.stdout.write(JSON.stringify({
  vector: capture('insert_vector', [3n, 5n]),
  tuple: capture('insert_tuple', [42n, true]),
  struct: capture('insert_struct', { vector: [3n, 5n], pair: [42n, true] }),
  tupleRoundtrip: capture('roundtrip_tuple', [42n, true]),
  structRoundtrip: capture('roundtrip_struct', { vector: [3n, 5n], pair: [42n, true] }),
  twelve: capture('record_twelve', 1n, 2n, 3n, 4n, 5n, 6n, 7n, 8n, 9n, 10n, 11n, 12n),
}, null, 2) + '\n');
