// This file is part of Compact.
// Copyright (C) 2026 contributors to Minokawa Compact
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Current-profile successor of historical producer cb21e82cfbaf9b0ffa441f41cc024fe5c0e8756c7e4b846604c18e530f80737a.
// The original producer/capture identities and public retained oracle remain unchanged.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync, writeFileSync, realpathSync, lstatSync, readdirSync, existsSync, unlinkSync} from 'node:fs';
import {basename, dirname, isAbsolute, join} from 'node:path';
import {createRequire} from 'node:module';
import {fileURLToPath, pathToFileURL} from 'node:url';

assert.equal(process.env.NODE_OPTIONS, undefined, 'custom Node loader/options are unsupported');
assert.equal(process.env.NODE_PATH, undefined, 'custom Node module search paths are unsupported');
assert.deepEqual(process.execArgv, [], 'invoke this producer without Node execution flags');

const names = ['source', 'contract', 'runtime', 'compiler', 'frontend', 'typescript', 'ir', 'rust', 'inputs', 'output', 'provenance'];
const options = {};
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i].replace(/^--/, '');
  assert.ok(process.argv[i] === `--${key}` && names.includes(key) && !Object.hasOwn(options, key), `unknown/duplicate option: ${process.argv[i]}`);
  const value = process.argv[i + 1];
  assert.ok(value && isAbsolute(value), `--${key} requires an absolute path`);
  options[key] = value;
}
assert.deepEqual(Object.keys(options).sort(), [...names].sort(), `required options: ${names.map(n => `--${n}`).join(' ')}`);
const sha256 = data => createHash('sha256').update(data).digest('hex');
const regularFile = path => {
  assert.ok(lstatSync(path).isFile() && realpathSync(path) === path, `canonical regular file required: ${path}`);
  return path;
};
for (const key of ['source', 'contract', 'compiler', 'frontend', 'ir', 'rust', 'inputs']) regularFile(options[key]);
assert.equal(basename(options.contract), 'index.js');
const declaration = regularFile(join(dirname(options.contract), 'index.d.ts'));
let contractScope = dirname(options.contract);
while (!existsSync(join(contractScope, 'package.json')) && dirname(contractScope) !== contractScope) contractScope = dirname(contractScope);
const contractPackage = existsSync(join(contractScope, 'package.json')) ? regularFile(join(contractScope, 'package.json')) : null;
for (const key of ['runtime', 'typescript']) {
  assert.ok(lstatSync(options[key]).isDirectory() && realpathSync(options[key]) === options[key], `canonical package directory required: ${key}`);
}
for (const key of ['output', 'provenance']) {
  assert.equal(realpathSync(dirname(options[key])), dirname(options[key]), 'output parent must already exist and be canonical');
  assert.ok(!existsSync(options[key]), `output already exists: ${key}`);
  try { lstatSync(options[key]); assert.fail(`output already exists: ${key}`); } catch (error) { assert.equal(error.code, 'ENOENT'); }
}
assert.notEqual(options.output, options.provenance);
const runtimeEntry = regularFile(join(options.runtime, 'dist/index.js'));
const contractRequire = createRequire(pathToFileURL(options.contract));
const runtimeRequire = createRequire(pathToFileURL(join(options.runtime, 'package.json')));
const resolvedRuntime = realpathSync(contractRequire.resolve('@midnight-ntwrk/compact-runtime'));
assert.equal(resolvedRuntime, runtimeEntry, 'generated contract resolves a different compact-runtime module');
const onchainEntry = realpathSync(runtimeRequire.resolve('@midnight-ntwrk/onchain-runtime-v3'));
let onchainRoot = dirname(onchainEntry);
while (!existsSync(join(onchainRoot, 'package.json'))) {
  const parent = dirname(onchainRoot); assert.notEqual(parent, onchainRoot); onchainRoot = parent;
}
// These are the runtime's two additional executable production dependencies.
// Resolve from the selected runtime, not from the producer's working directory.
const supportingEntries = ['@noble/hashes', 'object-inspect'].map(name => {
  const entry = realpathSync(runtimeRequire.resolve(name));
  let root = dirname(entry);
  while (!existsSync(join(root, 'package.json'))) {
    const parent = dirname(root); assert.notEqual(parent, root); root = parent;
  }
  assert.equal(JSON.parse(readFileSync(join(root, 'package.json'))).name, name);
  return {name, entry, root};
});
const pkg = JSON.parse(readFileSync(join(options.runtime, 'package.json')));
const wasmPkg = JSON.parse(readFileSync(join(onchainRoot, 'package.json')));
const typescriptPkg = JSON.parse(readFileSync(join(options.typescript, 'package.json')));
assert.equal(pkg.name, '@midnight-ntwrk/compact-runtime'); assert.equal(pkg.version, '0.16.101');
// This profile has no import/require condition split. Refuse a changed export
// map rather than treating createRequire resolution as proof of ESM identity.
assert.deepEqual(pkg.exports['.'], {default: './dist/index.js', types: './dist/index.d.ts'});
assert.equal(readFileSync(options.contract, 'utf8').split("import * as __compactRuntime from '@midnight-ntwrk/compact-runtime';").length, 2, 'expected one genuine generated runtime import');
assert.equal(wasmPkg.name, '@midnight-ntwrk/onchain-runtime-v3'); assert.equal(wasmPkg.version, '3.1.2');
assert.equal(typescriptPkg.name, 'typescript'); assert.equal(typescriptPkg.version, '6.0.3');

// The caller freezes actual bytes independently before this process starts.
// Include complete selected package payloads; node_modules is authenticated via
// the separately resolved onchain package. No timestamp or result caching.
const expectedBytes = readFileSync(options.inputs);
const expected = JSON.parse(expectedBytes);
assert.equal(expected.schema, 1); assert.ok(Array.isArray(expected.files));
const expectedByPath = new Map();
for (const row of expected.files) {
  assert.ok(isAbsolute(row.path) && /^[a-f0-9]{64}$/.test(row.sha256) && !expectedByPath.has(row.path), 'invalid/duplicate input pin');
  expectedByPath.set(row.path, row.sha256);
}
const producer = regularFile(fileURLToPath(import.meta.url));
const requiredFiles = [options.source, producer, options.contract, declaration, options.compiler, options.frontend, options.ir, options.rust, realpathSync(process.execPath), ...(contractPackage ? [contractPackage] : [])];
function packageFiles(root) {
  return readdirSync(root).sort().flatMap(name => {
    if (name === 'node_modules') return [];
    const path = join(root, name), info = lstatSync(path);
    if (info.isDirectory()) return packageFiles(path);
    assert.ok(info.isFile() && realpathSync(path) === path, `non-regular package payload: ${path}`);
    return [path];
  });
}
function snapshot() {
  assert.equal(sha256(readFileSync(options.inputs)), sha256(expectedBytes), 'input pin manifest changed');
  const required = new Set([...requiredFiles, ...packageFiles(options.runtime), ...packageFiles(onchainRoot), ...packageFiles(options.typescript), ...supportingEntries.flatMap(item => packageFiles(item.root))]);
  for (const path of required) assert.ok(expectedByPath.has(path), `missing input pin: ${path}`);
  assert.equal(realpathSync(contractRequire.resolve('@midnight-ntwrk/compact-runtime')), runtimeEntry, 'generated runtime resolution changed');
  assert.equal(realpathSync(runtimeRequire.resolve('@midnight-ntwrk/onchain-runtime-v3')), onchainEntry, 'onchain runtime resolution changed');
  for (const item of supportingEntries) assert.equal(realpathSync(runtimeRequire.resolve(item.name)), item.entry, 'supporting runtime resolution changed');
  return [...expectedByPath].map(([path, expectedHash]) => {
    const content = readFileSync(regularFile(path));
    assert.equal(sha256(content), expectedHash, `input changed: ${path}`);
    return {path, bytes: content.length, sha256: expectedHash};
  });
}
const before = snapshot();
const r = await import(pathToFileURL(runtimeEntry).href);
const generatedRuntime = await import(pathToFileURL(resolvedRuntime).href);
assert.equal(r.QueryContext, generatedRuntime.QueryContext, 'QueryContext module identity differs');
const {Contract, ledger} = await import(pathToFileURL(options.contract).href);
// Preserve official wrapper gas while separately observing every executed query.
// Generated witness ledger views have their own QueryContext/cost accumulator.
const queryCosts=[];
const originalQuery=r.QueryContext.prototype.query;
try {
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
assert.equal(rows.length, 21);
assert.equal(rows.filter(row => row.status === 'success').length, 15);
assert.equal(rows.filter(row => row.status === 'refused').length, 6);
const after = snapshot();
assert.deepEqual(after, before, 'inputs changed during capture');
const files = after.map(file => ({...file, path: `artifact:sha256:${file.sha256}/${basename(file.path)}`}));
const report = {
  status: 'passed',
  scope: 'New current-profile same-source official TypeScript runtime and preimage serialization; no proof generation/verification/application. Historical capture is unchanged.',
  utc: new Date().toISOString(),
  versions: {node: process.version, compactRuntime: pkg.version, onchainRuntime: wasmPkg.version, typescript: typescriptPkg.version},
  inputsManifestSha256: sha256(expectedBytes),
  predecessorProducerSha256: 'cb21e82cfbaf9b0ffa441f41cc024fe5c0e8756c7e4b846604c18e530f80737a',
  files, caseCount: rows.length, rows
};
// Exclusive publication preserves existing files; cleanup only files created here.
let wroteOutput = false, wroteProvenance = false;
try {
  writeFileSync(options.output, JSON.stringify(report, null, 2) + '\n', {flag: 'wx', mode: 0o600}); wroteOutput = true;
  writeFileSync(options.provenance, JSON.stringify({argv: process.argv, options, inputsManifestSha256: sha256(expectedBytes), runtimeEntry, onchainEntry, files: after}, null, 2) + '\n', {flag: 'wx', mode: 0o600}); wroteProvenance = true;
} catch (error) {
  if (wroteOutput) unlinkSync(options.output);
  if (wroteProvenance) unlinkSync(options.provenance);
  throw error;
}
console.log(JSON.stringify({...report, rows: undefined, files: undefined}, null, 2));
} finally {
  r.QueryContext.prototype.query = originalQuery;
}
