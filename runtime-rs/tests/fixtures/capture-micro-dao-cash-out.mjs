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
// Original source with explicit seeded state; no funded DAO lifecycle claim.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const resolved=createRequire(path).resolve('@midnight-ntwrk/compact-runtime');
const r=await import(pathToFileURL(resolved));
const {Contract,ledger:readLedger}=await import(pathToFileURL(path));
const max=(1n<<64n)-1n, max128=(1n<<128n)-1n;
const bytes=new r.CompactTypeBytes(32),u64=new r.CompactTypeUnsignedInteger(max,8),u128=new r.CompactTypeUnsignedInteger(max128,16),phase=new r.CompactTypeEnum(3,1),bool=r.CompactTypeBoolean,str=r.CompactTypeOpaqueString;
const fill=n=>new Uint8Array(32).fill(n),key={bytes:fill(7)};
const cell=(types,values)=>r.StateValue.newCell({value:types.flatMap((t,i)=>t.toValue(values[i])),alignment:types.flatMap(t=>t.alignment())});
const scalar=(t,v)=>cell([t],[v]);
const array=values=>values.reduce((a,v)=>a.arrayPush(v),r.StateValue.newArray());
const hex=state=>Buffer.from(state.serialize()).toString('hex');
const gas=v=>Object.fromEntries(Object.entries(v).map(([k,x])=>[k,String(x)]));
let calls=[],queries=[],events=[],partial,activeContext;
const unused=()=>{calls.push('unexpected');throw new Error('unexpected declared witness')};
const c=new Contract({local_secret_key:unused,local_state:unused,local_vote_cast:unused,local_advance_state:unused,local_record_vote:unused,local_path_of_cm:unused});
for(const [method,kind]of[['_createZswapInput_0','input'],['_createZswapOutput_0','output'],['_ownPublicKey_0','own-key']]){
 const original=c[method];c[method]=function(...args){const result=original.apply(this,args);events.push({kind,...(kind==='own-key'?{key:result}:{coin:args[2],...(kind==='output'?{recipient:args[3]}:{})})});return result;};
}
const originalCashOut=c._cash_out_0;c._cash_out_0=function(context,proof){partial=proof;activeContext=context;return originalCashOut.call(this,context,proof)};
const originalQuery=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){const out=originalQuery.call(this,...args);queries.push({ops:args[0],gas:gas(out.gasCost)});events.push({kind:'query',index:queries.length-1});return out;};
function seed(o){
 const state=c.initialState({initialPrivateState:{calls:0},initialZswapLocalState:r.emptyZswapLocalState(key)},fill(4),{seed_dust:10n,buy_in_dust:3n}).currentContractState;
 const f=state.data.state.asArray();f[1]=scalar(phase,o.phase??3);f[2]=cell([bool,str],[!o.emptyTopic,o.emptyTopic?'':(o.longTopic?'x'.repeat(4096):'Proposal 🗳️')]);f[3]=cell([bool,bytes],[!o.absent,fill(o.wrong?8:7)]);f[4]=scalar(u64,o.yes??4n);f[5]=scalar(u64,o.no??3n);f[6]=scalar(u64,o.round??7n);f[10]=cell([bytes,bytes,u128,u64],[fill(41),fill(42),o.value??99n,o.index??0n]);f[11]=scalar(bool,o.potFlag??true);
 if(!o.emptyCollections){let tree=f[7].asArray()[0].asBoundedMerkleTree();for(const[i,n]of[11,12].entries())tree=tree.update(BigInt(i),{value:bytes.toValue(fill(n)),alignment:bytes.alignment()});f[7]=array([r.StateValue.newBoundedMerkleTree(tree.rehash()),scalar(u64,2n)]);for(const i of[8,9]){let map=new r.StateMap();for(const n of[21,22])map=map.insert({value:bytes.toValue(fill(n)),alignment:bytes.alignment()},r.StateValue.newNull());f[i]=r.StateValue.newMap(map);}}
 state.data=new r.ChargedState(array(f));return state;
}
function view(data){const v=readLedger(data);return Object.fromEntries(['organizer','state','topic','beneficiary','yes','no','round','pot','pot_has_coin','costs'].map(k=>[k,v[k]]));}
function run(name,o={}){
 const state=seed(o),before=hex(state),beforeState=state.data;
 const ctx=r.createCircuitContext(r.dummyContractAddress(),o.missingKey?undefined:key,beforeState,{calls:0});ctx.currentZswapLocalState.currentIndex=2n;
 if(o.gasLimit)ctx.gasLimit=o.gasLimit;
 calls=[];queries=[];events=[];partial=undefined;activeContext=undefined;
 try{const out=c.circuits.cash_out(ctx);const capturedQueries=[...queries],capturedEvents=[...events],capturedCalls=[...calls];state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
  return{name,options:o,before,after:hex(state),beforeView:view(beforeState),afterView:view(state.data),result:out.result,output:out.proofData.output,privateState:out.context.currentPrivateState,privateOutputs:out.proofData.privateTranscriptOutputs,queries:capturedQueries,events:capturedEvents,declaredWitnessCalls:capturedCalls,plan:out.context.currentZswapLocalState,indices:Array.from(out.context.currentQueryContext.comIndices),effects:out.context.currentQueryContext.effects,publicTranscript:out.proofData.publicTranscript,wrapperGas:gas(out.gasCost),queryCostSum:Object.fromEntries(Object.keys(out.gasCost).map(k=>[k,String(capturedQueries.reduce((sum,q)=>sum+BigInt(q.gas[k]),0n))]))};
 }catch(error){const capturedQueries=[...queries],capturedEvents=[...events],capturedCalls=[...calls];return{name,options:o,before,error:String(error.message),queries:capturedQueries,events:capturedEvents,declaredWitnessCalls:capturedCalls,privateOutputs:partial?.privateTranscriptOutputs,callerView:view(ctx.currentQueryContext.state),prefixView:view(activeContext.currentQueryContext.state),prefixPlan:activeContext.currentZswapLocalState};}
}
const rows=[run('eligible'),run('emptyCollections',{emptyCollections:true,emptyTopic:true}),run('longTopic',{longTopic:true}),run('potFlagFalse',{potFlag:false}),run('zeroPot',{value:0n}),run('maxPot',{value:max128}),run('roundMax',{round:max}),run('roundBeforeMax',{round:max-1n}),run('zeroNo',{yes:1n,no:0n}),run('yesMax',{yes:max,no:max-1n}),...[0,1,2].map(phase=>run('phase'+phase,{phase})),run('absent',{absent:true}),run('wrongBeneficiary',{wrong:true}),run('tie',{yes:3n,no:3n}),run('noMajority',{yes:2n,no:3n}),run('bothMax',{yes:max,no:max}),run('missingKey',{missingKey:true}),run('missingKeyWrongPhase',{missingKey:true,phase:0}),run('zeroGas',{gasLimit:{readTime:0n,computeTime:0n,bytesWritten:0n,bytesDeleted:0n}})];
process.stdout.write(JSON.stringify(rows,(_,v)=>v instanceof Uint8Array?Array.from(v):typeof v==='bigint'?String(v):v,2)+'\n');
