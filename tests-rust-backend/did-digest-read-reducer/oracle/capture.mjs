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

import { readFileSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const generated = process.argv[2];
if (!generated || process.argv.length !== 3) throw Error('expected generated contract/index.js');
if (realpathSync(resolve(dirname(generated), 'node_modules/@midnight-ntwrk/compact-runtime')) !== realpathSync('runtime'))
  throw Error('generated contract must use the pinned local Compact runtime');
const contractModule = await import(pathToFileURL(resolve(generated)).href);
const sha256 = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const json = value => JSON.parse(JSON.stringify(value, (_, item) =>
  typeof item === 'bigint' ? String(item) : item instanceof Uint8Array ? Array.from(item) : item));
const key = { bytes: new Uint8Array(32) };
const stateHex = state => Buffer.from(state.serialize()).toString('hex');
const query = runtime.QueryContext.prototype.query;
let queryGas = [], witnessCalls = [];
runtime.QueryContext.prototype.query = function (...args) {
  const out = query.apply(this, args);
  queryGas.push(json(out.gasCost));
  return out;
};
const cases = [];
for (const spec of [
  { id: 'matching', methodId: 'reducer-renamed', expectedKey: 5n, admit: true },
  { id: 'missing', methodId: 'other', expectedKey: 5n, admit: true, error: 'missing' },
  { id: 'witness-denied', methodId: 'reducer-renamed', expectedKey: 5n, admit: false, error: 'witness denied' },
  { id: 'wrong-point', methodId: 'reducer-renamed', expectedKey: 6n, admit: true, error: 'wrong point' },
]) {
  queryGas = []; witnessCalls = [];
  const contract = new contractModule.Contract({ admitted: ({ privateState }) => {
    witnessCalls.push('admitted');
    return [privateState + 1, spec.admit];
  }});
  const initial = contract.initialState({
    initialPrivateState: 0,
    initialZswapLocalState: runtime.emptyZswapLocalState(key),
  }, 'reducer-renamed', runtime.ecMulGenerator(5n));
  const state = initial.currentContractState;
  const before = stateHex(state);
  const privateBefore = initial.currentPrivateState;
  queryGas = []; witnessCalls = [];
  const row = {
    id: spec.id, methodId: spec.methodId, expectedKey: String(spec.expectedKey), admit: spec.admit,
    before, privateBefore,
  };
  try {
    const result = contract.circuits.verify(
      runtime.createCircuitContext(runtime.dummyContractAddress(), key, state.data, privateBefore),
      spec.methodId, runtime.ecMulGenerator(spec.expectedKey),
    );
    state.data = new runtime.ChargedState(result.context.currentQueryContext.state.state);
    Object.assign(row, {
      after: stateHex(state), privateAfter: result.context.currentPrivateState,
      publicTranscript: json(result.proofData.publicTranscript),
      privateTranscript: result.proofData.privateTranscriptOutputs.map(value => ({
        valueAtoms: value.value.map(atom => Array.from(atom)), alignment: json(value.alignment),
      })),
    });
  } catch (error) {
    row.error = error.message;
  }
  row.queries = json(queryGas);
  row.witnessCalls = json(witnessCalls);
  if (spec.error && !row.error?.includes(spec.error)) throw Error(`${spec.id}: expected ${spec.error}, got ${row.error}`);
  if (!spec.error && row.error) throw Error(`${spec.id}: unexpected ${row.error}`);
  cases.push(row);
}
const provenance = [
  'examples/rust_backend/did_digest_read_reducer/contract.compact',
  'tests-rust-backend/did-digest-read-reducer/lib.rs',
  'tests-rust-backend/did-digest-read-reducer/oracle/capture.mjs',
  'tests-rust-backend/did-digest-read-reducer/tests/behavior.rs',
  'tests-rust-backend/did-digest-read-reducer/tests/provenance.rs',
  'runtime/src/built-ins.ts',
  'runtime/src/compact-types.ts',
].map(path => ({ path, sha256: sha256(path) }));
process.stdout.write(JSON.stringify({
  format: 'compact-did-digest-read-reducer-capture/v1',
  scope: 'Renamed and reordered String/Point product Map read with audited local Unit witness',
  generatedJavaScriptSha256: sha256(generated),
  runtimeJavaScriptSha256: sha256('runtime/dist/built-ins.js'),
  provenance, cases,
}, null, 2) + '\n');
