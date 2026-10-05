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

// Compile the original election_oracle.compact with --skip-zk and pass contract/index.js.
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { normalizeQueryProgram } from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';
const [contractPath] = process.argv.slice(2);
const runtime = await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractPath));
const secret = new Uint8Array(32).fill(7);
const prefix = new Uint8Array(32); prefix.set(new TextEncoder().encode('lares:election:pk:'));
const authority = runtime.persistentHash(new runtime.CompactTypeVector(2, new runtime.CompactTypeBytes(32)), [prefix, secret]);
const coinPublicKey = { bytes: new Uint8Array(32) };
let witnessState;
let witnessCalls = [];
const bytesType = new runtime.CompactTypeBytes(32);
const pathType = new runtime.CompactTypeMerkleTreePath(10, bytesType);
const missing = () => ({ is_some: false, value: { leaf: new Uint8Array(32), path:
  Array.from({ length: 10 }, () => ({ sibling: { field: 0n }, goes_left: false })) } });
let mode = 'normal';
const bumped = (state, patch = {}) => ({...state, ...patch, calls: state.calls + 1});
const contract = new Contract({
  private$secret_key: ({privateState}) => { witnessCalls.push('secret'); return [bumped(privateState),secret]; },
  private$state: ({privateState}) => { witnessCalls.push('state'); return [bumped(privateState),privateState.phase]; },
  private$state$advance: ({privateState}) => { witnessCalls.push('advance'); return [bumped(privateState,{phase:privateState.phase+1}),[]]; },
  private$vote$record: ({privateState}, ballot) => { witnessCalls.push('record'); return [bumped(privateState,{ballot}),[]]; },
  private$vote: ({privateState}) => { witnessCalls.push('vote'); return [bumped(privateState),privateState.ballot]; },
  context$eligible_voters$path_of: ({privateState}, pk) => {
    witnessCalls.push('path');
    const tree = witnessState.asArray()[6].asArray()[0].asBoundedMerkleTree();
    const leaf = mode === 'wrong_leaf' ? new Uint8Array(32).fill(8) : pk;
    const path = tree.findPathForLeaf({ value: bytesType.toValue(leaf), alignment: bytesType.alignment() });
    const result = path === undefined ? missing() : {is_some:true,value:pathType.fromValue(path.value)};
    if (mode === 'wrong_root') result.value.path[0].sibling.field += 1n;
    if (mode === 'malformed') result.value.path.pop();
    return [bumped(privateState),result];
  },
  context$committed_votes$path_of: ({privateState}, cm) => {
    witnessCalls.push('path');
    const tree = witnessState.asArray()[5].asArray()[0].asBoundedMerkleTree();
    const no = new Uint8Array(32); no.set(new TextEncoder().encode('no'));
    const leaf = mode === 'wrong_leaf' ? runtime.persistentHash(new runtime.CompactTypeVector(2,bytesType),[no,secret]) : cm;
    const path = tree.findPathForLeaf({value:bytesType.toValue(leaf),alignment:bytesType.alignment()});
    const result = path === undefined ? missing() : {is_some:true,value:pathType.fromValue(path.value)};
    if (mode === 'wrong_root') result.value.path[0].sibling.field += 1n;
    if (mode === 'malformed') result.value.path.pop();
    return [bumped(privateState),result];
  },
});
const initial = (key = authority) => contract.initialState({ initialPrivateState: {phase:0,ballot:0,calls:0},
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey) }, key);
const context = (state) => runtime.createCircuitContext(runtime.dummyContractAddress(), coinPublicKey,
  state.currentContractState.data, state.currentPrivateState);
const hex = (state, ctx) => { state.currentContractState.data = new runtime.ChargedState(ctx.currentQueryContext.state.state);
  return Buffer.from(state.currentContractState.serialize()).toString('hex'); };
let queries = [];
let raw = [];
const original = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = original.call(this, ...args);
  queries.push({ program: normalizeQueryProgram(args[0]), gasCost: output.gasCost });
  raw.push(args[0]); return output;
};


function invoke(current, method, ...args) {
  witnessState = current.currentQueryContext.state.state;
  return contract.circuits[method](current, ...args);
}
function seeded({noVoter=false,otherVoter=false,noAdvance=false}={}) {
  mode='normal';
  const state = initial(); let current=context(state);
  current=invoke(current,'set_topic','vote').context;
  if(!noVoter) current=invoke(current,'add_voter',otherVoter?new Uint8Array(32).fill(8):authority).context;
  if(!noAdvance) current=invoke(current,'advance').context;
  current.currentPrivateState={phase:0,ballot:0,calls:0};
  return {state,current};
}
function revealSeeded(ballot=0,{noCommit=false,noAdvance=false,opposite=false}={}) {
 const {state,current}=seeded(); let ready=current;
 if(!noCommit) ready=invoke(ready,'vote$commit',opposite?1:ballot).context;
 if(!noAdvance) ready=invoke(ready,'advance').context;
 ready.currentPrivateState={phase:1,ballot,calls:0};
 return {state,current:ready};
}
const scenarios=[];
for(const ballot of [0,1]) {
 const {state,current}=revealSeeded(ballot);const before=new runtime.ChargedState(current.currentQueryContext.state.state);
 queries=[];raw=[];witnessCalls=[];
 const output=invoke(current,'vote$reveal');
 const captured=queries.slice(), calls=witnessCalls.slice(), replayOps=raw.flat();
 const replayContext=runtime.createCircuitContext(runtime.dummyContractAddress(),coinPublicKey,before,{phase:1,ballot,calls:0});
 const replay=replayContext.currentQueryContext.query(replayOps,replayContext.costModel);
 scenarios.push({ballot,stateHex:hex(state,output.context),privateState:output.context.currentPrivateState,
  witnessCalls:calls,privateTranscriptOutputs:output.proofData.privateTranscriptOutputs,
  gasCost:output.gasCost,queries:captured,replayGas:replay.gasCost});
}
function rejected(make, pathMode='normal') {
 const current=make(); mode=pathMode;queries=[];raw=[];witnessCalls=[];
 try {invoke(current,'vote$reveal');throw new Error('unexpected success');}
 catch(error){return {error:error.message,witnessCalls:witnessCalls.slice(),queries:queries.slice()};}
}
const wrongPhase=rejected(()=>revealSeeded(0,{noAdvance:true}).current);
const wrongPrivate=rejected(()=>{const {current}=revealSeeded();current.currentPrivateState.phase=0;return current;});
const repeated=rejected(()=>{const {current}=revealSeeded();return invoke(current,'vote$reveal').context;});
const duplicate=rejected(()=>{const {current}=revealSeeded();const used=invoke(current,'vote$reveal').context;used.currentPrivateState={phase:1,ballot:0,calls:0};return used;});
const missingPath=rejected(()=>revealSeeded(0,{noCommit:true}).current);
const wrongRoot=rejected(()=>revealSeeded().current,'wrong_root');
const wrongLeaf=rejected(()=>revealSeeded(0,{opposite:true}).current,'wrong_leaf');
const malformed=rejected(()=>revealSeeded().current,'malformed');
process.stdout.write(JSON.stringify({scenarios,wrongPhase,wrongPrivate,repeated,duplicate,missingPath,wrongRoot,wrongLeaf,malformed},
 (_key,value)=>typeof value==='bigint'?value.toString():value instanceof Uint8Array?Array.from(value):value,2)+'\n');
