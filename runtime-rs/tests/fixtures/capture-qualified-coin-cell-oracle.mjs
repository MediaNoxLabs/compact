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

// Independent TS Cell.writeCoin oracle. Allocation is supplied through the
// runtime's createZswapOutput; it is never guessed from the coin itself.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [contractPath] = process.argv.slice(2);
const require=createRequire(contractPath);
const runtime=await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const {Contract}=await import(pathToFileURL(contractPath));
const contract=new Contract({});
const key={bytes:new Uint8Array(32)};
const address=runtime.dummyContractAddress();
const hex=x=>Buffer.from(x).toString('hex');
const coin=(second=false)=>({nonce:Uint8Array.from([110,111,110,99,101,second?50:0,...Array(26).fill(0)]),color:Uint8Array.from([99,111,108,111,114,...Array(27).fill(0)]),value:second?43n:42n});
const recipient=left=>({is_left:left,left:{bytes:Uint8Array.from([left?7:0,...Array(31).fill(0)])},right:{bytes:runtime.encodeContractAddress(address)}});
const gas=x=>Object.fromEntries(Object.entries(x).map(([k,v])=>[k,v.toString()]));
const queries=[];let queryAttempts=0;
const original=runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query=function(...args){queryAttempts+=1;const result=original.call(this,...args);queries.push({ops:args[0],gasCost:gas(result.gasCost)});return result;};
function run(left,index,allocate=true,occupied=false,wrong=false,malformed=false){
 const initial=contract.initialState({initialPrivateState:null,initialZswapLocalState:runtime.emptyZswapLocalState(key)});
 const initialStateHex=hex(initial.currentContractState.serialize());
 let deployed=runtime.ContractState.deserialize(initial.currentContractState.serialize());
 if(occupied){
  const prior=runtime.createCircuitContext(address,key,deployed.data,null);
  prior.currentZswapLocalState.currentIndex=7n;runtime.createZswapOutput(prior,coin(),recipient(false));
  const result=contract.circuits.write_coin(prior,coin(),recipient(false));
  deployed.data=new runtime.ChargedState(result.context.currentQueryContext.state.state);
 }
 if(malformed) deployed.data=new runtime.ChargedState(runtime.StateValue.newCell({value:[new Uint8Array()],alignment:[{tag:"atom",value:{tag:"bytes",length:1}}]}));
 const beforeStateHex=hex(deployed.serialize());
 const context=runtime.createCircuitContext(address,key,deployed.data,null);
 const value=coin(occupied),target=recipient(left);
 if(allocate){context.currentZswapLocalState.currentIndex=BigInt(index);runtime.createZswapOutput(context,value,wrong?recipient(!left):target);}
 queries.length=0;queryAttempts=0;
 try{
  const result=contract.circuits.write_coin(context,value,target);
  const writeQueries=[...queries];
  const output=contract.circuits.read_coin(result.context);
  deployed.data=new runtime.ChargedState(result.context.currentQueryContext.state.state);
  return {left,index,occupied,initialStateHex,beforeStateHex,afterStateHex:hex(deployed.serialize()),effects:result.context.currentQueryContext.effects,
    writeGas:gas(result.gasCost),writeQueries,publicTranscript:result.proofData.publicTranscript,privateCount:result.proofData.privateTranscriptOutputs.length,
    read:{nonce:hex(output.result.nonce),color:hex(output.result.color),value:output.result.value.toString(),mt_index:output.result.mt_index.toString()}};
 }catch(error){return{left,index,occupied,initialStateHex,beforeStateHex,error:error.message,queryAttempts,queries:[...queries]};}
}
process.stdout.write(JSON.stringify({right7:run(false,7),left11:run(true,11),zero:run(false,0),replacement:run(true,17,true,true),missing:run(false,7,false),wrongRecipient:run(true,7,true,false,true),malformed:run(false,7,true,false,false,true)},(_,x)=>x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?x.toString():x,2)+'\n');
