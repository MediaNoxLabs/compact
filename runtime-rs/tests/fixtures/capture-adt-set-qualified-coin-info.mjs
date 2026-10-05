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

// Capture original ADT Set source. The second case explicitly uses a mock
// commitment-index context because both source coins require index zero.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';
const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected generated contract/index.js');
const {Contract}=await import(pathToFileURL(contractPath).href);
const contract=new Contract({}); const key={bytes:new Uint8Array(32)}; const address=runtime.dummyContractAddress();
const initial=contract.initialState({initialPrivateState:null,initialZswapLocalState:runtime.emptyZswapLocalState(key)});
const b=s=>Buffer.from(s.serialize()).toString('hex'); const gas=c=>Object.fromEntries(Object.entries(c).map(([k,v])=>[k,v.toString()]));
const log=[]; const query=runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query=function(...args){let r=query.call(this,...args);log.push({opTags:args[0].map(x=>typeof x==='string'?x:Object.keys(x)[0]),gasCost:gas(r.gasCost)});return r};
// The two source coins both assert mt_index=0. Reusing index 0 here is
// a source-level mock context for parity, not a ledger-valid transaction.
function invoke(name,allocate){const current=contract.initialState({initialPrivateState:null,initialZswapLocalState:runtime.emptyZswapLocalState(key)});const context=runtime.createCircuitContext(address,key,current.currentContractState.data,current.currentPrivateState); if(allocate){const bytes=s=>Uint8Array.from([...Buffer.from(s),...Array(32-s.length).fill(0)]);const recipient={is_left:false,left:{bytes:new Uint8Array(32)},right:{bytes:runtime.encodeContractAddress(address)}};for(let value of [1n,2n]){context.currentZswapLocalState.currentIndex=0n;runtime.createZswapOutput(context,{nonce:bytes('nonce'),color:bytes('color'),value},recipient)}}
const start=log.length;
try{let out=contract.circuits[name](context);current.currentContractState.data=new runtime.ChargedState(out.context.currentQueryContext.state.state);return {name,allocate,initialStateHex:b(initial.currentContractState),afterStateHex:b(current.currentContractState),gas:gas(out.gasCost),effects:out.context.currentQueryContext.effects,queryLog:log.slice(start),privateTranscriptCount:out.proofData.privateTranscriptOutputs.length,publicTags:out.proofData.publicTranscript.map(x=>typeof x==='string'?x:Object.keys(x)[0])}}catch(e){return{name,allocate,error:String(e),queryLog:log.slice(start)}}}
console.log(JSON.stringify([invoke('test_QualifiedShieldedCoinInfo',false),invoke('test_ShieldedCoinInfo',false),invoke('test_ShieldedCoinInfo',true)],null,2));
