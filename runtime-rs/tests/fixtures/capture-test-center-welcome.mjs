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

// Compile the exact test-center/test-contracts/welcome.compact with --target ts
// --skip-zk, link to the matching compact-runtime, then pass both index.js paths.
import { pathToFileURL } from 'node:url';

const [contractPath, runtimePath] = process.argv.slice(2);
if (!contractPath || !runtimePath) throw new Error('expected contract and runtime paths');
const runtime = await import(pathToFileURL(runtimePath).href);
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);

const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    opTags: args[0].map((operation) => Object.keys(operation)[0]),
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
  });
  return result;
};

const witnessCalls = [];
const checkInWitnessCalls = [];
let localSkMode = 'zero';
const contract = new Contract({
  local_sk: ({ privateState }) => {
    witnessCalls.push(privateState);
    return [privateState, {
      is_some: localSkMode !== 'missing',
      value: new Uint8Array(32).fill(localSkMode === 'other' ? 1 : 0),
    }];
  },
  set_local_id: ({ privateState }, participant) => {
    checkInWitnessCalls.push({ privateState, participant });
    return [privateState + 1, []];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const stateHex = (state) => Buffer.from(state.serialize()).toString('hex');
function stateAfter(initial, queryState) {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(queryState);
  for (const entry of initial.currentContractState.operations()) {
    state.setOperation(entry, initial.currentContractState.operation(entry));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return stateHex(state);
}
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  const [kind, details] = Object.entries(operation)[0];
  if (kind === 'idx') return { kind, cached: details.cached, pushPath: details.pushPath, pathLength: details.path.length };
  if (kind === 'push') return { kind, storage: details.storage };
  if (kind === 'ins') return { kind, cached: details.cached, n: details.n };
  if (kind === 'popeq') return { kind, cached: details.cached, resultAtoms: details.result.value.map((atom) => Array.from(atom)) };
  if (kind === 'branch' || kind === 'jmp') return { kind, skip: details.skip };
  if (kind === 'swap' || kind === 'dup' || kind === 'concat') return { kind, n: details.n, cached: details.cached };
  throw new Error(`unexpected check-in VM operation: ${kind}`);
}
function captureCheckIn(initial, participant) {
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const before = stateHex(initial.currentContractState);
  const queryStart = queries.length;
  const witnessStart = checkInWitnessCalls.length;
  try {
    const output = contract.circuits.check_in(context, participant);
    return {
      success: true,
      participant,
      initialStateHex: before,
      afterStateHex: stateAfter(initial, output.context.currentQueryContext.state.state),
      privateState: output.context.currentPrivateState,
      result: output.result,
      gasCost: Object.fromEntries(Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()])),
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
      privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      })),
      queries: queries.slice(queryStart),
      witnessCalls: checkInWitnessCalls.slice(witnessStart),
    };
  } catch (error) {
    return {
      success: false,
      participant,
      initialStateHex: before,
      afterStateHex: stateAfter(initial, context.currentQueryContext.state.state),
      privateState: context.currentPrivateState,
      error: error.message,
      compactError: error.constructor.name === 'CompactError',
      queries: queries.slice(queryStart),
      witnessCalls: checkInWitnessCalls.slice(witnessStart),
    };
  }
}
function captureOrganizerCall(initial, name, argument, mode) {
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const before = stateHex(initial.currentContractState);
  const queryStart = queries.length;
  const witnessStart = witnessCalls.length;
  localSkMode = mode;
  try {
    const output = contract.circuits[name](context, argument);
    return {
      success: true, name, mode,
      argument: typeof argument === 'string' ? argument : Array.from(argument),
      initialStateHex: before,
      afterStateHex: stateAfter(initial, output.context.currentQueryContext.state.state),
      privateState: output.context.currentPrivateState,
      result: output.result,
      gasCost: Object.fromEntries(Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()])),
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)), alignment,
      })),
      queries: queries.slice(queryStart),
      witnessCalls: witnessCalls.slice(witnessStart),
    };
  } catch (error) {
    return {
      success: false, name, mode,
      argument: typeof argument === 'string' ? argument : Array.from(argument),
      initialStateHex: before,
      afterStateHex: stateAfter(initial, context.currentQueryContext.state.state),
      privateState: context.currentPrivateState,
      error: error.message,
      compactError: error.constructor.name === 'CompactError',
      queries: queries.slice(queryStart),
      witnessCalls: witnessCalls.slice(witnessStart),
    };
  } finally {
    localSkMode = 'zero';
  }
}
const cases = [];
let organizerCalls;
for (const present of [false, true]) {
  queries.length = 0;
  witnessCalls.length = 0;
  const participants = Array.from({ length: 5000 }, () => ({
    is_some: false, value: '',
  }));
  if (present) participants[7] = { is_some: true, value: 'alice' };
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, participants);
  const constructorQueries = queries.map((query) => structuredClone(query));
  const constructorWitnessCalls = [...witnessCalls];
  const view = ledger(initial.currentContractState.data);
  if (present) {
    organizerCalls = {
      participantSuccess: captureOrganizerCall(initial, 'add_participant', 'bob', 'zero'),
      organizerSuccess: captureOrganizerCall(initial, 'add_organizer', new Uint8Array(32).fill(7), 'zero'),
      participantMissingKey: captureOrganizerCall(initial, 'add_participant', 'bob', 'missing'),
      participantNotOrganizer: captureOrganizerCall(initial, 'add_participant', 'bob', 'other'),
      organizerMissingKey: captureOrganizerCall(initial, 'add_organizer', new Uint8Array(32).fill(7), 'missing'),
      organizerNotOrganizer: captureOrganizerCall(initial, 'add_organizer', new Uint8Array(32).fill(7), 'other'),
    };
  }
  cases.push({
    present,
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    privateState: initial.currentPrivateState,
    witnessCalls: constructorWitnessCalls,
    eligibleAlice: view.eligible_participants.member('alice'),
    eligibleSize: view.eligible_participants.size().toString(),
    organizerSize: view.organizer_pks.size().toString(),
    constructorQueries,
    checkIn: captureCheckIn(initial, present ? 'alice' : 'bob'),
  });
}
process.stdout.write(JSON.stringify({
  source: 'test-center/test-contracts/welcome.compact',
  cases,
  organizerCalls,
}, null, 2) + '\n');
