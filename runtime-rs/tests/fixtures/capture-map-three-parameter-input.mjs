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

// Compile examples/rust_backend/map_boolean_field.compact for TypeScript,
// link its contract/node_modules/@midnight-ntwrk/compact-runtime to this
// branch's runtime, then pass the generated contract/index.js as argument.
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
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const proofData = contract.circuits.put_pair(context, true, false, 42n).proofData;
const publicTranscriptShape = proofData.publicTranscript.map((operation) => {
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) {
    const { cached, n } = operation.ins;
    return { kind: 'ins', cached, n };
  }
  throw new Error(`unexpected operation in Map put_pair: ${Object.keys(operation)}`);
});
process.stdout.write(JSON.stringify({
  valueAtoms: proofData.input.value.map((atom) => Array.from(atom)),
  alignment: proofData.input.alignment,
  publicTranscriptShape,
}, null, 2) + '\n');
