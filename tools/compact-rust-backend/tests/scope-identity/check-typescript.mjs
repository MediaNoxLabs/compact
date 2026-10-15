// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Run against the exact generated TS contract, with its pinned runtime installed.
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
const [path] = process.argv.slice(2), require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const {Contract, pureCircuits} = await import(pathToFileURL(path));
const key = {bytes: new Uint8Array(32)}, c = new Contract({});
function context(seed = 5n) {
  const init = c.initialState({initialPrivateState: [], initialZswapLocalState: r.emptyZswapLocalState(key)}, seed);
  return r.createCircuitContext(r.dummyContractAddress(), key, init.currentContractState.data, []);
}
const outer = new Uint8Array(32).fill(1), inner = new Uint8Array(32).fill(2);
const out = c.circuits.shadow(context(), outer, inner);
assert.deepEqual(out.context.currentQueryContext.effects.claimedNullifiers, [Buffer.from(inner).toString('hex')]);
assert.deepEqual(out.context.currentQueryContext.effects.claimedShieldedSpends, [Buffer.from(outer).toString('hex')]);
const rows = [{name: 'shadow', nullifier: '02'.repeat(32), spend: '01'.repeat(32)}];
for (const [name, args, result, marker] of [
  ['local_return', [11n], 12n, 12n], ['siblings', [11n], 11n, 3n],
  ['nested', [11n], 11n, 2n], ['suffix', [11n,7n,5n,3n], 19n, 3n],
]) {
  const out = c.circuits[name](context(), ...args);
  assert.equal(out.result, result);
  assert.equal(c.circuits.read_marker(out.context).result, marker);
  rows.push({name, result, marker});
}
for (const seed of [0n,5n,65535n]) {
  assert.equal(c.circuits.read_count(context(seed)).result, seed + 9n);
  assert.equal(c.circuits.counter_scopes(context(seed),17n).result, seed + 28n);
}
for (const value of [0n,11n,93n]) {
  assert.equal(pureCircuits.folded(value), value + 3n);
  assert.equal(pureCircuits.alias_a(value,7n), value);
  assert.equal(pureCircuits.alias_b(value,7n), value);
  assert.equal(pureCircuits.dollars(value,7n), value + 7n);
}
console.log(JSON.stringify({rows, constructorAndCounterSeeds:[0,5,65535], pureSeeds:[0,11,93], passed:true}, (_,x)=>typeof x === 'bigint'?String(x):x,2));
