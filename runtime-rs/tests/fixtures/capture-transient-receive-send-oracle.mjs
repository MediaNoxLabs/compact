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

// Capture unchanged receiveShielded → sendImmediateShielded full-value calls.
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

function run(recipientKind, value, start, nonce = 1) {
  const contract = new Contract({});
  const events = [];
  for (const [method, kind] of [['_createZswapInput_0', 'input'], ['_createZswapOutput_0', 'output']]) {
    const original = contract[method];
    if (typeof original !== 'function') throw new Error(`missing generated helper ${method}`);
    contract[method] = function (...args) {
      const result = original.apply(this, args);
      events.push({ kind, coin: args[2], ...(kind === 'output' ? { recipient: args[3] } : {}) });
      return result;
    };
  }
  const coinPublicKey = { bytes: bytes(7) };
  const address = runtime.decodeContractAddress(bytes(9));
  const initial = contract.initialState({
    initialPrivateState: [],
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const context = runtime.createCircuitContext(
    address, coinPublicKey, initial.currentContractState.data, [],
  );
  context.currentZswapLocalState.currentIndex = BigInt(start);
  const coin = { nonce: bytes(nonce), color: bytes(2), value };
  const self = { bytes: bytes(9) };
  const recipient = recipientKind === 'user'
    ? { is_left: true, left: coinPublicKey, right: { bytes: bytes(0) } }
    : { is_left: false, left: { bytes: bytes(0) },
      right: recipientKind === 'self' ? self : { bytes: bytes(12) } };
  const before = hex(initial.currentContractState.serialize());
  queries.length = 0;
  try {
    const output = contract.circuits.receive_then_send(context, coin, recipient);
    const captured = [...queries];
    const replayContext = runtime.createCircuitContext(
      address, coinPublicKey, initial.currentContractState.data, [],
    );
    const replay = originalQuery.call(
      replayContext.currentQueryContext, captured.flatMap((query) => query.ops),
      replayContext.costModel, replayContext.gasLimit,
    );
    const resultingState = runtime.ContractState.deserialize(initial.currentContractState.serialize());
    resultingState.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
    const selfRecipient = { is_left: false, left: { bytes: bytes(0) }, right: self };
    const sumGas = Object.fromEntries(Object.keys(captured[0].gas).map((key) => [
      key, captured.reduce((sum, query) => sum + BigInt(query.gas[key]), 0n).toString(),
    ]));
    return {
      recipientKind, value: value.toString(), start: String(start), nonce,
      coin, recipient, before, after: hex(resultingState.serialize()),
      result: output.result,
      privateState: output.context.currentPrivateState,
      privateOutputs: output.proofData.privateTranscriptOutputs,
      publicTranscript: output.proofData.publicTranscript,
      queries: captured, wrapperLastQueryGas: gas(output.gasCost), sumQueryGas: sumGas,
      wholeProgramReplayGas: gas(replay.gasCost),
      effects: output.context.currentQueryContext.effects,
      plan: output.context.currentZswapLocalState,
      events,
      provisionalComIndices: Array.from(output.context.currentQueryContext.comIndices.entries()),
      receivedCommitmentHex: hex(contract._coinCommitment_0(coin, selfRecipient)),
      sentCommitmentHex: hex(contract._coinCommitment_0(output.result.sent, recipient)),
      transientNullifierHex: hex(contract._coinNullifier_0(coin, self)),
    };
  } catch (error) {
    return {
      recipientKind, value: value.toString(), start: String(start), nonce,
      coin, recipient, before, error: String(error.message), queries: [...queries], events,
    };
  }
}

const rows = [
  run('user', 42n, 2),
  run('contract', 42n, 2),
  run('self', 42n, 2),
  run('user', 0n, 7),
  run('user', 1n << 64n, 13),
  run('contract', (1n << 128n) - 1n, 13),
  run('user', 42n, 2, 3),
];
for (const row of rows) {
  if (row.error) throw new Error(`unexpected ${row.recipientKind} capture failure: ${row.error}`);
  if (row.result.change.is_some || row.result.sent.value !== row.coin.value
      || row.events.map((event) => event.kind).join(',') !== 'output,input,output'
      || row.plan.inputs.length !== 1 || row.plan.outputs.length !== 2
      || row.plan.inputs[0].mt_index !== 0n
      || row.plan.currentIndex !== BigInt(row.start) + 2n
      || row.privateOutputs.length !== 3) {
    throw new Error('full immediate-send shape/order mismatch');
  }
}
const provenance = {
  source: 'examples/rust_backend/transient_receive_send_oracle.compact',
  standardLibraryHelpers: ['compiler/standard-library.compact:155', 'compiler/standard-library.compact:199'],
  generatedContractSha256: createHash('sha256').update(generatedSource).digest('hex'),
  generatedRuntimeVersion, runtimePackageName: runtimePackage.name, runtimePackageVersion,
  runtimeTypesSha256, runtimeValueBytes: descriptorAlignment.at(-1).value.length,
  allocationPolicy: 'raw TypeScript provisional source order; not normalized offer allocation',
  gasPolicy: 'query sum, wrapper last-query cost and whole-program replay retained separately',
  intentOrderObservation: 'successful generated _createZswapInput_0/_createZswapOutput_0 calls; original helpers executed unchanged',
};
process.stdout.write(JSON.stringify({ provenance, rows }, encode, 2) + '\n');
