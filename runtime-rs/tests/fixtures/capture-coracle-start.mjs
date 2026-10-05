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
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { readFileSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
// Compile the unchanged Coracle source with --skip-zk; link the generated
// package to an isolated corrected TS runtime, then pass all four paths.
const [path, runtimeRoot, ledgerPath, sourcePath] = process.argv.slice(2);
if (!path || !runtimeRoot || !ledgerPath || !sourcePath) throw new Error('expected generated index.js, corrected runtime root, pinned ledger-v8 module, source');
const resolved = realpathSync(createRequire(path).resolve('@midnight-ntwrk/compact-runtime'));
if (!resolved.startsWith(realpathSync(runtimeRoot) + '/')) throw new Error('wrong corrected runtime');
const ledger = await import(pathToFileURL(ledgerPath).href);
const r = await import(pathToFileURL(resolved).href);
const { Contract, ledger: readLedger } = await import(pathToFileURL(path).href);
const max = (1n << 128n) - 1n;
const bytes = new r.CompactTypeBytes(32);
const u64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
const u128 = new r.CompactTypeUnsignedInteger(max, 16);
const phase = new r.CompactTypeEnum(6, 1);
const fill = n => new Uint8Array(32).fill(n);
const key = { bytes: fill(7) };
const cell = (types, values) => r.StateValue.newCell({ value: types.flatMap((type, i) => type.toValue(values[i])), alignment: types.flatMap(type => type.alignment()) });
const scalar = (type, value) => cell([type], [value]);
const array = values => values.reduce((a, value) => a.arrayPush(value), r.StateValue.newArray());
const hex = state => Buffer.from(state.serialize()).toString('hex');
const gas = value => Object.fromEntries(Object.entries(value).map(([name, cost]) => [name, String(cost)]));
let witnessCalls = [], queries = [], events = [], mode = 'normal';
const unused = () => { throw new Error('unexpected witness'); };
const contract = new Contract({
 local_secret_key:({privateState:p})=>{witnessCalls.push('secret');if(mode==='secretFailure')throw new Error('secret witness refused');return [{...p,calls:p.calls+1},fill(4)];},
 fresh_nonce:({privateState:p})=>{witnessCalls.push('nonce');if(mode==='nonceFailure')throw new Error('nonce witness refused');return [{...p,calls:p.calls+1},mode==='nonceMalformed'?-1n:9n];},
 local_set_board:({privateState:p},board)=>{witnessCalls.push(['set',board]);if(mode==='setFailure')throw new Error('set witness refused');return [{...p,calls:p.calls+1,board},[]];},
 local_board:unused,
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
  const initial = contract.initialState({initialPrivateState:{calls:0},initialZswapLocalState:r.emptyZswapLocalState(key)});
  const state = initial.currentContractState;
  const fields = state.data.state.asArray();
  fields[5]=scalar(phase,options.phase??0);
  if((options.phase??0)===1)fields[6]=cell([bytes,bytes,u128,u64],[fill(1),fill(options.potColor??5),options.potValue??17n,0n]);
  state.data=new r.ChargedState(array(fields));
  const before=hex(state),beforeState=state.data;
  const context=r.createCircuitContext(r.dummyContractAddress(),options.missingKey?undefined:key,beforeState,{calls:0});
  context.currentZswapLocalState.currentIndex=2n;
  const wager={nonce:fill(3),color:fill(options.wagerColor??5),value:options.wagerValue??17n};
  const deposit={nonce:fill(2),color:fill(options.depositColor??0),value:options.depositValue??100000n};
  witnessCalls = []; queries = []; events = []; mode = options.mode ?? 'normal';
  try {
    const result = contract.circuits.start(context, options.pos??4n, wager, deposit);
    const captured = [...queries], calls = [...witnessCalls], intentEvents = [...events];
    state.data = new r.ChargedState(result.context.currentQueryContext.state.state);
    const replay = r.createCircuitContext(r.dummyContractAddress(), key, r.ContractState.deserialize(Buffer.from(before, "hex")).data, null);
    for (const [commitment, index] of result.context.currentQueryContext.comIndices) replay.currentQueryContext = replay.currentQueryContext.insertCommitment(commitment, index);
    let partition;
    try {
      const originalState = ledger.ContractState.deserialize(Buffer.from(before, 'hex'));
      let query = new ledger.QueryContext(originalState.data, r.dummyContractAddress());
      for (const [commitment, index] of result.context.currentQueryContext.comIndices) query = query.insertCommitment(commitment, index);
      const pair = ledger.partitionTranscripts([new ledger.PreTranscript(query, result.proofData.publicTranscript, undefined)], ledger.LedgerParameters.initialParameters())[0];
      partition = pair.map(transcript => transcript ? {programLength: transcript.program.length, effects: transcript.effects, gas: transcript.gas} : null);
    } catch(error) { partition = {error: String(error.message)}; }
    let replayProbe;
    try { replayProbe = {gas: gas(originalQuery.call(replay.currentQueryContext, captured.flatMap(q => q.ops), replay.costModel).gasCost)}; }
    catch(error) { replayProbe = {error: String(error.message)}; }
    const view = readLedger(state.data);
    const ledgerAfter = {pot:view.pot,red_deposit:view.red_deposit,blue_deposit:view.blue_deposit,red:view.red,blue:view.blue,red_board:view.red_board,blue_board:view.blue_board,state:view.state};
    return {name,options,before,ledgerAfter,after:hex(state),result:result.result,output:result.proofData.output,privateState:result.context.currentPrivateState,privateOutputs:result.proofData.privateTranscriptOutputs,effects:result.context.currentQueryContext.effects,queries:captured,publicTranscript:result.proofData.publicTranscript,witnessCalls:calls,events:intentEvents,plan:result.context.currentZswapLocalState,wrapperLastQueryGas:gas(result.gasCost),queryCostSum:Object.fromEntries(Object.keys(result.gasCost).map(k=>[k,String(captured.reduce((sum,q)=>sum+BigInt(q.gas[k]),0n))])),replayProbe,partition,provisionalComIndices:Array.from(result.context.currentQueryContext.comIndices)};
  } catch(error) {
    return {name,options,before,error:String(error.message),queries:[...queries],witnessCalls:[...witnessCalls],events:[...events]};
  }
}
const rows=[
 run('red'),run('blue',{phase:1}),run('redPos1',{pos:1n}),run('bluePos9',{phase:1,pos:9n}),
 run('redNoKey',{missingKey:true}),run('blueNoKey',{phase:1,missingKey:true}),
 run('phaseBlueStarted',{phase:2}),run('phaseRedTurn',{phase:3}),run('phaseBlueTurn',{phase:4}),run('phaseRedWins',{phase:5}),run('phaseBlueWins',{phase:6}),
 run('blueWrongWagerValue',{phase:1,wagerValue:16n}),run('blueWrongWagerColor',{phase:1,wagerColor:7}),run('badDepositValue',{depositValue:99999n}),run('badDepositColor',{depositColor:7}),
 run('badPos0',{pos:0n}),run('badPos10',{pos:10n}),run('redZeroWager',{wagerValue:0n}),run('blueZeroWager',{phase:1,wagerValue:0n,potValue:0n}),
 run('secretFailure',{mode:'secretFailure'}),run('nonceFailure',{mode:'nonceFailure'}),run('nonceMalformed',{mode:'nonceMalformed'}),run('setFailureRed',{mode:'setFailure'}),run('setFailureBlue',{phase:1,mode:'setFailure'}),
 run('blueMergeOverflow',{phase:1,wagerValue:max,potValue:max}),run('wagerBeforeDeposit',{phase:1,wagerValue:16n,depositValue:0n}),run('depositBeforePos',{depositValue:0n,pos:0n}),
];
const hash = p => createHash('sha256').update(readFileSync(p)).digest('hex');
const provenance={source:'test-center/test-contracts/coracle.compact',sourceSha256:hash(sourcePath),generatedIndexSha256:hash(path),runtimeVersion:JSON.parse(readFileSync(`${runtimeRoot}/package.json`,'utf8')).version,runtimeBuiltinsSha256:hash(`${runtimeRoot}/dist/built-ins.js`),runtimeTypesSha256:hash(`${runtimeRoot}/dist/compact-types.js`),ledgerModuleSha256:hash(ledgerPath),descriptor:r.ShieldedCoinInfoDescriptor.alignment(),scope:'unchanged original source; explicitly seeded prior red/blue states, no funded lifecycle or proof claim'};
process.stdout.write(JSON.stringify({provenance,rows},(_,value)=>value instanceof Uint8Array?Array.from(value):typeof value==='bigint'?String(value):value instanceof Map?Object.fromEntries(value):value,2)+'\n');
