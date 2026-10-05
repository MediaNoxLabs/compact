// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Original source; explicit prior-state fixtures do not claim funded setup.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const r=await import(pathToFileURL(createRequire(path).resolve('@midnight-ntwrk/compact-runtime')));
const {Contract}=await import(pathToFileURL(path));
const bytes=new r.CompactTypeBytes(32), u64=new r.CompactTypeUnsignedInteger((1n<<64n)-1n,8), phase=new r.CompactTypeEnum(3,1);
const pathType=new r.CompactTypeMerkleTreePath(10,bytes), vector=new r.CompactTypeVector(3,bytes);
const key={bytes:new Uint8Array(32)}, secret=new Uint8Array(32).fill(7), addr=r.dummyContractAddress();
const pad=s=>{const b=new Uint8Array(32);b.set(new TextEncoder().encode(s));return b;};
const commitment=(vote,round)=>r.persistentHash(vector,[pad(vote?'yes':'no'),r.convertFieldToBytes(32,round,'seed round'),secret]);
const cell=(type,value)=>r.StateValue.newCell({value:type.toValue(value),alignment:type.alignment()});
const array=values=>values.reduce((a,v)=>a.arrayPush(v),r.StateValue.newArray());
const hex=s=>Buffer.from(s.serialize()).toString('hex');
const gas=v=>Object.fromEntries(Object.entries(v).map(([k,x])=>[k,String(x)]));
let calls=[],pathArgs=[],snapshot,mode='normal',currentRound=0n,currentVote=true,queries=[];
const bump=(p,extra={})=>({...p,calls:p.calls+1,...extra});
const c=new Contract({
 local_state:({privateState:p})=>{calls.push('state');return[bump(p),p.phase];},
 local_secret_key:({privateState:p})=>{calls.push('secret');return[bump(p),secret];},
 local_vote_cast:({privateState:p})=>{calls.push('vote');return[bump(p),{is_some:p.vote!==null,value:p.vote??false}];},
 local_advance_state:({privateState:p})=>{calls.push('advance');return[bump(p,{phase:2}),[]];},
 local_record_vote:()=>{throw new Error('reveal must not record ballot');},
 local_path_of_cm:({privateState:p},cm)=>{
  calls.push('path');pathArgs.push(cm);
  const tree=snapshot.asArray()[7].asArray()[0].asBoundedMerkleTree();
  const key=mode==='wrong_leaf'?commitment(!currentVote,currentRound):cm;
  const found=tree.findPathForLeaf({value:bytes.toValue(key),alignment:bytes.alignment()});
  const value=found===undefined?{leaf:new Uint8Array(32),path:Array.from({length:10},()=>({sibling:{field:0n},goes_left:false}))}:pathType.fromValue(found.value);
  if(mode==='wrong_root')value.path[0].sibling.field+=1n;
  if(mode==='malformed')value.path.pop();
  return[bump(p),{is_some:found!==undefined,value}];
 }
});
const original=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){const out=original.call(this,...args);queries.push({ops:args[0],gas:gas(out.gasCost)});return out;};
function seeded(vote,round,options={}) {
 const init=c.initialState({initialPrivateState:{phase:1,vote,calls:0},initialZswapLocalState:r.emptyZswapLocalState(key)},new Uint8Array(32).fill(4),{seed_dust:10n,buy_in_dust:3n});
 const fields=init.currentContractState.data.state.asArray();fields[1]=cell(phase,options.wrongPhase?1:2);fields[6]=cell(u64,round);
 if(!options.noTree){let tree=fields[7].asArray()[0].asBoundedMerkleTree();const leafRound=options.crossRound?round-1n:round;
  for(const [i,v]of[vote,!vote].entries())tree=tree.update(BigInt(i),{value:bytes.toValue(commitment(v,leafRound)),alignment:bytes.alignment()});
  fields[7]=array([r.StateValue.newBoundedMerkleTree(tree.rehash()),cell(u64,2n)]);
 }
 init.currentContractState.data=new r.ChargedState(array(fields));
 const ctx=r.createCircuitContext(addr,key,init.currentContractState.data,{phase:options.wrongPrivate?0:1,vote:options.noVote?null:vote,calls:0});
 return{state:init.currentContractState,ctx};
}
function invoke(ctx){snapshot=ctx.currentQueryContext.state.state;return c.circuits.vote_reveal(ctx);}
function run(name,vote=true,round=7n,options={}) {
 currentRound=round;currentVote=vote;mode='normal';let{state,ctx}=seeded(vote,round,options);
 if(options.repeat||options.duplicate){ctx=invoke(ctx).context;if(options.duplicate)ctx.currentPrivateState={phase:1,vote,calls:0};state.data=new r.ChargedState(ctx.currentQueryContext.state.state);}
 const before=hex(state), beforeState=state.data;
 if(options.gasLimit)ctx.gasLimit=options.gasLimit;
 calls=[];pathArgs=[];queries=[];mode=options.mode??'normal';
 try {const out=invoke(ctx);const q=[...queries], witnessCalls=[...calls];state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
  const replay=r.createCircuitContext(addr,key,beforeState,null);const replayGas=gas(original.call(replay.currentQueryContext,q.flatMap(x=>x.ops),replay.costModel).gasCost);
  return{name,vote,round,options,before,after:hex(state),result:out.result,output:out.proofData.output,privateState:out.context.currentPrivateState,privateOutputs:out.proofData.privateTranscriptOutputs,effects:out.context.currentQueryContext.effects,queries:q,reportedGas:gas(out.gasCost),queryCostSum:Object.fromEntries(Object.keys(out.gasCost).map(k=>[k,String(q.reduce((sum,x)=>sum+BigInt(x.gas[k]),0n))])),replayGas,publicTranscript:out.proofData.publicTranscript,witnessCalls,pathArgs};
 }catch(error){return{name,vote,round,options,before,error:error.message,witnessCalls:[...calls],pathArgs:[...pathArgs],queries:[...queries]};}
}
const rows=[run('yes0',true,0n),run('no0',false,0n),run('yes7'),run('no7',false),run('yesMax',true,(1n<<64n)-1n),run('wrongPhase',true,7n,{wrongPhase:true}),run('wrongPrivate',true,7n,{wrongPrivate:true}),run('noVote',true,7n,{noVote:true}),run('missingPath',true,7n,{noTree:true}),run('wrongRoot',true,7n,{mode:'wrong_root'}),run('wrongLeaf',true,7n,{mode:'wrong_leaf'}),run('crossRound',true,7n,{crossRound:true}),run('malformed',true,7n,{mode:'malformed'}),run('repeat',true,7n,{repeat:true}),run('duplicate',true,7n,{duplicate:true}),run('zeroGas',true,7n,{gasLimit:{readTime:0n,computeTime:0n,bytesWritten:0n,bytesDeleted:0n}})];
const budget=Object.fromEntries(Object.entries(rows[2].queries[0].gas).map(([k,v])=>[k,BigInt(v)]));rows.push(run('prefixGas',true,7n,{gasLimit:budget}));
process.stdout.write(JSON.stringify(rows,(_,x)=>x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
