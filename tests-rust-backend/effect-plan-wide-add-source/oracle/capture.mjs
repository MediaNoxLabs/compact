import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,symlinkSync,existsSync} from 'node:fs';
const out='/tmp/rust040-wide-add';
const deps='/tmp/compact-adr234/registry/receive/contract/node_modules';
if(!existsSync(out+'/ts-source/contract/node_modules'))symlinkSync(deps,out+'/ts-source/contract/node_modules');
const r=await import(deps+'/@midnight-ntwrk/compact-runtime/dist/index.js');
const {Contract,ledger}=await import('./ts-source/contract/index.js');
// Preserve official wrapper gas while separately observing every executed query.
// Generated witness ledger views have their own QueryContext/cost accumulator.
const queryCosts=[];
const originalQuery=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){
 const result=originalQuery.call(this,...args);
 queryCosts.push({gasCost:result.gasCost,opTags:args[0].map(op=>typeof op==='string'?op:Object.keys(op)[0])});
 return result;
};
const queryCostSum=queries=>Object.fromEntries(['readTime','computeTime','bytesWritten','bytesDeleted'].map(key=>[key,queries.reduce((sum,q)=>sum+q.gasCost[key],0n).toString()]));
const clean=v=>JSON.parse(JSON.stringify(v,(_,x)=>typeof x==='bigint'?x.toString():x instanceof Uint8Array?[...x]:x));
const raw=x=>{const a=[];while(x>0n){a.push(Number(x&255n));x>>=8n;}return a};
const atoms=x=>x.value.map(a=>[...a]);
const align=n=>[{tag:'atom',value:{tag:'bytes',length:n}}];
const field=[{tag:'atom',value:{tag:'field'}}];
const rows=[];
function execute(circuit,args,left=0n,right=0n,fail=null){
 const events=[];
 const witness=name=>ctx=>{events.push([name,ctx.privateState,ctx.ledger.calls]);if(fail===name)throw Error(name+' witness refused');return[ctx.privateState+1,name==='left'?left:right]};
 const contract=new Contract({left:witness('left'),right:witness('right')});
 const key={bytes:new Uint8Array(32)};const initial=contract.initialState({initialPrivateState:5,initialZswapLocalState:r.emptyZswapLocalState(key)});
 const context=r.createCircuitContext(r.dummyContractAddress(),key,initial.currentContractState.data,5);
 const start=queryCosts.length;
 try{return{call:contract.circuits[circuit](context,...args),events,queries:queryCosts.slice(start)}}catch(error){return{error:String(error),events,queries:queryCosts.slice(start)}}
}
function success(circuit,args,input,output,inAlign,outAlign,expected,left=0n,right=0n,events=[]){
 const observed=execute(circuit,args,left,right);assert.equal(observed.error,undefined);
 const {call}=observed;assert.deepEqual(call.gasCost,observed.queries[0].gasCost);assert.equal(observed.queries.length,1+events.length);assert.deepEqual(observed.queries[0].opTags,['idx','addi','ins']);for(const query of observed.queries.slice(1))assert.deepEqual(query.opTags,['dup','idx','popeq']);assert.deepEqual(call.result,expected);assert.deepEqual(observed.events,events);assert.equal(ledger(call.context.currentQueryContext.state).calls,1n);
 assert.deepEqual(atoms(call.proofData.input),input);assert.deepEqual(atoms(call.proofData.output),output);assert.deepEqual(call.proofData.input.alignment,inAlign);assert.deepEqual(call.proofData.output.alignment,outAlign);
 assert.ok(call.proofData.publicTranscript.length>0);
 const encoded=r.proofDataIntoSerializedPreimage(call.proofData.input,call.proofData.output,call.proofData.publicTranscript,call.proofData.privateTranscriptOutputs,circuit);
 rows.push({circuit,args:clean(args),left:String(left),right:String(right),status:'success',result:clean(call.result),events:clean(events),privateState:call.context.currentPrivateState,proofData:clean(call.proofData),gasCost:clean(call.gasCost),queries:clean(observed.queries),queryCostSum:queryCostSum(observed.queries),serializedBytes:encoded.length});
}
function failure(circuit,args,left=0n,right=0n,events=[],fail=null){const actual=execute(circuit,args,left,right,fail);assert.match(actual.error,fail?new RegExp(fail+' witness refused'):/cast from Field or Uint value to smaller Uint value failed/);assert.deepEqual(actual.events,events);rows.push({circuit,args:clean(args),left:String(left),right:String(right),fail,status:'refused',error:actual.error,events:clean(events),queries:clean(actual.queries),queryCostSum:queryCostSum(actual.queries)})}
const max128=(1n<<128n)-1n,max129=(1n<<129n)-1n;
for(const[a,b]of[[0n,0n],[max128,0n],[max128,1n],[max128,max128]])success('add128',[a,b],[raw(a),raw(b)],[raw(a+b)],[...align(16),...align(16)],align(17),{value:a+b});
for(const[a,b]of[[max128,1n],[max129,0n],[max129,255n]])success('mixed',[a,b],[raw(a),raw(b)],[raw(a+b)],[...align(17),...align(1)],align(17),{value:a+b});
for(const[a,b]of[[0n,0n],[max128,0n]])success('narrow128',[a,b],[raw(a),raw(b)],[raw(a+b)],[...align(16),...align(16)],align(16),{value:a+b});
for(const[a,b]of[[max128,1n],[max128,max128]])failure('narrow128',[a,b]);
for(const[left,right]of[[max128,1n],[max129-1n,1n],[max129,1n],[max129,255n]])for(const selected of[false,true]){
 const events=selected?[['left',5,1n],['right',6,1n]]:[];
 if(selected&&left+right>max129){failure('witnessed',[selected],left,right,events);continue}
 const value=selected?left+right:0n;
 success('witnessed',[selected],[selected?[1]:[]],[raw(value)],align(1),align(17),{value},left,right,events);
 const row=rows.at(-1);assert.equal(row.privateState,selected?7:5);assert.deepEqual(row.proofData.privateTranscriptOutputs.map(atoms),selected?[[raw(left)],[raw(right)]]:[]);
}
for(const fail of['left','right'])failure('witnessed',[true],1n,1n,fail==='left'?[['left',5,1n]]:[['left',5,1n],['right',6,1n]],fail);
const pkg=deps+'/@midnight-ntwrk/compact-runtime';const wasm=pkg+'/node_modules/@midnight-ntwrk/onchain-runtime-v3';
const paths=['/Users/ysh/.codex/worktrees/recording-effect-prototype/compact/examples/rust_backend/effect_plan_wide_add.compact',out+'/probe-official-ts.mjs',out+'/ts-source/contract/index.js',out+'/ts-source/contract/index.d.ts','/tmp/rust040-scope-identity/source/obj/compiler/compactc',pkg+'/package.json',pkg+'/dist/compact-types.js',pkg+'/dist/proof-data.js',wasm+'/package.json',wasm+'/midnight_onchain_runtime_wasm_bg.wasm'];
const files=paths.map(path=>{const content=readFileSync(path);return{path,bytes:content.length,sha256:createHash('sha256').update(content).digest('hex')}});
const report={status:'passed',scope:'Same-source official TypeScript runtime and preimage serialization; no proof generation/verification/application.',utc:new Date().toISOString(),versions:{node:process.version,compactRuntime:JSON.parse(readFileSync(pkg+'/package.json')).version,onchainRuntime:JSON.parse(readFileSync(wasm+'/package.json')).version},files,caseCount:rows.length,rows};
writeFileSync(out+'/official-ts-wide-add.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({...report,rows:undefined,files:undefined},null,2));
