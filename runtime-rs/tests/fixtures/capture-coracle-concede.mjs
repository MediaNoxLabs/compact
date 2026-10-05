// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Complete original source; seeded prior state does not prove the game funding lifecycle.
import { createHash } from 'node:crypto';
import { readFileSync, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

// Compile test-center/test-contracts/coracle.compact with --skip-zk. Link its
// generated package to an isolated corrected Compact TS runtime, then pass
// contract/index.js, runtime root, pinned ledger-v8 module, and source path.
const [contractPath, runtimeRoot, ledgerPath, sourcePath] = process.argv.slice(2);
if (!contractPath || !runtimeRoot || !ledgerPath || !sourcePath) {
  throw new Error('expected contract/index.js, corrected runtime root, ledger-v8 module, source');
}
const resolvedRuntime = createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime');
if (!realpathSync(resolvedRuntime).startsWith(realpathSync(runtimeRoot) + '/')) {
  throw new Error('generated contract is not linked to the selected corrected runtime');
}
const r = await import(pathToFileURL(resolvedRuntime).href);
const ledger = await import(pathToFileURL(ledgerPath).href);
const { Contract } = await import(pathToFileURL(contractPath).href);
const hash = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const normalize = (value) => JSON.parse(JSON.stringify(value,
  (_, inner) => inner instanceof Uint8Array ? Array.from(inner) : typeof inner === 'bigint' ? inner.toString() : inner));
const gas = (value) => Object.fromEntries(Object.entries(value).map(([key, cost]) => [key, String(cost)]));
const bytes = new r.CompactTypeBytes(32);
const field = r.CompactTypeField;
const phase = new r.CompactTypeEnum(6, 1);
const vector = new r.CompactTypeVector(2, bytes);
const u128 = new r.CompactTypeUnsignedInteger((1n << 128n) - 1n, 16);
const u64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
const boardType = { alignment: () => field.alignment(), toValue: board => field.toValue(board.position) };
const commitmentType = { alignment: () => field.alignment(), toValue: c => field.toValue(c.value) };
const maybeType = {
  alignment: () => r.CompactTypeBoolean.alignment().concat(field.alignment()),
  toValue: m => r.CompactTypeBoolean.toValue(m.is_some).concat(field.toValue(m.value)),
};
const qualifiedType = {
  alignment: () => bytes.alignment().concat(bytes.alignment(), u128.alignment(), u64.alignment()),
  toValue: c => bytes.toValue(c.nonce).concat(bytes.toValue(c.color), u128.toValue(c.value), u64.toValue(c.mt_index)),
};
const fill = byte => new Uint8Array(32).fill(byte);
const key = { bytes: fill(0) };
const address = r.dummyContractAddress();
const secret = color => fill(color === 'red' ? 7 : 8);
const pad = value => { const result = fill(0); result.set(new TextEncoder().encode(value)); return result; };
const publicKey = (color, keyColor = color) => r.persistentHash(vector,
  [secret(keyColor), pad(`coracle:${color}`)]);
const cell = (type, value) => r.StateValue.newCell({ value: type.toValue(value), alignment: type.alignment() });
const array = values => values.reduce((state, value) => state.arrayPush(value), r.StateValue.newArray());
const stateHex = state => Buffer.from(state.serialize()).toString('hex');
const coin = (nonce, value, index) => ({nonce:fill(nonce),color:fill(2),value:BigInt(value),mt_index:BigInt(index)});
let active, calls = [], queries = [], partial;
const contract = new Contract({
  local_secret_key: ({ privateState }) => {
    calls.push('secret');
    if (active.secretFailure && calls.filter(name => name === 'secret').length === 2) {
      throw new Error('selected secret witness failed');
    }
    const changed = active.impostor || (active.changedSecret && calls.length > 1);
    return [{calls:privateState.calls + 1},changed?fill(9):secret(active.color)];
  },
  local_board: ({ privateState }) => {
    calls.push('board');
    if (active.boardFailure) throw new Error('selected board witness failed');
    return [{calls:privateState.calls + 1},{
      nonce:active.wrongNonce?12n:11n,
      contents:{position:active.invalidBoard?0n:active.board},
    }];
  },
  local_set_board: () => { throw new Error('unselected local_set_board called'); },
  fresh_nonce: () => { throw new Error('unselected fresh_nonce called'); },
});
const originalConcede = contract._concede_0;
contract._concede_0 = function (context, proof) {
  partial = proof;
  return originalConcede.call(this, context, proof);
};
const originalQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const output = originalQuery.call(this, ...args);
  queries.push({raw:args[0],gas:gas(output.gasCost),
    tags:args[0].map(op=>typeof op==='string'?op:Object.keys(op)[0])});
  return output;
};

function run(name, color = 'red', options = {}) {
  active = {color,board:1n,...options};
  calls = []; queries = []; partial = undefined;
  const initial = contract.initialState({
    initialPrivateState:{calls:0},initialZswapLocalState:r.emptyZswapLocalState(key),
  });
  const fields = initial.currentContractState.data.state.asArray();
  fields[0] = cell(bytes,publicKey('red'));
  fields[1] = cell(bytes,publicKey('blue',options.bothPlayers?'red':'blue'));
  const commitment = {value:r.transientCommit(boardType,{position:active.board},11n)};
  fields[2] = cell(commitmentType,commitment);
  fields[3] = cell(commitmentType,commitment);
  fields[4] = cell(maybeType,{is_some:!options.empty,
    value:options.empty?0n:options.alive?5n:active.board});
  fields[5] = cell(phase,options.phase ?? (options.wrongTurn ? color==='red'?4:3 : color==='red'?3:4));
  fields[6] = cell(qualifiedType,coin(3,42,0));
  fields[7] = cell(qualifiedType,coin(4,17,1));
  fields[8] = cell(qualifiedType,coin(5,19,2));
  initial.currentContractState.data = new r.ChargedState(array(fields));
  const before = stateHex(initial.currentContractState);
  const beforeState = initial.currentContractState.data;
  const executionKey = {bytes:fill(options.keyByte ?? 11)};
  const context = r.createCircuitContext(address,executionKey,beforeState,{calls:0});
  context.currentZswapLocalState.currentIndex = 3n;
  if (options.missingKey) context.currentZswapLocalState.coinPublicKey = undefined;
  if (options.zeroGas) context.gasLimit = {readTime:0n,computeTime:0n,bytesWritten:0n,bytesDeleted:0n};
  calls = []; queries = [];
  try {
    const out = contract.circuits.concede(context);
    const ownQueries = [...queries];
    const indices = Array.from(out.context.currentQueryContext.comIndices);
    const prior = ledger.ContractState.deserialize(Buffer.from(before,'hex'));
    let query = new ledger.QueryContext(prior.data,address);
    for (const [commitment,index] of indices) query = query.insertCommitment(commitment,index);
    const pair = ledger.partitionTranscripts([
      new ledger.PreTranscript(query,out.proofData.publicTranscript,undefined),
    ],ledger.LedgerParameters.initialParameters())[0];
    const partition = pair.map(transcript => transcript ? {
      programLength:transcript.program.length,effects:transcript.effects,gas:transcript.gas,
    } : null);
    if (!partition[0] || partition[1] || partition[0].programLength !== 42
      || indices.length !== 1 || out.context.currentZswapLocalState.inputs.length !== 1
      || out.context.currentZswapLocalState.outputs.length !== 1) {
      throw new Error('original concede guaranteed single-offer partition changed');
    }
    const replay = r.createCircuitContext(address,executionKey,beforeState,null);
    for (const [commitment,index] of indices) {
      replay.currentQueryContext = replay.currentQueryContext.insertCommitment(commitment,index);
    }
    const replayed = originalQuery.call(replay.currentQueryContext,
      ownQueries.flatMap(item=>item.raw),replay.costModel);
    initial.currentContractState.data = new r.ChargedState(out.context.currentQueryContext.state.state);
    return {name,color,options,before,after:stateHex(initial.currentContractState),
      result:out.result,output:out.proofData.output,privateState:out.context.currentPrivateState,
      privateOutputs:out.proofData.privateTranscriptOutputs,
      publicTranscript:out.proofData.publicTranscript,queries:ownQueries,
      witnessCalls:[...calls],effects:out.context.currentQueryContext.effects,
      plan:out.context.currentZswapLocalState,comIndices:indices,partition,
      reportedGas:gas(out.gasCost),replayGas:gas(replayed.gasCost),
      queryCostSum:Object.fromEntries(Object.keys(out.gasCost).map(key=>[key,
        String(ownQueries.reduce((sum,item)=>sum+BigInt(item.gas[key]),0n))])),
      replayEffects:replayed.context.effects};
  } catch(error) {
    return {name,color,options,before,error:String(error.message),
      witnessCalls:[...calls],privateOutputPrefix:partial?.privateTranscriptOutputs??[],
      queries:[...queries]};
  }
}

const rows = [
  run('red'),run('blue','blue'),
  run('distinctRecipient','red',{keyByte:23}),
  run('bothPlayersRed','red',{bothPlayers:true}),
];
for (const color of ['red','blue']) {
  for (const option of ['impostor','changedSecret','wrongNonce','invalidBoard',
    'wrongTurn','alive','empty','missingKey','secretFailure','boardFailure','zeroGas']) {
    rows.push(run(`${color}-${option}`,color,{[option]:true}));
  }
}
rows.push(run('bothPlayersWrongTurn','red',{bothPlayers:true,wrongTurn:true}));
const provenance = {
  sourceSha256:hash(sourcePath),generatedIndexSha256:hash(contractPath),
  runtimeVersion:JSON.parse(readFileSync(`${runtimeRoot}/package.json`,'utf8')).version,
  runtimeBuiltinsSha256:hash(`${runtimeRoot}/dist/built-ins.js`),
  runtimeTypesSha256:hash(`${runtimeRoot}/dist/compact-types.js`),
  ledgerModuleSha256:hash(ledgerPath),
  scope:'unchanged original source; explicitly seeded prior game and coin state',
};
process.stdout.write(JSON.stringify(normalize({provenance,rows}),null,2)+'\n');
