// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Independent TS oracle for ADR0355's opt-in recording-plan experiment.
// Usage: node capture-effect-plan-prototype.mjs <compiled index.js> <runtime index.js>
import { pathToFileURL } from 'node:url';
const [contractPath, runtimePath] = process.argv.slice(2);
if (!contractPath || !runtimePath) throw new Error('expected contract and runtime paths');
const runtime = await import(pathToFileURL(runtimePath).href);
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);
const gas = (value) => Object.fromEntries(Object.entries(value).map(([k,v]) => [k,v.toString()]));
const coinPublicKey = { bytes: new Uint8Array(32) };
const originalQuery = runtime.QueryContext.prototype.query;
let capture = false;
let inWitness = false;
let queries = [];
let programs = [];
let observations = [];
let markValue = 77n;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  if (capture) {
    queries.push({ gasCost: gas(result.gasCost), witness: inWitness });
    if (!inWitness) programs.push(args[0]);
  }
  return result;
};
const contract = new Contract({ mark: ({ ledger, privateState }) => {
  inWitness = true;
  try { observations.push([ledger.state.toString(), ledger.round.toString(), privateState]); }
  finally { inWitness = false; }
  return [privateState + 1, markValue];
}});
function run(name, selector, next, setup = false, mark = 77n) {
  markValue = mark;
  const initial = contract.initialState({ initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey) });
  const context = () => runtime.createCircuitContext(runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState);
  if (setup) {
    const preceding = contract.circuits.guardedChoice(context(), 0n, 9n);
    initial.currentContractState.data = new runtime.ChargedState(preceding.context.currentQueryContext.state.state);
    initial.currentPrivateState = preceding.context.currentPrivateState;
  }
  queries = []; programs = []; observations = []; capture = true;
  let output;
  try { output = contract.circuits[name](context(), BigInt(selector), BigInt(next)); }
  catch (error) {
    return { error: error.message, witnessObservations: observations, queries };
  } finally { capture = false; }
  const replay = originalQuery.call(context().currentQueryContext, programs.flat(), output.context.costModel);
  initial.currentContractState.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
  const view = ledger(initial.currentContractState.data);
  return {
    result: output.result.toString(), state: view.state.toString(), round: view.round.toString(),
    privateState: output.context.currentPrivateState, witnessObservations: observations,
    afterStateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    privateTranscriptOutputs: output.proofData.privateTranscriptOutputs,
    publicTranscript: output.proofData.publicTranscript, output: output.proofData.output,
    effects: output.context.currentQueryContext.effects, queries,
    reportedGas: gas(output.gasCost), replayGas: gas(replay.gasCost),
  };
}
const cases = {};
for (const [id, name, selector, next, setup] of [
  ['guarded-zero','guardedChoice',0,9,false], ['guarded-one','guardedChoice',1,9,false],
  ['guarded-two','guardedChoice',2,9,false], ['ordered-zero','orderedObservation',0,9,false],
  ['ordered-one','orderedObservation',1,9,false], ['ordered-two','orderedObservation',2,0,false],
  ['guarded-nonzero','guardedChoice',1,9,true], ['ordered-nonzero','orderedObservation',1,9,true],
]) cases[id] = run(name,selector,next,setup);
const failures = {};
for (const [id, name, selector, next, mark] of [
  ['early-guard','guardedChoice',1,8,77], ['guarded-mark','guardedChoice',1,9,76],
  ['ordered-early-mark','orderedObservation',0,9,76],
  ['ordered-late-mark','orderedObservation',1,9,76], ['late-guard','orderedObservation',2,9,77],
]) failures[id] = run(name,selector,next,false,BigInt(mark));
process.stdout.write(JSON.stringify({cases,failures}, (_,v) =>
  v instanceof Uint8Array ? Array.from(v) : typeof v === 'bigint' ? v.toString() : v, 2) + '\n');
