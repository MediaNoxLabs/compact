// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// Complete original source; explicit prior state does not claim funded game setup.
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
const [path] = process.argv.slice(2);
const r = await import(pathToFileURL(createRequire(path).resolve('@midnight-ntwrk/compact-runtime')));
const {Contract} = await import(pathToFileURL(path));
const bytes = new r.CompactTypeBytes(32), field = r.CompactTypeField;
const phase = new r.CompactTypeEnum(6, 1), vector = new r.CompactTypeVector(2, bytes);
const boardType = {alignment: () => field.alignment(), toValue: b => field.toValue(b.position)};
const commitmentType = {alignment: () => field.alignment(), toValue: c => field.toValue(c.value)};
const maybeType = {
  alignment: () => r.CompactTypeBoolean.alignment().concat(field.alignment()),
  toValue: m => r.CompactTypeBoolean.toValue(m.is_some).concat(field.toValue(m.value)),
};
const key = {bytes: new Uint8Array(32)}, address = r.dummyContractAddress();
const secret = color => new Uint8Array(32).fill(color === 'red' ? 7 : 8);
const pad = s => {const b = new Uint8Array(32); b.set(new TextEncoder().encode(s)); return b;};
const publicKey = (color, keyColor = color) => r.persistentHash(vector, [secret(keyColor), pad(`coracle:${color}`)]);
const cell = (type, value) => r.StateValue.newCell({value: type.toValue(value), alignment: type.alignment()});
const array = values => values.reduce((a, v) => a.arrayPush(v), r.StateValue.newArray());
const hex = s => Buffer.from(s.serialize()).toString('hex');
const gas = v => Object.fromEntries(Object.entries(v).map(([k, x]) => [k, String(x)]));
let calls = [], queries = [], active = {};
const c = new Contract({
  local_secret_key: ({privateState: p}) => {
    calls.push('secret');
    const wrong = active.impostor || (active.changedSecret && calls.length > 1);
    return [{calls: p.calls + 1}, wrong ? new Uint8Array(32).fill(9) : secret(active.color)];
  },
  local_board: ({privateState: p}) => {
    calls.push('board');
    return [{calls: p.calls + 1}, {
      nonce: active.wrongNonce ? 12n : 11n,
      contents: {position: active.invalidBoard ? 0n : active.board},
    }];
  },
  local_set_board: () => {throw Error('unselected local_set_board called');},
  fresh_nonce: () => {throw Error('unselected fresh_nonce called');},
});
const query = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = query.call(this, ...args);
  queries.push({ops: args[0], gas: gas(out.gasCost)});
  return out;
};
function seeded(color, options) {
  const init = c.initialState({initialPrivateState: {calls: 0}, initialZswapLocalState: r.emptyZswapLocalState(key)});
  const fields = init.currentContractState.data.state.asArray();
  fields[0] = cell(bytes, publicKey('red'));
  fields[1] = cell(bytes, publicKey('blue', options.bothPlayers ? 'red' : 'blue'));
  const commitment = {value: r.transientCommit(boardType, {position: options.board}, 11n)};
  fields[2] = cell(commitmentType, commitment);
  fields[3] = cell(commitmentType, commitment);
  fields[4] = cell(maybeType, {is_some: !options.empty, value: options.empty ? 0n : (options.dead ? options.board : 5n)});
  fields[5] = cell(phase, options.wrongTurn ? (color === 'red' ? 4 : 3) : (options.firstBlue ? 2 : color === 'red' ? 3 : 4));
  init.currentContractState.data = new r.ChargedState(array(fields));
  const ctx = r.createCircuitContext(address, key, init.currentContractState.data, {calls: 0});
  return {state: init.currentContractState, ctx};
}
function run(name, color = 'red', guess = 4n, options = {}) {
  options = {board: 1n, ...options};
  active = {color, ...options};
  let {state, ctx} = seeded(color, options);
  if (options.repeat) {
    ctx = c.circuits.guess(ctx, guess).context;
    state.data = new r.ChargedState(ctx.currentQueryContext.state.state);
  }
  const before = hex(state), beforeState = state.data;
  if (options.gasLimit) ctx.gasLimit = options.gasLimit;
  calls = []; queries = [];
  try {
    const out = c.circuits.guess(ctx, guess);
    const q = [...queries];
    state.data = new r.ChargedState(out.context.currentQueryContext.state.state);
    const replay = r.createCircuitContext(address, key, beforeState, null);
    const replayGas = gas(query.call(replay.currentQueryContext, q.flatMap(x => x.ops), replay.costModel).gasCost);
    return {name, color, guess, options, before, after: hex(state), result: out.result,
      output: out.proofData.output, privateState: out.context.currentPrivateState,
      privateOutputs: out.proofData.privateTranscriptOutputs, effects: out.context.currentQueryContext.effects,
      queries: q, reportedGas: gas(out.gasCost),
      queryCostSum: Object.fromEntries(Object.keys(out.gasCost).map(k => [k, String(q.reduce((s, x) => s + BigInt(x.gas[k]), 0n))])),
      replayGas, publicTranscript: out.proofData.publicTranscript, witnessCalls: [...calls]};
  } catch (error) {
    return {name, color, guess, options, before, error: error.message, witnessCalls: [...calls], queries: [...queries]};
  }
}
const rows = [
  run('red'), run('blue', 'blue'),
  run('bothPlayersRed', 'red', 4n, {bothPlayers: true}),
  run('bothPlayersBlueTurn', 'red', 4n, {bothPlayers: true, wrongTurn: true}), run('firstBlue', 'blue', 9n, {firstBlue: true, empty: true}),
  run('redEmpty', 'red', 1n, {empty: true, board: 9n}),
];
for (const color of ['red', 'blue']) {
  for (const option of ['impostor', 'changedSecret', 'wrongNonce', 'invalidBoard', 'wrongTurn', 'dead', 'repeat']) {
    rows.push(run(`${color}-${option}`, color, 4n, {[option]: true}));
  }
  rows.push(run(`${color}-zeroPosition`, color, 0n), run(`${color}-largePosition`, color, 10n));
}
rows.push(run('zeroGas', 'red', 4n, {gasLimit: {readTime: 0n, computeTime: 0n, bytesWritten: 0n, bytesDeleted: 0n}}));
rows.push(run('perQueryBudget', 'red', 4n, {gasLimit: Object.fromEntries(Object.entries(rows[0].queries[0].gas).map(([k, v]) => [k, BigInt(v)]))}));
process.stdout.write(JSON.stringify(rows, (_, x) => x instanceof Uint8Array ? Array.from(x) : typeof x === 'bigint' ? String(x) : x, 2) + '\n');
