// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits } = await import(pathToFileURL(path));
const max128 = (1n << 128n) - 1n, max129 = (1n << 129n) - 1n;
const bytes = value => Array.from({length: 32}, (_, i) => Number((value >> BigInt(i*8)) & 255n));
function pure(name, a, b) {
  try {const result = pureCircuits[name](a, b); return {name, a, b, aBytes:bytes(a), bBytes:bytes(b), result, bytes:bytes(result)};}
  catch(error) {return {name, a, b, aBytes:bytes(a), bBytes:bytes(b), error:String(error)};}
}
function witnessed(a, b, selected) {
  const key = {bytes:new Uint8Array(32)};
  const c = new Contract({next_value:(ctx, tag)=>[[...ctx.privateState, Number(tag)], tag === 1n ? a : b]});
  const init = c.initialState({initialPrivateState:[], initialZswapLocalState:r.emptyZswapLocalState(key)});
  const ctx = r.createCircuitContext(r.decodeContractAddress(key.bytes), key, init.currentContractState.data, []);
  const before = Buffer.from(init.currentContractState.serialize()).toString('hex');
  const out = c.circuits.witnessed(ctx, selected);
  const state = r.ContractState.deserialize(init.currentContractState.serialize());
  state.data = new r.ChargedState(out.context.currentQueryContext.state.state);
  return {name:'witnessed', a, b, selected, result:out.result, bytes:bytes(out.result), before,
    after:Buffer.from(state.serialize()).toString('hex'), effects:out.context.currentQueryContext.effects,
    privateState:out.context.currentPrivateState, privateOutputs:out.proofData?.privateTranscriptOutputs ?? null,
    output:out.proofData?.output ?? null, gas:out.gasCost ?? null};
}
const rows = [
  pure('add128',0n,0n),pure('add128',max128,1n),pure('add128',max128,max128),
  pure('merge128',max128,0n),pure('merge128',max128-1n,1n),pure('merge128',max128,1n),
  pure('mixed',max129,255n),pure('mixed',1n<<128n,1n),
  pure('narrow129',max129,0n),pure('narrow129',max129,1n),pure('narrow129',max129,2n),
  witnessed(max128,max128,true),witnessed(max128,1n,true),witnessed(max128,max128,false),
];
process.stdout.write(JSON.stringify(rows, (_,x)=>x instanceof Map ? Object.fromEntries(x)
  : x instanceof Uint8Array ? Array.from(x) : typeof x==='bigint' ? String(x) : x, 2)+'\n');
