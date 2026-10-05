// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { readFileSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
const ledger = await import(pathToFileURL(process.argv[3] ?? new URL('../../../tools/compact-rust-backend/wallet-live/node_modules/@midnight-ntwrk/ledger-v8/midnight_ledger_wasm_fs.js', import.meta.url).pathname));
const path = process.argv[2];
if (!path) throw new Error('usage: node capture-micro-dao-set-topic.mjs <original-contract.js> [ledger-v8.js]');
const resolved = realpathSync(createRequire(path).resolve('@midnight-ntwrk/compact-runtime'));
if (!resolved.startsWith('/private/tmp/compact-adr200-runtime/') && !resolved.startsWith('/tmp/compact-adr200-runtime/')) throw new Error('wrong runtime');
const r = await import(pathToFileURL(resolved));
const { Contract, ledger: readLedger } = await import(pathToFileURL(path));
const max = (1n << 128n) - 1n;
const bytes = new r.CompactTypeBytes(32);
const u64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
const u128 = new r.CompactTypeUnsignedInteger(max, 16);
const phase = new r.CompactTypeEnum(3, 1);
const fill = n => new Uint8Array(32).fill(n);
const key = { bytes: fill(7) };
const cell = (types, values) => r.StateValue.newCell({ value: types.flatMap((type, i) => type.toValue(values[i])), alignment: types.flatMap(type => type.alignment()) });
const scalar = (type, value) => cell([type], [value]);
const array = values => values.reduce((a, value) => a.arrayPush(value), r.StateValue.newArray());
const hex = state => Buffer.from(state.serialize()).toString('hex');
const gas = value => Object.fromEntries(Object.entries(value).map(([name, cost]) => [name, String(cost)]));
let witnessCalls = [], queries = [], events = [], mode = 'normal';
const unused = () => { throw new Error('unexpected witness'); };
const contract = new Contract({
  local_secret_key: ({privateState}) => {
    witnessCalls.push('secret');
    if (mode === 'failure') throw new Error('secret witness refused');
    return [{calls: privateState.calls + 1}, mode === 'malformed' ? new Uint8Array(31) : fill(mode === 'wrong' ? 8 : 4)];
  },
  local_state: unused, local_vote_cast: unused, local_advance_state: unused,
  local_record_vote: unused, local_path_of_cm: unused,
});
for (const [method, kind] of [['_createZswapInput_0', 'input'], ['_createZswapOutput_0', 'output']]) {
  const original = contract[method];
  contract[method] = function (...args) {
    const result = original.apply(this, args);
    events.push({kind, coin: args[2], ...(kind === 'output' ? {recipient: args[3]} : {})});
    return result;
  };
}
const originalQuery = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ops: args[0], gas: gas(result.gasCost)});
  return result;
};
function run(name, options = {}) {
  const initial = contract.initialState({initialPrivateState:{calls:0},initialZswapLocalState:r.emptyZswapLocalState(key)}, fill(4), {seed_dust: options.cost ?? 10n, buy_in_dust:3n});
  const state = initial.currentContractState;
  const fields = state.data.state.asArray();
  fields[1] = scalar(phase, options.phase ?? 0);
  fields[11] = scalar(r.CompactTypeBoolean, options.occupied ?? false);
  if (options.occupied) fields[10] = cell([bytes,bytes,u128,u64],[fill(1),fill(options.potColor ?? 0),options.potValue ?? 17n,0n]);
  state.data = new r.ChargedState(array(fields));
  const before = hex(state), beforeState = state.data;
  const context = r.createCircuitContext(r.dummyContractAddress(), key, beforeState, {calls:0});
  context.currentZswapLocalState.currentIndex = 2n;
  const coin = {nonce:fill(3),color:fill(options.color ?? 0),value:options.value ?? 10n};
  witnessCalls = []; queries = []; events = []; mode = options.mode ?? 'normal';
  try {
    const result = contract.circuits.set_topic(context, options.topic ?? 'Proposal 🗳️', {bytes:fill(5)}, coin);
    const captured = [...queries], calls = [...witnessCalls], intentEvents = [...events];
    state.data = new r.ChargedState(result.context.currentQueryContext.state.state);
    const replay = r.createCircuitContext(r.dummyContractAddress(), key, r.ContractState.deserialize(Buffer.from(before, "hex")).data, null);
    for (const [commitment, index] of result.context.currentQueryContext.comIndices) replay.currentQueryContext = replay.currentQueryContext.insertCommitment(commitment, index);
    let partition;
    try {
      const originalState = ledger.ContractState.deserialize(Buffer.from(before, 'hex'));
      let query = new ledger.QueryContext(originalState.data, r.dummyContractAddress());
      for (const [commitment, index] of result.context.currentQueryContext.comIndices) query = query.insertCommitment(commitment, index);
      const pair = ledger.partitionTranscripts([new ledger.PreTranscript(query, result.proofData.publicTranscript, undefined)], ledger.LedgerParameters.initialParameters())[0];
      partition = pair.map(transcript => transcript ? {program: transcript.program, effects: transcript.effects, gas: transcript.gas} : null);
    } catch(error) { partition = {error: String(error.message)}; }
    let replayProbe;
    try { replayProbe = {gas: gas(originalQuery.call(replay.currentQueryContext, captured.flatMap(q => q.ops), replay.costModel).gasCost)}; }
    catch(error) { replayProbe = {error: String(error.message)}; }
    const view = readLedger(state.data);
    const ledgerAfter = {pot: view.pot, pot_has_coin: view.pot_has_coin, topic: view.topic, beneficiary: view.beneficiary, state: view.state};
    return {name,options,before,ledgerAfter,after:hex(state),result:result.result,output:result.proofData.output,privateState:result.context.currentPrivateState,privateOutputs:result.proofData.privateTranscriptOutputs,effects:result.context.currentQueryContext.effects,queries:captured,publicTranscript:result.proofData.publicTranscript,witnessCalls:calls,events:intentEvents,plan:result.context.currentZswapLocalState,wrapperLastQueryGas:gas(result.gasCost),queryCostSum:Object.fromEntries(Object.keys(result.gasCost).map(k=>[k,String(captured.reduce((sum,q)=>sum+BigInt(q.gas[k]),0n))])),replayProbe,partition,provisionalComIndices:Array.from(result.context.currentQueryContext.comIndices)};
  } catch(error) {
    return {name,options,before,error:String(error.message),queries:[...queries],witnessCalls:[...witnessCalls],events:[...events]};
  }
}
const rows = [
  run('empty'), run('occupied',{occupied:true}),
  run('emptyLongTopic',{topic:'x'.repeat(4096)}), run('occupiedEmptyTopic',{occupied:true,topic:''}),
  run('authority',{mode:'wrong'}), run('phase',{phase:1}),
  run('badCost',{value:9n}), run('badColor',{color:2}),
  run('badCostBeforeSecret',{value:9n,mode:'failure'}),
  run('badAuthorityBeforePhase',{phase:1,mode:'wrong'}),
  run('mergeColor',{occupied:true,potColor:2}),
  run('mergeOverflow',{occupied:true,potValue:max}),
  run('mergeColorBeforeOverflow',{occupied:true,potColor:2,potValue:max}),
  run('zeroSeed',{cost:0n,value:0n}),
  run('witnessFailure',{mode:'failure'}), run('witnessMalformed',{mode:'malformed'}),
];
const provenance={source:'test-center/test-contracts/micro-dao.compact',runtime:resolved,generatedSha256:createHash('sha256').update(readFileSync(path)).digest('hex'),descriptor:r.ShieldedCoinInfoDescriptor.alignment(),scope:'independent original-source TypeScript execution/partition evidence; explicit prior states, no funded lifecycle or proof claim'};
process.stdout.write(JSON.stringify({provenance,rows},(_,value)=>value instanceof Uint8Array?Array.from(value):typeof value==='bigint'?String(value):value instanceof Map?Object.fromEntries(value):value,2)+'\n');
