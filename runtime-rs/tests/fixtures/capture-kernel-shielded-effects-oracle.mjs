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

// Independent original TypeScript execution of Kernel effects.
import {createRequire} from 'node:module';import {pathToFileURL} from 'node:url';
const [path]=process.argv.slice(2),require=createRequire(path);
const r=await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const {Contract}=await import(pathToFileURL(path));
const key={bytes:new Uint8Array(32)},addr=r.dummyContractAddress();
const bytes=n=>Uint8Array.from([n,...Array(31).fill(0)]),gas=v=>Object.fromEntries(Object.entries(v).map(([k,x])=>[k,String(x)]));
const queries=[];let attempts=0;const original=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){attempts++;const out=original.call(this,...args);queries.push({ops:args[0],gas:gas(out.gasCost)});return out;};
function run(name,args,label){
 const c=new Contract({next_domain:ctx=>[[...ctx.privateState,'domain'],bytes(3)],next_amount:ctx=>[[...ctx.privateState,'amount'],13n]});
 const init=c.initialState({initialPrivateState:[],initialZswapLocalState:r.emptyZswapLocalState(key)});
 const ctx=r.createCircuitContext(addr,key,init.currentContractState.data,[]);
 const before=Buffer.from(init.currentContractState.serialize()).toString('hex');queries.length=0;attempts=0;
 try{const out=c.circuits[name](ctx,...args);const state=r.ContractState.deserialize(init.currentContractState.serialize());state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
 return {name,label,args,before,after:Buffer.from(state.serialize()).toString('hex'),effects:out.context.currentQueryContext.effects,privateState:out.context.currentPrivateState,privateOutputs:out.proofData.privateTranscriptOutputs,queries:[...queries],transcript:out.proofData.publicTranscript,gas:gas(out.gasCost)};
 }catch(e){return{name,label,args,before,error:e.message,queries:[...queries],attempts};}
}
const rows=[run('mint',[bytes(1),0n],'zero'),run('mint',[bytes(1),7n],'seven'),run('mint',[bytes(1),(1n<<64n)-1n],'max'),run('nullifier',[bytes(2)],'nullifier'),run('spend',[bytes(2)],'spend'),run('claim_receive',[bytes(2)],'receive'),run('batch',[bytes(1),bytes(1),7n,11n],'same'),run('batch',[bytes(1),bytes(2),7n,11n],'different'),run('batch',[bytes(1),bytes(1),(1n<<64n)-1n,1n],'overflow'),run('selected',[false,bytes(2)],'false'),run('selected',[true,bytes(2)],'true'),run('witness_order',[],'witness')];
process.stdout.write(JSON.stringify(rows,(_,x)=>x instanceof Map?Object.fromEntries(x):x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
