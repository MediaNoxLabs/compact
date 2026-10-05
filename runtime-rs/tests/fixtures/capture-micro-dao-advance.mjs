// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Original microDAO source; explicit prior states do not claim a funded DAO lifecycle.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const r = await import(pathToFileURL(createRequire(path).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const max = (1n << 64n) - 1n;
const bytes = new r.CompactTypeBytes(32), u64 = new r.CompactTypeUnsignedInteger(max, 8);
const u128 = new r.CompactTypeUnsignedInteger((1n << 128n) - 1n, 16);
const phase = new r.CompactTypeEnum(3, 1), bool = r.CompactTypeBoolean, str = r.CompactTypeOpaqueString;
const key = { bytes: new Uint8Array(32) }, fill = n => new Uint8Array(32).fill(n);
const cell = (types, values) => r.StateValue.newCell({ value: types.flatMap((t, i) => t.toValue(values[i])), alignment: types.flatMap(t => t.alignment()) });
const scalar = (t, v) => cell([t], [v]);
const array = values => values.reduce((a, v) => a.arrayPush(v), r.StateValue.newArray());
const hex = state => Buffer.from(state.serialize()).toString('hex');
const gas = v => Object.fromEntries(Object.entries(v).map(([k, x]) => [k, String(x)]));
let calls = [], queries = [], mode = 'normal';
const unused = () => { throw new Error('unexpected witness'); };
const c = new Contract({
 local_secret_key: ({ privateState: p }) => {
  calls.push('secret');
  if (mode === 'failure') throw new Error('advance witness refused');
  return [{ calls: p.calls + 1 }, mode === 'malformed' ? new Uint8Array(31) : fill(mode === 'wrong' ? 7 : 4)];
 },
 local_state: unused, local_vote_cast: unused, local_advance_state: unused,
 local_record_vote: unused, local_path_of_cm: unused,
});
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
 const out = original.call(this, ...args);
 queries.push({ ops: args[0], gas: gas(out.gasCost) });
 return out;
};
function seeded(options) {
 const init = c.initialState({ initialPrivateState: { calls: 0 }, initialZswapLocalState: r.emptyZswapLocalState(key) }, fill(4), { seed_dust: 10n, buy_in_dust: 3n });
 const f = init.currentContractState.data.state.asArray();
 f[1] = scalar(phase, options.phase ?? 3);
 f[4] = scalar(u64, options.yes ?? 2n); f[5] = scalar(u64, options.no ?? 3n); f[6] = scalar(u64, options.round ?? 7n);
 f[10] = cell([bytes, bytes, u128, u64], [fill(41), fill(42), 99n, 17n]);
 f[11] = scalar(bool, options.pot ?? true);
 if (!options.empty) {
  f[2] = cell([bool, str], [true, 'Round 🗳️']);
  f[3] = cell([bool, bytes], [true, fill(31)]);
  let tree = f[7].asArray()[0].asBoundedMerkleTree();
  for (const [i, n] of [11, 12].entries()) tree = tree.update(BigInt(i), { value: bytes.toValue(fill(n)), alignment: bytes.alignment() });
  f[7] = array([r.StateValue.newBoundedMerkleTree(tree.rehash()), scalar(u64, 2n)]);
  for (const index of [8, 9]) {
   let map = new r.StateMap();
   for (const n of [21, 22]) map = map.insert({ value: bytes.toValue(fill(n)), alignment: bytes.alignment() }, r.StateValue.newNull());
   f[index] = r.StateValue.newMap(map);
  }
 }
 init.currentContractState.data = new r.ChargedState(array(f));
 return init.currentContractState;
}
function run(name, options = {}) {
 const state = seeded(options), before = hex(state), beforeState = state.data;
 const ctx = r.createCircuitContext(r.dummyContractAddress(), key, beforeState, { calls: 0 });
 if (options.gasLimit) ctx.gasLimit = options.gasLimit;
 calls = []; queries = []; mode = options.mode ?? 'normal';
 try {
  const out = c.circuits.advance(ctx), q = [...queries], witnessCalls = [...calls];
  state.data = new r.ChargedState(out.context.currentQueryContext.state.state);
  const replay = r.createCircuitContext(r.dummyContractAddress(), key, beforeState, null);
  const replayGas = gas(original.call(replay.currentQueryContext, q.flatMap(x => x.ops), replay.costModel).gasCost);
  return { name, options, before, after: hex(state), result: out.result, output: out.proofData.output, privateState: out.context.currentPrivateState, privateOutputs: out.proofData.privateTranscriptOutputs, effects: out.context.currentQueryContext.effects, queries: q, reportedGas: gas(out.gasCost), queryCostSum: Object.fromEntries(Object.keys(out.gasCost).map(k => [k, String(q.reduce((sum, x) => sum + BigInt(x.gas[k]), 0n))])), replayGas, publicTranscript: out.proofData.publicTranscript, witnessCalls };
 } catch (error) {
  return { name, options, before, error: error.message, witnessCalls: [...calls], queries: [...queries] };
 }
}
const rows = [
 run('commit', { phase: 1 }), run('reveal', { phase: 2 }), run('final'),
 run('zero', { yes: 0n, no: 0n }), run('equal', { yes: 3n, no: 3n }),
 run('empty', { empty: true, pot: false, round: 0n }),
 run('setup', { phase: 0 }), run('setupWrong', { phase: 0, mode: 'wrong' }),
 ...[1, 2, 3].map(phase => run(`wrong${phase}`, { phase, mode: 'wrong' })),
 run('cashOut', { yes: 4n, no: 3n }),
 run('thresholdMax', { yes: max - 1n, no: max - 1n }),
 run('thresholdMaxReject', { yes: max, no: max - 1n }),
 run('noMax', { no: max }), run('roundMax', { round: max }),
 run('witnessFailure', { mode: 'failure' }), run('witnessMalformed', { mode: 'malformed' }),
 run('zeroGas', { gasLimit: { readTime: 0n, computeTime: 0n, bytesWritten: 0n, bytesDeleted: 0n } }),
];
const budget = Object.fromEntries(Object.entries(rows[2].queries[0].gas).map(([k, v]) => [k, BigInt(v)]));
rows.push(run('prefixGas', { gasLimit: budget }));
process.stdout.write(JSON.stringify(rows, (_, x) => x instanceof Uint8Array ? Array.from(x) : typeof x === 'bigint' ? String(x) : x, 2) + '\n');
