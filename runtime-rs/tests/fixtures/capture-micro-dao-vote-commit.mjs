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
// Seeded original-source execution; no funded token lifecycle claim.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const resolved = createRequire(path).resolve('@midnight-ntwrk/compact-runtime');
const r = await import(pathToFileURL(resolved));
const { Contract, ledger: readLedger } = await import(pathToFileURL(path));
const max = (1n << 128n) - 1n;
const bytes = new r.CompactTypeBytes(32);
const u64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
const u128 = new r.CompactTypeUnsignedInteger(max, 16);
const phase = new r.CompactTypeEnum(3, 1);
const fill = n => new Uint8Array(32).fill(n);
const key = { bytes: fill(7) };
const cell = (types, values) => r.StateValue.newCell({ value: types.flatMap((type, i) => type.toValue(values[i])), alignment: types.flatMap(type => type.alignment()) });
const scalar = (type, value) => cell([type], [value]);
const array = values => values.reduce((a, value) => a.arrayPush(value), r.StateValue.newArray());
const hex = state => Buffer.from(state.serialize()).toString('hex');
const gas = value => Object.fromEntries(Object.entries(value).map(([name, cost]) => [name, String(cost)]));
let witnessCalls = [], queries = [], events = [], mode = 'normal';
const pad = s => {const out=fill(0);out.set(new TextEncoder().encode(s));return out;};
const vector=new r.CompactTypeVector(3,bytes);
const nullifier=round=>r.persistentHash(vector,[pad('lares:udao:cm-nul:'),r.convertFieldToBytes(32,round,'research round'),fill(4)]);
const bump=p=>({...p,calls:p.calls+1});
const unused = () => { throw new Error('unexpected witness'); };
const contract = new Contract({
  local_secret_key: ({privateState:p}) => { witnessCalls.push('secret'); if(mode==='secretFailure')throw new Error('secret witness refused');return [bump(p), mode==='secretMalformed'?new Uint8Array(31):fill(4)]; },
  local_state: ({privateState:p}) => { witnessCalls.push('state');if(mode==='stateFailure')throw new Error('state witness refused');return [bump(p),p.phase]; },
  local_record_vote: ({privateState:p},vote) => {witnessCalls.push(['record',vote]);if(mode==='recordFailure')throw new Error('record witness refused');return [{...bump(p),vote},[]];},
  local_advance_state: ({privateState:p}) => {witnessCalls.push('advance');if(mode==='advanceFailure')throw new Error('advance witness refused');return [{...bump(p),phase:p.phase+1},[]];},
  local_vote_cast: unused, local_path_of_cm: unused,
});
for (const [method, kind] of [['_createZswapInput_0', 'input'], ['_createZswapOutput_0', 'output']]) {
  const original = contract[method];
  contract[method] = function (...args) {
    const result = original.apply(this, args);
    events.push({kind, coin: args[2], ...(kind === 'output' ? {recipient: args[3]} : {})});
    return result;
  };
}
const originalQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ops: args[0], gas: gas(result.gasCost)});
  return result;
};
function run(name, options = {}) {
  const initial = contract.initialState({initialPrivateState:{calls:0},initialZswapLocalState:r.emptyZswapLocalState(key)}, fill(4), {seed_dust: options.cost ?? 10n, buy_in_dust:3n});
  const state = initial.currentContractState;
  const fields = state.data.state.asArray();
  fields[1] = scalar(phase, options.phase ?? 1);
  fields[6] = scalar(u64, options.round ?? 7n);
  if(options.duplicate)fields[8]=r.StateValue.newMap(fields[8].asMap().insert({value:bytes.toValue(nullifier(options.round??7n)),alignment:bytes.alignment()},r.StateValue.newNull()));
  if(options.fullTree){const tree=fields[7].asArray();tree[1]=scalar(u64,1024n);fields[7]=array(tree);}
  state.data = new r.ChargedState(array(fields));
  const before = hex(state), beforeState = state.data;
  const context = r.createCircuitContext(r.dummyContractAddress(), key, beforeState, {calls:0,phase:options.privatePhase??0,vote:null});
  context.currentZswapLocalState.currentIndex = 2n;
  const colorContext=r.createCircuitContext(r.dummyContractAddress(),key,beforeState,null);
  const color=contract.circuits.dao_voting_token(colorContext).result;
  const coin = {nonce:fill(3),color:options.wrongColor?fill(0):color,value:options.value ?? 1n};
  witnessCalls = []; queries = []; events = []; mode = options.mode ?? 'normal';
  try {
    const result = contract.circuits.vote_commit(context, options.ballot??true, coin);
    const captured = [...queries], calls = [...witnessCalls], intentEvents = [...events];
    state.data = new r.ChargedState(result.context.currentQueryContext.state.state);
    const replay = r.createCircuitContext(r.dummyContractAddress(), key, r.ContractState.deserialize(Buffer.from(before, "hex")).data, null);
    for (const [commitment, index] of result.context.currentQueryContext.comIndices) replay.currentQueryContext = replay.currentQueryContext.insertCommitment(commitment, index);
    let replayProbe;
    try { replayProbe = {gas: gas(originalQuery.call(replay.currentQueryContext, captured.flatMap(q => q.ops), replay.costModel).gasCost)}; }
    catch(error) { replayProbe = {error: String(error.message)}; }
    const view = readLedger(state.data);
    const ledgerAfter = {round:view.round,state:view.state,committedSize:view.committed_participants.size(),treeNext:u64.fromValue(state.data.state.asArray()[7].asArray()[1].asCell().value)};
    return {name,options,before,ledgerAfter,after:hex(state),result:result.result,output:result.proofData.output,privateState:result.context.currentPrivateState,privateOutputs:result.proofData.privateTranscriptOutputs,effects:result.context.currentQueryContext.effects,queries:captured,publicTranscript:result.proofData.publicTranscript,witnessCalls:calls,events:intentEvents,plan:result.context.currentZswapLocalState,wrapperLastQueryGas:gas(result.gasCost),queryCostSum:Object.fromEntries(Object.keys(result.gasCost).map(k=>[k,String(captured.reduce((sum,q)=>sum+BigInt(q.gas[k]),0n))])),replayProbe,provisionalComIndices:Array.from(result.context.currentQueryContext.comIndices)};
  } catch(error) {
    return {name,options,before,error:String(error.message),queries:[...queries],witnessCalls:[...witnessCalls],events:[...events]};
  }
}
const rows = [
 run('yes7'),run('no7',{ballot:false}),run('yes0',{round:0n}),run('yesMax',{round:(1n<<64n)-1n}),
 run('wrongPhase',{phase:0}),run('wrongPrivate',{privatePhase:1}),run('wrongColor',{wrongColor:true}),run('wrongValue',{value:2n}),
 run('duplicate',{duplicate:true}),run('fullTree',{fullTree:true}),
 run('stateFailure',{mode:'stateFailure'}),run('secretFailure',{mode:'secretFailure'}),run('secretMalformed',{mode:'secretMalformed'}),
 run('recordFailure',{mode:'recordFailure'}),run('advanceFailure',{mode:'advanceFailure'}),
 run('phaseBeforeStateFailure',{phase:0,mode:'stateFailure'}),run('colorBeforeValue',{wrongColor:true,value:2n}),run('valueBeforeSecret',{value:2n,mode:'secretFailure'}),
];
process.stdout.write(JSON.stringify(rows,(_,value)=>value instanceof Uint8Array?Array.from(value):typeof value==='bigint'?String(value):value instanceof Map?Object.fromEntries(value):value,2)+'\n');
