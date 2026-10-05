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

// Capture unchanged qualified merge and receive → immediate merge helpers.
// The raw runtime uses provisional source-order indices; it is deliberately not
// rewritten to represent a normalized offer's authoritative allocation.
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { readFileSync, realpathSync } from 'node:fs';
import { sep } from 'node:path';
import { pathToFileURL } from 'node:url';

const [contractPath, runtimeRoot] = process.argv.slice(2);
if (!contractPath || !runtimeRoot) {
  throw new Error('expected generated TypeScript contract and corrected runtime root');
}
const require = createRequire(contractPath);
const resolvedRuntime = realpathSync(require.resolve('@midnight-ntwrk/compact-runtime'));
const actualRuntimeRoot = realpathSync(runtimeRoot);
if (!resolvedRuntime.startsWith(actualRuntimeRoot + sep)) {
  throw new Error('generated bundle did not resolve the requested runtime package');
}
const runtime = await import(pathToFileURL(resolvedRuntime).href);
const { Contract } = await import(pathToFileURL(contractPath).href);
const generatedSource = readFileSync(contractPath, 'utf8');
const generatedRuntimeVersion = generatedSource.match(/checkRuntimeVersion\('([^']+)'\)/)?.[1];
const runtimePackage = JSON.parse(readFileSync(
  `${actualRuntimeRoot}/package.json`, 'utf8',
));
const runtimePackageVersion = runtimePackage.version;
const runtimeTypesSha256 = createHash('sha256')
  .update(readFileSync(`${actualRuntimeRoot}/dist/compact-types.js`)).digest('hex');
const descriptorAlignment = runtime.ShieldedCoinInfoDescriptor.alignment();
if (generatedRuntimeVersion !== runtimePackageVersion
    || descriptorAlignment.at(-1)?.value?.length !== 16) {
  throw new Error('expected matched runtime with 16-byte ShieldedCoinInfo value');
}

const bytes = (first) => Uint8Array.from([first, ...Array(31).fill(0)]);
const hex = (value) => Buffer.from(value).toString('hex');
const gas = (value) => Object.fromEntries(
  Object.entries(value).map(([name, cost]) => [name, cost.toString()]),
);
const encode = (_name, value) => value instanceof Uint8Array ? Array.from(value)
  : typeof value === 'bigint' ? value.toString()
    : value instanceof Map ? Object.fromEntries(value) : value;
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ ops: args[0], gas: gas(result.gasCost) });
  return result;
};

function run(name, aValue, bValue, { color = 2, duplicate = false, swap = false } = {}) {
  const contract = new Contract({});
  const events = [];
  for (const [method, kind] of [['_createZswapInput_0', 'input'], ['_createZswapOutput_0', 'output']]) {
    const original = contract[method];
    if (typeof original !== 'function') throw new Error(`missing generated helper ${method}`);
    contract[method] = function (...args) {
      const result = original.apply(this, args);
      events.push({ kind, coin: args[2] });
      return result;
    };
  }
  const key = { bytes: bytes(7) };
  const address = runtime.decodeContractAddress(bytes(9));
  const initial = contract.initialState({
    initialPrivateState: [],
    initialZswapLocalState: runtime.emptyZswapLocalState(key),
  });
  const context = runtime.createCircuitContext(address, key, initial.currentContractState.data, []);
  context.currentZswapLocalState.currentIndex = 2n;
  let a = { nonce: bytes(1), color: bytes(2), value: aValue, mt_index: 0n };
  let b = { nonce: bytes(duplicate ? 1 : 3), color: bytes(color), value: bValue, mt_index: 1n };
  if (swap) [a, b] = [b, a];
  if (name === 'receive_then_merge') {
    const { mt_index, ...coin } = b;
    b = coin;
  }
  queries.length = 0;
  try {
    const before = hex(initial.currentContractState.serialize());
    const result = contract.circuits[name](context, a, b);
    const captured = [...queries];
    const replayContext = runtime.createCircuitContext(address, key, initial.currentContractState.data, []);
    const replay = originalQuery.call(
      replayContext.currentQueryContext, captured.flatMap((query) => query.ops),
      replayContext.costModel, replayContext.gasLimit,
    );
    const state = runtime.ContractState.deserialize(initial.currentContractState.serialize());
    state.data = new runtime.ChargedState(result.context.currentQueryContext.state.state);
    const sumQueryGas = Object.fromEntries(Object.keys(captured[0].gas).map((dimension) => [
      dimension, captured.reduce((sum, query) => sum + BigInt(query.gas[dimension]), 0n).toString(),
    ]));
    return {
      name, a, b, swap, before, after: hex(state.serialize()), result: result.result,
      queries: captured, events, publicTranscript: result.proofData.publicTranscript,
      privateOutputs: result.proofData.privateTranscriptOutputs,
      plan: result.context.currentZswapLocalState,
      effects: result.context.currentQueryContext.effects,
      wrapperLastQueryGas: gas(result.gasCost), sumQueryGas,
      wholeProgramReplayGas: gas(replay.gasCost),
    };
  } catch (error) {
    // No execution context/private transcript is returned on failure. Retain
    // only the successful query and intent effects observed before rejection.
    return { name, a, b, swap, error: String(error.message), queries: [...queries], events };
  }
}

const max = (1n << 128n) - 1n;
const rows = [];
for (const name of ['merge_qualified', 'receive_then_merge']) {
  rows.push(
    run(name, 17n, 25n),
    run(name, 17n, 25n, { swap: true }),
    run(name, 0n, 0n),
    run(name, max - 1n, 1n),
    run(name, 1n << 64n, 1n),
    run(name, max, 1n),
    run(name, max, max),
    run(name, max, 1n, { color: 4 }),
    run(name, 17n, 25n, { color: 4 }),
    run(name, 17n, 17n, { duplicate: true }),
  );
}
for (const row of rows) {
  const wrongColor = hex(row.a.color) !== hex(row.b.color);
  const overflow = row.a.value + row.b.value > max;
  const immediate = row.name === 'receive_then_merge';
  if (wrongColor || overflow) {
    const expected = wrongColor ? 'Can only merge coins of the same color'
      : 'cast from Field or Uint value to smaller Uint value failed';
    if (!row.error?.includes(expected)) throw new Error(`wrong rejection precedence: ${row.error}`);
    if (row.queries.length !== (immediate ? 5 : 3)) throw new Error('wrong rejected query prefix');
  } else {
    if (row.error) throw new Error(`unexpected failure: ${row.error}`);
    if (row.result.value !== row.a.value + row.b.value
        || row.privateOutputs.length !== (immediate ? 4 : 3)
        || row.queries.length !== (immediate ? 7 : 5)
        || row.plan.inputs.length !== 2
        || row.plan.outputs.length !== (immediate ? 2 : 1)) {
      throw new Error('unexpected successful merge shape');
    }
  }
}
const provenance = {
  source: 'examples/rust_backend/shielded_merge_oracle.compact',
  generatedRuntimeVersion, runtimePackageVersion, runtimeTypesSha256,
  contractSha256: createHash('sha256').update(generatedSource).digest('hex'),
  allocationPolicy: 'raw TS provisional source order; authoritative offer allocation tested separately',
  gasPolicy: 'sum, wrapper-last and replay costs retained separately',
  errorPolicy: 'only successful query and intent prefixes are observable after rejection',
};
process.stdout.write(JSON.stringify({ provenance, rows }, encode, 2) + '\n');
