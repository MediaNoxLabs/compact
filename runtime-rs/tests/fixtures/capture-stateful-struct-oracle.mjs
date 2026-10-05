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

// Independent TS evidence: named fields normalize to declared member order.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [path] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const bytes = n => Uint8Array.from([n, ...Array(31).fill(0)]);
const key = { bytes: bytes(0) };
const addr = r.decodeContractAddress(bytes(9));
const coin = { nonce: bytes(7), color: bytes(8), value: 42n };
const recipient = { is_left: false, left: key, right: { bytes: bytes(9) } };
const gas = v => v == null ? null : Object.fromEntries(
  Object.entries(v).map(([k, x]) => [k, String(x)])
);
const queries = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = original.call(this, ...args);
  queries.push({ ops: args[0], gas: gas(out.gasCost) });
  return out;
};

function run(name, selected) {
  const c = new Contract({
    next_value: (ctx, tag) => [
      [...ctx.privateState, Number(tag)],
      tag * 10n + BigInt(ctx.privateState.length),
    ],
  });
  const init = c.initialState({
    initialPrivateState: [], initialZswapLocalState: r.emptyZswapLocalState(key),
  });
  const ctx = r.createCircuitContext(addr, key, init.currentContractState.data, []);
  ctx.currentZswapLocalState.currentIndex = 7n;
  const before = Buffer.from(init.currentContractState.serialize()).toString('hex');
  queries.length = 0;
  const args = name === 'snapshot' ? [selected]
    : name === 'planned' ? [coin, recipient]
    : name === 'pure_snapshot' ? [9n, (1n << 64n) - 1n] : [];
  const out = c.circuits[name](ctx, ...args);
  const state = r.ContractState.deserialize(init.currentContractState.serialize());
  state.data = new r.ChargedState(out.context.currentQueryContext.state.state);
  return {
    name, selected, args, before,
    after: Buffer.from(state.serialize()).toString('hex'),
    result: out.result,
    effects: out.context.currentQueryContext.effects,
    privateState: out.context.currentPrivateState,
    // Pure wrappers do not expose proofData or gasCost.
    privateOutputs: out.proofData?.privateTranscriptOutputs ?? [],
    output: out.proofData?.output ?? null,
    plan: out.context.currentZswapLocalState,
    queries: [...queries], gas: gas(out.gasCost),
  };
}
const rows = [
  run('snapshot', true), run('snapshot', false), run('reverse'),
  run('nested'), run('planned'), run('pure_snapshot'),
];
process.stdout.write(JSON.stringify(rows, (_, x) =>
  x instanceof Map ? Object.fromEntries(x)
    : x instanceof Uint8Array ? Array.from(x)
    : typeof x === 'bigint' ? String(x) : x, 2) + '\n');
