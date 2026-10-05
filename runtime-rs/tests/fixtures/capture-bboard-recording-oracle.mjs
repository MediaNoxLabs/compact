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

// Fresh original-source capture. Rehydrate ledger state per call, retain private state.
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {normalizeQueryProgram} from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';
const [contractPath]=process.argv.slice(2);
const runtime=await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const {Contract}=await import(pathToFileURL(contractPath));
let key=7, calls=[], queries=[], raw=[];
const contract=new Contract({local_secret_key:({privateState})=>{calls.push(privateState);return [privateState+1,new Uint8Array(32).fill(key)];}});
const coinPublicKey={bytes:new Uint8Array(32)};
const initial=()=>contract.initialState({initialPrivateState:5,initialZswapLocalState:runtime.emptyZswapLocalState(coinPublicKey)});
const original=runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query=function(...args){const result=original.call(this,...args);queries.push({program:normalizeQueryProgram(args[0]),gasCost:result.gasCost});raw.push(args[0]);return result;};
const context=(state)=>runtime.createCircuitContext(runtime.dummyContractAddress(),coinPublicKey,state.currentContractState.data,state.currentPrivateState);
const hex=(state)=>Buffer.from(state.serialize()).toString('hex');
function update(state,result){const next=runtime.ContractState.deserialize(state.currentContractState.serialize());next.data=new runtime.ChargedState(result.context.currentQueryContext.state.state);return {currentContractState:next,currentPrivateState:result.context.currentPrivateState};}
function invoke(state,method,message){const ctx=context(state);return method==='post'?contract.circuits.post(ctx,message):contract.circuits.take_down(ctx);}
function capture(state,method,message){
 queries=[];raw=[];calls=[];
 const result=invoke(state,method,message), next=update(state,result);
 const captured=queries.slice(), witnessCalls=calls.slice(), program=raw.flat();
 const replay=context(state).currentQueryContext.query(program,context(state).costModel);
 return {state:next, evidence:{method,message,result:result.result,stateHex:hex(next.currentContractState),privateState:next.currentPrivateState,witnessCalls,
 privateTranscriptOutputs:result.proofData.privateTranscriptOutputs,gasCost:result.gasCost,queries:captured,replayGas:replay.gasCost}};
}
let state=initial();const scenarios=[];
for(const message of ['', '🌙 Midnight — 你好, Привіт']){
 let result=capture(state,'post',message);scenarios.push(result.evidence);state=result.state;
 result=capture(state,'take_down');scenarios.push(result.evidence);state=result.state;
}
function reject(state,method,secret=7){key=secret;queries=[];raw=[];calls=[];try{invoke(state,method,'blocked');throw new Error('unexpected success');}catch(error){return {error:error.message,witnessCalls:calls.slice(),queries:queries.slice()};}finally{key=7;}}
const empty=initial(), posted=update(empty,invoke(empty,'post','owner'));
const emptyTakeDown=reject(empty,'take_down');
const occupiedPost=reject(posted,'post');
const wrongOwner=reject(posted,'take_down',8);
// Counter has advanced, but forge the old poster key on an otherwise posted board.
const after=update(posted,invoke(posted,'take_down'));
const newPosted=update(after,invoke(after,'post','next instance'));
const oldPoster=posted.currentContractState.data.state.asArray()[3];
const parts=newPosted.currentContractState.data.state.asArray();parts[3]=oldPoster;
newPosted.currentContractState.data=new runtime.ChargedState(parts.reduce((array,value)=>array.arrayPush(value),runtime.StateValue.newArray()));
const oldInstance=reject(newPosted,'take_down');
process.stdout.write(JSON.stringify({scenarios,emptyTakeDown,occupiedPost,wrongOwner,oldInstance},(_key,value)=>typeof value==='bigint'?value.toString():value instanceof Uint8Array?Array.from(value):value,2)+'\n');
