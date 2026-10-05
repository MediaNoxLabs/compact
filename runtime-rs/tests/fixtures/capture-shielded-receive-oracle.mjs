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

// Independent TypeScript capture of the unchanged CompactStandardLibrary
// receiveShielded helper through two differently named exported wrappers.
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const [contractPath, coraclePath] = process.argv.slice(2);
if (!contractPath || !coraclePath) {
  throw new Error('expected generated receive and original Coracle TypeScript contract paths');
}
const require = createRequire(contractPath);
const generatedSource = readFileSync(contractPath, 'utf8');
const generatedRuntimeVersion = generatedSource.match(/checkRuntimeVersion\('([^']+)'\)/)?.[1];
const runtimePackageVersion = JSON.parse(readFileSync(
  require.resolve('@midnight-ntwrk/compact-runtime/package.json'), 'utf8',
)).version;
if (!generatedRuntimeVersion || generatedRuntimeVersion !== runtimePackageVersion) {
  throw new Error('generated contract/runtime version mismatch in oracle capture');
}
const runtime = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const coinValueAlignment = runtime.ShieldedCoinInfoDescriptor.alignment().at(-1);
if (coinValueAlignment?.tag !== 'atom'
    || coinValueAlignment.value.tag !== 'bytes'
    || coinValueAlignment.value.length !== 8) {
  throw new Error('Historical ADR195 capture requires the original b8 coin descriptor; '
    + 'use runtime/test/check-shielded-receive-u128.mjs for the corrected u128 runtime');
}
const { Contract } = await import(pathToFileURL(contractPath).href);
const { Contract: CoracleContract } = await import(pathToFileURL(coraclePath).href);
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

function run(name, value, nonce, color, addressByte, start) {
  const contract = new Contract({});
  const coinPublicKey = { bytes: bytes(7) };
  const address = runtime.decodeContractAddress(bytes(addressByte));
  const initial = contract.initialState({
    initialPrivateState: [],
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const context = runtime.createCircuitContext(
    address, coinPublicKey, initial.currentContractState.data, [],
  );
  context.currentZswapLocalState.currentIndex = BigInt(start);
  const coin = { nonce: bytes(nonce), color: bytes(color), value: BigInt(value) };
  const recipient = {
    is_left: false,
    left: { bytes: bytes(0) },
    right: { bytes: bytes(addressByte) },
  };
  const before = hex(initial.currentContractState.serialize());
  queries.length = 0;
  const output = contract.circuits[name](context, coin);
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
  const commitment = contract._coinCommitment_0(coin, recipient);
  return {
    name, value: String(value), nonce, color, addressByte, start: String(start),
    before, after: hex(resultingState.serialize()),
    result: output.result, privateState: output.context.currentPrivateState,
    privateOutputs: output.proofData.privateTranscriptOutputs,
    publicTranscript: output.proofData.publicTranscript,
    queries: captured, gas: gas(output.gasCost), replayGas: gas(replay.gasCost),
    effects: output.context.currentQueryContext.effects,
    plan: output.context.currentZswapLocalState,
    commitmentHex: hex(commitment),
    comIndices: Array.from(output.context.currentQueryContext.comIndices.entries()),
  };
}

const rows = [
  run('accept', 0n, 1, 2, 9, 0),
  run('accept_renamed', 42n, 3, 2, 9, 7),
  run('accept', (1n << 64n) - 1n, 4, 5, 11, 13),
];
const overRuntimeLimit = [1n << 64n, (1n << 128n) - 1n].map((value) => {
  const contract = new Contract({});
  const coin = { nonce: bytes(4), color: bytes(5), value };
  const recipient = { is_left: false, left: { bytes: bytes(0) }, right: { bytes: bytes(11) } };
  const pureCommitmentHex = hex(contract._coinCommitment_0(coin, recipient));
  try {
    run('accept', value, 4, 5, 11, 13);
    return { value: value.toString(), rejected: false, pureCommitmentHex };
  } catch (error) {
    return { value: value.toString(), rejected: true, pureCommitmentHex,
      message: String(error.message) };
  }
});
const unusedWitness = () => { throw new Error('pure identity vectors must not call witnesses'); };
const coracle = new CoracleContract({
  local_secret_key: unusedWitness, local_board: unusedWitness,
  local_set_board: unusedWitness, fresh_nonce: unusedWitness,
});
const receive = new Contract({});
const identityVectors = [
  [1, 2, 42n, 9], [4, 5, 1n << 64n, 11],
  [4, 5, (1n << 128n) - 1n, 11], [4, 5, 42n, 12],
].map(([nonce, color, value, addressByte]) => {
  const coin = { nonce: bytes(nonce), color: bytes(color), value };
  const address = { bytes: bytes(addressByte) };
  const contractRecipient = { is_left: false, left: { bytes: bytes(0) }, right: address };
  const userRecipient = { is_left: true, left: { bytes: bytes(7) },
    right: { bytes: bytes(0) } };
  return {
    nonce, color, value: value.toString(), addressByte,
    contractCommitmentHex: hex(receive._coinCommitment_0(coin, contractRecipient)),
    userCommitmentHex: hex(receive._coinCommitment_0(coin, userRecipient)),
    contractNullifierHex: hex(coracle._coinNullifier_0(coin, address)),
  };
});
const provenance = {
  source: 'examples/rust_backend/shielded_receive_oracle.compact',
  nullifierSource: 'test-center/test-contracts/coracle.compact',
  generatedRuntimeVersion,
  runtimePackageVersion,
  runtimeCoinValueDescriptor: 'runtime/src/compact-types.ts ShieldedCoinInfoDescriptor value: MaxUint8Descriptor',
};
process.stdout.write(JSON.stringify({ provenance, rows, overRuntimeLimit, identityVectors }, encode, 2) + '\n');
