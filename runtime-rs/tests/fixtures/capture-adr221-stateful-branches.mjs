// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Opposite sampled ternary branches and nonempty collection observations.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [contractPath, sourcePath, compilerPath, mode] = process.argv.slice(2);
if (!contractPath || !sourcePath || !compilerPath || !['ternary', 'set-size', 'call-arg'].includes(mode)) {
  throw new Error('usage: node capture-adr221-stateful-branches.mjs <contract/index.js> <source.compact> <compactc-scheme> ternary|set-size|call-arg');
}
const contractIndex = resolve(contractPath);
const runtimeIndex = createRequire(contractIndex).resolve('@midnight-ntwrk/compact-runtime');
const runtimePackage = JSON.parse(readFileSync(join(dirname(runtimeIndex), '..', 'package.json')));
const runtime = await import(pathToFileURL(runtimeIndex).href);
const { Contract, ledger } = await import(pathToFileURL(contractIndex).href);
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const key = { bytes: new Uint8Array(32) };
const contract = new Contract(mode === 'ternary' ? {
  echoField: (context, value) => [context.privateState, value],
} : mode === 'call-arg' ? {
  sumWitness: (context, values) => [context.privateState, values[0] + values[1]],
} : {});
const hex = value => Buffer.from(value.serialize()).toString('hex');
const gas = value => Object.fromEntries(Object.entries(value).map(([name, n]) => [name, String(n)]));
const array = fields => fields.reduce((state, field) => state.arrayPush(field), runtime.StateValue.newArray());
const cell = (descriptor, value) => runtime.StateValue.newCell({
  value: descriptor.toValue(value), alignment: descriptor.alignment(),
});
function seeded() {
  const initial = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(key),
  }, ...(mode === 'ternary' ? [true, true, 111n] : []));
  const fields = initial.currentContractState.data.state.asArray();
  if (mode === 'ternary') {
    fields[0] = cell(runtime.CompactTypeBoolean, true);
  } else if (mode === 'call-arg') {
    fields[6] = cell(runtime.CompactTypeBoolean, false);
  } else {
    const field = runtime.CompactTypeField;
    const aligned = value => ({ value: field.toValue(BigInt(value)), alignment: field.alignment() });
    fields[2] = runtime.StateValue.newMap(fields[2].asMap().insert(aligned(42), runtime.StateValue.newNull()));
    fields[3] = runtime.StateValue.newMap(fields[3].asMap().insert(aligned(7), cell(field, 9n)));
  }
  initial.currentContractState.data = new runtime.ChargedState(array(fields));
  return initial;
}
function contractState(original, current) {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(current);
  for (const name of original.operations()) state.setOperation(name, original.operation(name));
  state.maintenanceAuthority = original.maintenanceAuthority;
  state.balance = original.balance;
  return state;
}
function shape(op) {
  if (typeof op === 'string') return { kind: op };
  for (const name of ['idx', 'push', 'ins', 'rem', 'dup', 'popeq', 'branch', 'jmp', 'swap', 'concat', 'addi']) {
    if (!op[name]) continue;
    const value = op[name];
    if (name === 'idx') return { kind: name, cached: value.cached, pushPath: value.pushPath, pathLength: value.path.length };
    if (name === 'popeq') return { kind: name, cached: value.cached, resultAtoms: value.result.value.map(atom => Array.from(atom)) };
    if (name === 'push') return { kind: name, storage: value.storage };
    return { kind: name, ...Object.fromEntries(Object.entries(value).filter(([key]) => ['cached', 'n', 'skip', 'immediate'].includes(key))) };
  }
  throw new Error(`unknown public VM operation ${Object.keys(op)}`);
}
let queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = originalQuery.call(this, ...args);
  queries.push({ opTags: args[0].map(op => typeof op === 'string' ? op : Object.keys(op)[0]), gasCost: gas(output.gasCost) });
  return output;
};
function run(name) {
  const initial = seeded();
  const before = hex(initial.currentContractState);
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), key,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  queries = [];
  const output = contract.circuits[name](context);
  const ownQueries = [...queries];
  const view = ledger(output.context.currentQueryContext.state);
  const sum = Object.fromEntries(Object.keys(output.gasCost).map(dimension => [dimension,
    String(ownQueries.reduce((total, query) => total + BigInt(query.gasCost[dimension]), 0n))]));
  return {
    name, before, after: hex(contractState(initial.currentContractState, output.context.currentQueryContext.state.state)),
    result: String(output.result),
    effects: output.context.currentQueryContext.effects,
    flags: mode === 'set-size' ? { set: view.flag_set, map: view.flag_map, setSize: String(view.s.size()), mapSize: String(view.m.size()) } :
      mode === 'call-arg' ? { flag: view.flag, fieldCell: String(view.fieldCell) } : undefined,
    publicTranscriptShape: output.proofData.publicTranscript.map(shape),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
    queryCount: ownQueries.length, queries: ownQueries,
    queryCostSum: sum, reportedGas: gas(output.gasCost),
  };
}
const names = mode === 'ternary'
  ? ['streamCompareEq', 'streamStructMember']
  : mode === 'set-size' ? ['check_set_empty', 'check_map_empty'] : ['impureInIfArm'];
process.stdout.write(JSON.stringify({
  format: 'adr221-original-stateful-opposite-state-v1', mode,
  provenance: {
    sourceSha256: hash(sourcePath), generatedJsSha256: hash(contractIndex),
    runtimeVersion: runtimePackage.version, runtimeJsSha256: hash(runtimeIndex),
    compilerSha256: hash(compilerPath),
  },
  rows: names.map(run),
}, null, 2) + '\n');
