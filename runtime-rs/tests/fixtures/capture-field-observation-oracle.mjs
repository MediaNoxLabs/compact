// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Pass the generated TS bundle from the unchanged field_observation_oracle source.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const original = r.QueryContext.prototype.query;
const queries = [];
r.QueryContext.prototype.query = function (...args) {
  const q = { ops: args[0] }; queries.push(q);
  try { const out = original.call(this, ...args); q.gas = out.gasCost; return out; }
  catch (error) { q.error = String(error); throw error; }
};
function run(name, first, second, selected = false, zeroGas = false) {
  const contract = new Contract({});
  const key = { bytes: new Uint8Array(32) };
  const init = contract.initialState({ initialPrivateState: [], initialZswapLocalState: r.emptyZswapLocalState(key) }, BigInt(first), BigInt(second));
  const before = Buffer.from(init.currentContractState.serialize()).toString('hex');
  const context = r.createCircuitContext(r.dummyContractAddress(), key, init.currentContractState.data, []);
  if (zeroGas) context.gasLimit = r.emptyRunningCost();
  queries.length = 0;
  const row = { name, first, second, selected, zeroGas, before };
  try {
    const args = ['selected', 'optional'].includes(name) ? [selected] : [];
    const out = contract.circuits[name](context, ...args);
    const after = r.ContractState.deserialize(init.currentContractState.serialize());
    after.data = new r.ChargedState(out.context.currentQueryContext.state.state);
    Object.assign(row, { result: out.result, after: Buffer.from(after.serialize()).toString('hex'),
      effects: out.context.currentQueryContext.effects, privateState: out.context.currentPrivateState,
      privateOutputs: out.proofData.privateTranscriptOutputs, output: out.proofData.output,
      gas: out.gasCost, publicTranscript: out.proofData.publicTranscript });
    const replay = r.createCircuitContext(r.dummyContractAddress(), key, init.currentContractState.data, []);
    row.replayGas = original.call(replay.currentQueryContext, queries.flatMap(q => q.ops), replay.costModel).gasCost;
  } catch (error) { row.error = String(error); }
  return { ...row, queries: [...queries] };
}
const rows = [];
for (const [first, second] of [[7, 19], [0, 3]]) {
  for (const name of ['direct', 'snapshot', 'via_field_helper', 'via_snapshot_helper', 'pair']) rows.push(run(name, first, second));
  for (const name of ['selected', 'optional']) for (const selected of [false, true]) rows.push(run(name, first, second, selected));
}
rows.push(run('direct', 7, 19, false, true));
for (const name of ['selected', 'optional']) for (const selected of [false, true]) rows.push(run(name, 7, 19, selected, true));
process.stdout.write(JSON.stringify(rows, (_, x) => x instanceof Uint8Array ? Array.from(x) : typeof x === 'bigint' ? x.toString() : x instanceof Map ? Object.fromEntries(x) : x, 2) + '\n');
