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

import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {normalizeQueryProgram} from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';
const [contractPath]=process.argv.slice(2);
const runtime=await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const {Contract}=await import(pathToFileURL(contractPath));
const bytes=new runtime.CompactTypeBytes(32),vector=new runtime.CompactTypeVector(4,bytes),pathType=new runtime.CompactTypeMerkleTreePath(32,bytes);
const fill=n=>new Uint8Array(32).fill(n),coin=n=>({nonce:{bytes:fill(n)},opening:{bytes:fill(n+1)}}),pk={bytes:runtime.persistentHash(bytes,fill(1))};
const prefix=new Uint8Array(32);prefix.set(new TextEncoder().encode('lares:zerocash:commit'));
const cm=(coin,key=pk)=>({bytes:runtime.persistentHash(vector,[prefix,coin.nonce.bytes,coin.opening.bytes,key.bytes])});
const destination={zk:{bytes:fill(9)},encryption:Uint8Array.from([10,11,12])};
let queries=[],raw=[],calls=[],snapshot,oldPath,mode='normal';
const step=(name,privateState,patch={})=>{calls.push(name);return {...privateState,...patch,calls:privateState.calls+1};};
const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
function pathOf(state,commitment){const tree=state.asArray()[1].asArray()[0].asBoundedMerkleTree();const path=tree.findPathForLeaf({value:bytes.toValue(commitment.bytes),alignment:bytes.alignment()});const value=pathType.fromValue(path.value);return {...value,leaf:{bytes:value.leaf}};}
const contract=new Contract({
 private$zk_secret_key:({privateState})=>[step('secret',privateState),{bytes:fill(1)}],
 private$zk_public_key:({privateState})=>[step('public_key',privateState),pk],
 private$add_coin:({privateState},value)=>{if(!equal(value,coin(privateState.next-2)))throw Error('wrong added coin');return[step('add_coin',privateState,{owned:privateState.owned+1}),[]];},
 private$remove_coin:({privateState},value)=>{if(!equal(value,coin(3)))throw Error('wrong removed coin');return[step('remove_coin',privateState,{owned:privateState.owned-1}),[]];},
 context$new_coin_info:({privateState})=>[step('new_coin',privateState,{next:privateState.next+2}),coin(privateState.next)],
 context$path_of:({privateState},commitment)=>{
  if(!equal(commitment,cm(coin(3))))throw Error('wrong path argument');
  const next=step('path',privateState);
  let value=oldPath??pathOf(snapshot,mode==='wrong_leaf'?cm(coin(5)):commitment);
  value={leaf:value.leaf,path:value.path.map(entry=>({sibling:{...entry.sibling},goes_left:entry.goes_left}))};
  if(mode==='wrong_root')value.path[0].sibling.field+=1n;
  if(mode==='malformed')value.path.pop();
  return[next,value];
 },
 context$encrypt:({privateState},key,value)=>{
  const next=step('encrypt',privateState);
  if(!equal(key,destination.encryption)||!equal(value,coin(privateState.next-2)))throw Error('wrong encryption arguments');
  if(mode==='encryption_failure')throw Error('failed assert: encryption rejected');
  return[next,Uint8Array.from([...key,value.nonce.bytes[0],value.opening.bytes[0],255])];
 }
});
const coinPublicKey={bytes:new Uint8Array(32)};
const initial=()=>contract.initialState({initialPrivateState:{calls:0,next:3,owned:0},initialZswapLocalState:runtime.emptyZswapLocalState(coinPublicKey)});
const context=state=>runtime.createCircuitContext(runtime.dummyContractAddress(),coinPublicKey,state.currentContractState.data,state.currentPrivateState);
const original=runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query=function(...args){const output=original.call(this,...args);queries.push({program:normalizeQueryProgram(args[0]),gasCost:output.gasCost});raw.push(args[0]);return output;};
function update(state,result){const next=runtime.ContractState.deserialize(state.currentContractState.serialize());next.data=new runtime.ChargedState(result.context.currentQueryContext.state.state);return{currentContractState:next,currentPrivateState:result.context.currentPrivateState};}
function seeded(historical){mode='normal';oldPath=undefined;let state=initial();state=update(state,contract.circuits.zerocash_mint(context(state)));const path=pathOf(state.currentContractState.data.state,cm(coin(3)));if(historical)state=update(state,contract.circuits.zerocash_mint(context(state)));state.currentPrivateState.calls=0;return{state,path};}
function spend(state){snapshot=state.currentContractState.data.state;return contract.circuits.spend(context(state),destination,coin(3));}
const scenarios=[];
for(const historical of [false,true]){
 const {state,path}=seeded(historical);oldPath=historical?path:undefined;queries=[];raw=[];calls=[];
 const result=spend(state),after=update(state,result),captured=queries.slice(),witnessCalls=calls.slice();
 const replay=context(state).currentQueryContext.query(raw.flat(),context(state).costModel);
 scenarios.push({historical,stateHex:Buffer.from(after.currentContractState.serialize()).toString('hex'),privateState:after.currentPrivateState,witnessCalls,
 privateTranscriptOutputs:result.proofData.privateTranscriptOutputs,queries:captured,gasCost:result.gasCost,replayGas:replay.gasCost});
}
const failures={};
for(const name of ['duplicate','wrong_root','wrong_leaf','malformed','encryption_failure']){
 const {state:seed}=seeded(name==='wrong_leaf');let state=seed;
 if(name==='duplicate')state=update(state,spend(state));
 mode=name;oldPath=undefined;queries=[];raw=[];calls=[];
 try{spend(state);throw Error('unexpected success');}catch(error){failures[name]={error:error.message,witnessCalls:calls.slice(),queries:queries.slice()};}
}
process.stdout.write(JSON.stringify({scenarios,failures},(_key,value)=>typeof value==='bigint'?value.toString():value instanceof Uint8Array?Array.from(value):value,2)+'\n');
