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

// Capture unchanged CompactStandardLibrary.sendShielded through four exported
// wrappers. The caller must supply an independently built, corrected-u128
// runtime package; the historical b8 package belongs to ADR-0195 evidence.
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

function run(name, sentValue, recipientByte, start, inputValue = 42n) {
  const contract = new Contract({});
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
  const input = { nonce: bytes(1), color: bytes(2), value: inputValue, mt_index: 0n };
  const self = { bytes: bytes(9) };
  const user = { bytes: bytes(7) };
  const foreign = { bytes: bytes(recipientByte) };
  const recipient = name === 'send_to_user'
    ? { is_left: true, left: user, right: { bytes: bytes(0) } }
    : { is_left: false, left: { bytes: bytes(0) },
      right: name === 'send_to_self' ? self : foreign };
  const args = name === 'send_to_self' ? [input, sentValue]
    : name === 'send_to_user' ? [input, user, sentValue]
      : name === 'send_to_contract' ? [input, foreign, sentValue]
        : [input, recipient, sentValue];
  const before = hex(initial.currentContractState.serialize());
  queries.length = 0;
  try {
    const output = contract.circuits[name](context, ...args);
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
    const sentCommitment = contract._coinCommitment_0(output.result.sent, recipient);
    const changeRecipient = { is_left: false, left: { bytes: bytes(0) }, right: self };
    const changeCommitment = output.result.change.is_some
      ? contract._coinCommitment_0(output.result.change.value, changeRecipient) : null;
    return {
      name, sentValue: sentValue.toString(), inputValue: inputValue.toString(), recipientByte,
      start: String(start), recipient, input, before,
      after: hex(resultingState.serialize()), result: output.result,
      privateState: output.context.currentPrivateState,
      privateOutputs: output.proofData.privateTranscriptOutputs,
      publicTranscript: output.proofData.publicTranscript,
      queries: captured, gas: gas(output.gasCost), replayGas: gas(replay.gasCost),
      effects: output.context.currentQueryContext.effects,
      plan: output.context.currentZswapLocalState,
      comIndices: Array.from(output.context.currentQueryContext.comIndices.entries()),
      sentCommitmentHex: hex(sentCommitment),
      changeCommitmentHex: changeCommitment ? hex(changeCommitment) : null,
      nullifierHex: hex(contract._coinNullifier_0(
        { nonce: input.nonce, color: input.color, value: input.value }, self,
      )),
    };
  } catch (error) {
    return {
      name, sentValue: sentValue.toString(), inputValue: inputValue.toString(), recipientByte,
      start: String(start), recipient, input, before,
      error: String(error.message), queries: [...queries],
    };
  }
}

const rows = [
  run('send_to_self', 42n, 9, 1),
  run('send_to_self', 17n, 9, 1),
  run('send_to_user', 42n, 7, 1),
  run('send_to_user', 17n, 7, 1),
  run('send_to_contract', 42n, 12, 1),
  run('send_to_contract', 17n, 12, 1),
  run('send_forward', 42n, 12, 1),
  run('send_to_self', 43n, 9, 1),
  run('send_to_user', 43n, 7, 1),
  run('send_to_self', 1n << 64n, 9, 1, 1n << 64n),
];
const provenance = {
  source: 'examples/rust_backend/shielded_send_oracle.compact',
  standardLibraryHelper: 'compiler/standard-library.compact:161',
  generatedRuntimeVersion,
  runtimePackageName: runtimePackage.name,
  runtimePackageVersion,
  runtimeTypesSha256,
  runtimeValueBytes: descriptorAlignment.at(-1).value.length,
};
process.stdout.write(JSON.stringify({ provenance, rows }, encode, 2) + '\n');
